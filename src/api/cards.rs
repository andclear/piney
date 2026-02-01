use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use base64::{engine::general_purpose, Engine as _};
use chrono::TimeZone;
use futures::StreamExt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Cursor, Write};
use tokio::fs;
use tracing::warn;
use uuid::Uuid;
use zip::write::FileOptions;

use crate::api::dashboard::invalidate_cache;
use crate::entities::character_card;
use crate::utils::hash::compute_json_hash;
use crate::utils::token::calculate_card_tokens;

#[derive(Serialize, Deserialize)]
pub struct ImportResult {
    file_name: String,
    status: String, // "success" | "error"
    reason: Option<String>,
}

// 包装 Handler，将 Result 转换为 Response，避免 E0277 错误
pub async fn import(State(db): State<DatabaseConnection>, multipart: Multipart) -> Response {
    match process_import(db, multipart).await {
        Ok(json) => json.into_response(),
        Err(err) => err.into_response(),
    }
}

async fn process_import(
    db: DatabaseConnection,
    mut multipart: Multipart,
) -> Result<Json<Vec<ImportResult>>, (StatusCode, String)> {
    let storage_dir = crate::utils::paths::get_data_path("cards");
    if !storage_dir.exists() {
        fs::create_dir_all(&storage_dir)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    let mut results: Vec<ImportResult> = Vec::new();

    // 处理 Multipart 请求中的所有文件
    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field.content_type().unwrap_or("").to_string();

        // 读取数据，如果读取失败记录错误并继续
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                results.push(ImportResult {
                    file_name,
                    status: "error".to_string(),
                    reason: Some(format!("读取文件失败: {}", e)),
                });
                continue;
            }
        };

        // 基于 Content-Type 或文件名检测类型
        let is_png = content_type == "image/png" || file_name.to_lowercase().ends_with(".png");
        let is_json =
            content_type == "application/json" || file_name.to_lowercase().ends_with(".json");

        if is_png {
            // 处理 PNG 角色卡
            match process_png_card(&db, &data, storage_dir.clone()).await {
                Ok(_) => {
                    results.push(ImportResult {
                        file_name,
                        status: "success".to_string(),
                        reason: None,
                    });
                }
                Err(err_msg) => {
                    results.push(ImportResult {
                        file_name,
                        status: "error".to_string(),
                        reason: Some(err_msg),
                    });
                }
            }
        } else if is_json {
            // 处理 JSON 角色卡
            match process_json_card(&db, &data, storage_dir.clone()).await {
                Ok(_) => {
                    results.push(ImportResult {
                        file_name,
                        status: "success".to_string(),
                        reason: None,
                    });
                }
                Err(err_msg) => {
                    results.push(ImportResult {
                        file_name,
                        status: "error".to_string(),
                        reason: Some(err_msg),
                    });
                }
            }
        } else {
            results.push(ImportResult {
                file_name,
                status: "error".to_string(),
                reason: Some("不支持的文件格式".to_string()),
            });
        }
    }

    // Invalidate cache if any success
    if results.iter().any(|r| r.status == "success") {
        invalidate_cache();
    }
    Ok(Json(results))
}
async fn process_png_card(
    db: &DatabaseConnection,
    data: &[u8],
    storage_dir: std::path::PathBuf,
) -> Result<(), String> {
    // 1. 手动解析 PNG Chunks 并提取 JSON
    let extracted_json = extract_png_metadata(data)?;

    // 验证这一 JSON 格式是否合法
    let json_val: Value =
        serde_json::from_str(&extracted_json).map_err(|e| format!("元数据 JSON 无效: {}", e))?;

    // 2. 检查重复 (Pre-check)
    // 计算哈希（使用紧凑格式以保证一致性）
    let compact_json =
        serde_json::to_string(&json_val).map_err(|e| format!("序列化 JSON 失败: {}", e))?;
    let data_hash = compute_json_hash(&compact_json);

    let existing = character_card::Entity::find()
        .filter(character_card::Column::DataHash.eq(&data_hash))
        .one(db)
        .await
        .map_err(|e| format!("数据库查询失败: {}", e))?;

    if let Some(existing_card) = existing {
        return Err(format!("角色卡已存在: {}", existing_card.name));
    }

    // 3. 确定无重复后，保存文件
    let uuid = Uuid::new_v4();
    let card_dir = storage_dir.join(uuid.to_string());

    if !card_dir.exists() {
        fs::create_dir_all(&card_dir)
            .await
            .map_err(|e| format!("创建角色卡目录失败: {}", e))?;
    }

    // 保存原始 PNG
    let png_name = "v1_source.png";
    let png_path = card_dir.join(png_name);
    fs::write(&png_path, data)
        .await
        .map_err(|e| format!("保存原始 PNG 失败: {}", e))?;

    // 生成 WebP 缩略图
    let img = image::load_from_memory(data).map_err(|e| format!("图片加载失败: {}", e))?;
    let encoder = webp::Encoder::from_image(&img).map_err(|e| format!("WebP 编码失败: {}", e))?;
    let webp_data = encoder.encode(75.0).to_vec();
    let webp_name = "v1_thumbnail.webp";
    let webp_path = card_dir.join(webp_name);
    fs::write(&webp_path, &webp_data)
        .await
        .map_err(|e| format!("保存 WebP 缩略图失败: {}", e))?;

    // 4. 保存到数据库
    let avatar_path = format!("/cards/{}/v1_thumbnail.webp", uuid);
    save_card_model(db, uuid, json_val, Some(avatar_path), data_hash, "import").await
}

async fn process_json_card(
    db: &DatabaseConnection,
    data: &[u8],
    storage_dir: std::path::PathBuf,
) -> Result<(), String> {
    let json_string = String::from_utf8(data.to_vec()).map_err(|_| "JSON 编码无效".to_string())?;
    // 验证 JSON
    let v: Value = serde_json::from_str(&json_string).map_err(|e| format!("无效的 JSON: {}", e))?;

    // 检查是否为世界书
    if v.get("entries").is_some() && v.get("data").is_none() && v.get("name").is_none() {
        return Err("检测到世界书文件，请在世界书页面进行导入".to_string());
    }
    // 检查是否为有效的角色卡
    if v.get("data").is_none() && v.get("name").is_none() {
        return Err("无效的角色卡格式：缺少必要的 'data' 或 'name' 字段".to_string());
    }

    // 1. 检查重复
    let compact_json = serde_json::to_string(&v).map_err(|e| format!("序列化 JSON 失败: {}", e))?;
    let data_hash = compute_json_hash(&compact_json);

    let existing = character_card::Entity::find()
        .filter(character_card::Column::DataHash.eq(&data_hash))
        .one(db)
        .await
        .map_err(|e| format!("数据库查询失败: {}", e))?;

    if let Some(existing_card) = existing {
        return Err(format!("角色卡已存在: {}", existing_card.name));
    }

    // 2. 保存文件 (Optional, but DB is primary)
    let uuid = Uuid::new_v4();
    let card_dir = storage_dir.join(uuid.to_string());
    if !card_dir.exists() {
        fs::create_dir_all(&card_dir)
            .await
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    // 3. 保存数据库
    save_card_model(
        db,
        uuid,
        v,
        Some("/default.webp".to_string()),
        data_hash,
        "import",
    )
    .await
}

// 提取的 PNG 元数据解析逻辑
fn extract_png_metadata(data: &[u8]) -> Result<String, String> {
    // PNG 签名校验
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if data.len() < 8 || &data[..8] != PNG_SIGNATURE {
        return Err("非法的 PNG 文件签名".to_string());
    }

    let mut offset = 8;
    while offset + 8 <= data.len() {
        let length_bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
        let length = u32::from_be_bytes(length_bytes) as usize;
        let chunk_type = &data[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start + length;

        if data_end > data.len() {
            warn!("PNG Chunk 越界，停止解析");
            break;
        }

        if chunk_type == b"tEXt" {
            let chunk_data = &data[data_start..data_end];
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                let keyword_bytes = &chunk_data[..null_pos];
                let text_bytes = &chunk_data[null_pos + 1..];
                if let Ok(keyword) = String::from_utf8(keyword_bytes.to_vec()) {
                    if keyword == "ccv3" || keyword == "chara" {
                        if let Ok(decoded) = general_purpose::STANDARD.decode(text_bytes) {
                            if let Ok(s) = String::from_utf8(decoded) {
                                return Ok(s); // Found it!
                            }
                        } else if let Ok(s) = String::from_utf8(text_bytes.to_vec()) {
                            return Ok(s); // Found it (raw)!
                        }
                    }
                }
            }
        }
        offset = data_end + 4;
    }
    Err("无效的角色卡图片：未找到元数据 (ccv3/chara)".to_string())
}

async fn save_card_model(
    db: &DatabaseConnection,
    uuid: Uuid,
    json: Value,
    avatar: Option<String>,
    data_hash: String,
    source: &str, // "import" 或 "local"
) -> Result<(), String> {
    // 规范化 V2/V3 结构 (仅用于提取字段)
    let card_data = if let Some(d) = json.get("data") {
        if d.is_object() {
            d
        } else {
            &json
        }
    } else {
        &json
    };

    let name = card_data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未知角色")
        .to_string();
    let description = card_data
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let author = card_data
        .get("creator")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            card_data
                .get("creator_notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    let spec = json
        .get("spec")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let spec_version = json
        .get("spec_version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // 格式化 JSON（只格式化，不添加/删除任何字段）
    let pretty_json_str =
        serde_json::to_string_pretty(&json).map_err(|e| format!("格式化 JSON 失败: {}", e))?;

    // 从 JSON 中提取 tags（可能是数组或逗号分隔的字符串）
    let tags_json = if let Some(tags_value) = card_data.get("tags") {
        if tags_value.is_array() {
            serde_json::to_string_pretty(tags_value).unwrap_or_else(|_| "[]".to_string())
        } else if let Some(tags_str) = tags_value.as_str() {
            let tags: Vec<&str> = tags_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            serde_json::to_string_pretty(&tags).unwrap_or_else(|_| "[]".to_string())
        } else {
            "[]".to_string()
        }
    } else {
        "[]".to_string()
    };

    // 计算 token
    let counts = calculate_card_tokens(&json);

    // 版本号独立管理，不从角色卡 JSON 中提取，默认为 None（前端显示为 1.0）
    let active_model = character_card::ActiveModel {
        id: Set(uuid),
        name: Set(name),
        description: Set(description),
        author: Set(author),
        avatar: Set(avatar),
        spec: Set(spec),
        spec_version: Set(spec_version),
        data: Set(pretty_json_str.clone()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        category_id: Set(None),
        tags: Set(tags_json),
        rating: Set(0.0),
        cover_blur: Set(false),
        version: Set(None),
        deleted_at: Set(None),
        custom_summary: Set(None),
        user_note: Set(None),
        metadata_modified: Set(false),
        data_hash: Set(Some(data_hash)),
        token_count_total: Set(Some(counts.total)),
        token_count_spec: Set(Some(counts.spec)),
        token_count_wb: Set(Some(counts.wb)),
        token_count_other: Set(Some(counts.other)),
        source: Set(source.to_string()),
    };

    active_model
        .insert(db)
        .await
        .map_err(|e| format!("数据库错误: {}", e))?;

    // --- Auto-create Initial Version (V1) ---
    let version_note = if source == "local" {
        "本地新建"
    } else {
        "初始导入"
    };
    let version = crate::entities::character_versions::ActiveModel {
        id: Set(Uuid::new_v4()),
        character_id: Set(uuid),
        version_number: Set("V1".to_string()),
        note: Set(Some(version_note.to_string())),
        data: Set(pretty_json_str), // Use the formatted JSON
        created_at: Set(chrono::Utc::now().naive_utc()),
    };
    if let Err(e) = version.insert(db).await {
        tracing::error!("Failed to create initial version for card {}: {}", uuid, e);
        // We do not fail the import if version creation fails, just log error.
    }

    Ok(())
}

#[derive(Serialize)]
pub struct DebugImportResponse {
    logs: Vec<String>,
    saved_json: Option<String>,
    error: Option<String>,
}

pub async fn debug_import(
    State(db): State<DatabaseConnection>,
    mut multipart: Multipart,
) -> Json<DebugImportResponse> {
    let mut logs = Vec::new();
    let mut saved_json = None;
    let mut error = None;

    logs.push("开始处理调试导入请求...".to_string());

    let storage_dir = crate::utils::paths::get_data_path("cards");
    if !storage_dir.exists() {
        match fs::create_dir_all(&storage_dir).await {
            Ok(_) => logs.push(format!("创建存储目录 {:?} 成功", storage_dir)),
            Err(e) => {
                let msg = format!("创建存储目录失败: {}", e);
                logs.push(msg.clone());
                error = Some(msg);
                return Json(DebugImportResponse {
                    logs,
                    saved_json,
                    error,
                });
            }
        }
    }

    if let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        logs.push(format!("接收到文件: {}", file_name));

        match field.bytes().await {
            Ok(data) => {
                logs.push(format!("文件读取成功，大小: {} bytes", data.len()));

                // 仅支持 PNG 调试
                if file_name.to_lowercase().ends_with(".png") {
                    logs.push("检测到 PNG 文件，开始解析...".to_string());

                    // --- PNG 解析逻辑 (带日志) ---
                    let mut extracted = None;

                    // 1. 签名检查
                    logs.push("检查 PNG 签名...".to_string());
                    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
                    if data.len() >= 8 && &data[..8] == PNG_SIGNATURE {
                        logs.push("签名有效".to_string());

                        // 2. 遍历 Chunk
                        logs.push("开始遍历 Chunks...".to_string());
                        let mut offset = 8;
                        let mut chunk_count = 0;

                        while offset + 8 <= data.len() {
                            chunk_count += 1;
                            let length_bytes: [u8; 4] =
                                data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                            let length = u32::from_be_bytes(length_bytes) as usize;
                            let chunk_type = &data[offset + 4..offset + 8];
                            let type_str = String::from_utf8_lossy(chunk_type);

                            let data_start = offset + 8;
                            let data_end = data_start + length;

                            if data_end > data.len() {
                                logs.push(format!(
                                    "Chunk #{} ({}) 越界! Offset: {}, Len: {}",
                                    chunk_count, type_str, offset, length
                                ));
                                break;
                            }

                            if chunk_type == b"tEXt" {
                                logs.push(format!(
                                    "Chunk #{} [tEXt] found. Len: {}",
                                    chunk_count, length
                                ));
                                let chunk_data = &data[data_start..data_end];
                                if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                                    let keyword_bytes = &chunk_data[..null_pos];
                                    let keyword = String::from_utf8_lossy(keyword_bytes);
                                    logs.push(format!("  Keyword: {}", keyword));

                                    if keyword == "ccv3" || keyword == "chara" {
                                        let text_bytes = &chunk_data[null_pos + 1..];
                                        logs.push(format!(
                                            "  Found target metadata! Length: {}",
                                            text_bytes.len()
                                        ));

                                        // Try decode
                                        if let Ok(decoded) =
                                            general_purpose::STANDARD.decode(text_bytes)
                                        {
                                            logs.push("  Base64 Decode: SUCCESS".to_string());
                                            if let Ok(s) = String::from_utf8(decoded) {
                                                logs.push("  UTF-8 Parse: SUCCESS".to_string());
                                                // Keep the "best" result (e.g. ccv3 over chara, or longest)
                                                // For debug, we just update extracted if it's ccv3 or if we haven't found one yet
                                                if keyword == "ccv3" {
                                                    logs.push("  Identified ccv3 (V3 Spec). Updating candidate.".to_string());
                                                    extracted = Some(s);
                                                } else if extracted.is_none() {
                                                    logs.push("  Identified chara (Legacy). Setting as candidate.".to_string());
                                                    extracted = Some(s);
                                                }
                                            } else {
                                                logs.push("  UTF-8 Parse: FAILED".to_string());
                                            }
                                        } else {
                                            logs.push(
                                                "  Base64 Decode: FAILED. Trying raw text..."
                                                    .to_string(),
                                            );
                                            if let Ok(s) = String::from_utf8(text_bytes.to_vec()) {
                                                logs.push("  Raw Text UTF-8: SUCCESS".to_string());
                                                if keyword == "ccv3" {
                                                    logs.push(
                                                        "  Identified ccv3. Updating candidate."
                                                            .to_string(),
                                                    );
                                                    extracted = Some(s);
                                                } else if extracted.is_none() {
                                                    extracted = Some(s);
                                                }
                                            } else {
                                                logs.push("  Raw Text UTF-8: FAILED".to_string());
                                            }
                                        }
                                    }
                                } else {
                                    logs.push(
                                        "  Malformed tEXt chunk (no null separator)".to_string(),
                                    );
                                }
                            }

                            offset = data_end + 4; // Skip CRC
                        }
                    } else {
                        logs.push("签名无效!".to_string());
                    }

                    if let Some(json_str) = extracted {
                        logs.push("元数据提取成功。".to_string());

                        // 3. 模拟保存并读取
                        logs.push("尝试保存到数据库...".to_string());
                        let uuid = Uuid::new_v4();

                        match serde_json::from_str::<Value>(&json_str) {
                            Ok(json_val) => {
                                let compact_json =
                                    serde_json::to_string(&json_val).unwrap_or_default();
                                let data_hash =
                                    crate::utils::hash::compute_json_hash(&compact_json);

                                match save_card_model(
                                    &db, uuid, json_val, None, data_hash, "import",
                                )
                                .await
                                {
                                    Ok(_) => {
                                        logs.push("保存成功。".to_string());
                                        logs.push(format!("UUID: {}", uuid));

                                        // 4. 从数据库回读验证
                                        logs.push("正在从数据库回读 verify...".to_string());
                                        match character_card::Entity::find_by_id(uuid)
                                            .one(&db)
                                            .await
                                        {
                                            Ok(Some(model)) => {
                                                logs.push("数据库读取成功。".to_string());
                                                let db_data_len = model.data.len();
                                                logs.push(format!(
                                                    "DB中 data 字段长度: {}",
                                                    db_data_len
                                                ));

                                                let raw_val: Result<Value, _> =
                                                    serde_json::from_str(&json_str);
                                                let saved_val: Result<Value, _> =
                                                    serde_json::from_str(&model.data);

                                                match (raw_val, saved_val) {
                                                    (Ok(v1), Ok(v2)) => {
                                                        if v1 == v2 {
                                                            logs.push("验证通过：数据内容一致 (Ignored formatting)。".to_string());
                                                        } else {
                                                            logs.push(
                                                                "验证失败：数据内容不一致！"
                                                                    .to_string(),
                                                            );
                                                        }
                                                    }
                                                    _ => logs.push(
                                                        "验证警告：无法解析 JSON 进行比对。"
                                                            .to_string(),
                                                    ),
                                                }

                                                saved_json = Some(model.data);
                                            }
                                            Ok(None) => logs
                                                .push("错误：保存后无法查找到记录！".to_string()),
                                            Err(e) => logs.push(format!("回读查询失败: {}", e)),
                                        }
                                    }
                                    Err(e) => logs.push(format!("保存数据库失败: {}", e)),
                                }
                            }
                            Err(e) => logs.push(format!("JSON 解析失败: {}", e)),
                        }
                    } else {
                        logs.push("错误：未找到有效元数据。".to_string());
                        error = Some("未找到元数据".to_string());
                    }
                } else {
                    logs.push("非 PNG 文件 (仅支持 PNG 调试)".to_string());
                }
            }
            Err(e) => logs.push(format!("读取 bytes 失败: {}", e)),
        }
    } else {
        logs.push("没有接收到文件 field".to_string());
    }

    Json(DebugImportResponse {
        logs,
        saved_json,
        error,
    })
}

// ============ 新建角色卡 API ============

#[derive(Serialize)]
pub struct CreateCardResponse {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct CreateCardRequest {
    pub name: String,
}

/// POST /api/cards/create - 新建空白角色卡
pub async fn create_card(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<CreateCardRequest>,
) -> Result<Json<CreateCardResponse>, (StatusCode, String)> {
    let storage_dir = crate::utils::paths::get_data_path("cards");
    if !storage_dir.exists() {
        tokio::fs::create_dir_all(&storage_dir)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // 生成 UUID
    let uuid = Uuid::new_v4();

    // 生成创建时间字符串
    let now = chrono::Local::now();
    let create_date = now.format("%Y-%m-%d @%Hh %Mm %Ss %3fms").to_string();

    // 构建空白角色卡 JSON 模板（参考 docs/cankao/新建角色卡.json）
    let card_json = serde_json::json!({
        "name": payload.name,
        "description": "",
        "personality": "",
        "scenario": "",
        "first_mes": "",
        "mes_example": "",
        "creatorcomment": "",
        "avatar": "none",
        "talkativeness": "0.5",
        "fav": false,
        "tags": [],
        "spec": "chara_card_v3",
        "spec_version": "3.0",
        "data": {
            "name": payload.name,
            "description": "",
            "personality": "",
            "scenario": "",
            "first_mes": "",
            "mes_example": "",
            "creator_notes": "",
            "system_prompt": "",
            "post_history_instructions": "",
            "tags": [],
            "creator": "",
            "character_version": "",
            "alternate_greetings": [],
            "extensions": {
                "talkativeness": "0.5",
                "fav": false,
                "world": "",
                "depth_prompt": {
                    "prompt": "",
                    "depth": 4,
                    "role": "system"
                }
            },
            "group_only_greetings": []
        },
        "create_date": create_date
    });

    // 创建角色卡目录（用于后续可能的封面上传等）
    let card_dir = storage_dir.join(uuid.to_string());
    if !card_dir.exists() {
        tokio::fs::create_dir_all(&card_dir).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("创建目录失败: {}", e),
            )
        })?;
    }

    // 计算 hash
    let compact_json = serde_json::to_string(&card_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("序列化 JSON 失败: {}", e),
        )
    })?;
    let data_hash = crate::utils::hash::compute_json_hash(&compact_json);

    // 保存到数据库 (使用默认封面, source = "local")
    save_card_model(
        &db,
        uuid,
        card_json,
        Some("/default.webp".to_string()),
        data_hash,
        "local",
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    invalidate_cache();
    Ok(Json(CreateCardResponse { id: uuid }))
}

// ============ 列表和更新 API ============

#[derive(Deserialize)]
pub struct ListCardsQuery {
    pub category_id: Option<Uuid>,
    pub search: Option<String>,
    pub tags: Option<String>, // 逗号分隔的标签
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedResponse {
    pub items: Vec<CardListItem>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

#[derive(Serialize)]
pub struct CardListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub avatar: Option<String>,
    pub category_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub rating: f64,
    pub cover_blur: bool,
    pub version: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>, // Added
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, FromQueryResult)]
struct CardListRow {
    pub id: Uuid,
    pub name: String,
    // pub description: Option<String>, // Removed
    pub author: Option<String>,
    pub avatar: Option<String>,
    pub category_id: Option<Uuid>,
    pub tags: String,
    pub rating: f64,
    pub cover_blur: bool,
    pub version: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime, // Added
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

/// GET /api/cards - 获取角色卡列表
pub async fn list(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ListCardsQuery>,
) -> Result<Json<PaginatedResponse>, (StatusCode, String)> {
    let mut select = character_card::Entity::find()
        .select_only()
        .columns([
            character_card::Column::Id,
            character_card::Column::Name,
            // character_card::Column::Description, // Removed
            character_card::Column::Author,
            character_card::Column::Avatar,
            character_card::Column::CategoryId,
            character_card::Column::Tags,
            character_card::Column::Rating,
            character_card::Column::CoverBlur,
            character_card::Column::Version,
            character_card::Column::CreatedAt,
            character_card::Column::UpdatedAt, // Added
            character_card::Column::DeletedAt,
        ])
        .filter(character_card::Column::DeletedAt.is_null());

    // 按分类筛选
    if let Some(cat_id) = query.category_id {
        select = select.filter(character_card::Column::CategoryId.eq(cat_id));
    }

    // 按名称、描述、作者、标签、概览搜索
    if let Some(search) = &query.search {
        if !search.is_empty() {
            select = select.filter(
                sea_orm::Condition::any()
                    .add(character_card::Column::Name.contains(search))
                    .add(character_card::Column::Description.contains(search))
                    .add(character_card::Column::Author.contains(search))
                    .add(character_card::Column::Tags.contains(search))
                    .add(character_card::Column::CustomSummary.contains(search)),
            );
        }
    }

    // Sorting
    let order = match query.order.as_deref() {
        Some("asc") => sea_orm::Order::Asc,
        _ => sea_orm::Order::Desc,
    };

    select = match query.sort.as_deref() {
        Some("name") => select.order_by(character_card::Column::Name, order),
        Some("created_at") => select.order_by(character_card::Column::CreatedAt, order),
        Some("updated_at") => select.order_by(character_card::Column::UpdatedAt, order),
        _ => select.order_by(character_card::Column::UpdatedAt, sea_orm::Order::Desc), // Default: Last updated
    };

    // Pagination defaults
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let paginator = select.into_model::<CardListRow>().paginate(&db, page_size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_pages = paginator
        .num_pages()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let cards = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<CardListItem> = cards
        .into_iter()
        .map(|c| {
            let tags: Vec<String> = serde_json::from_str(&c.tags).unwrap_or_default();
            CardListItem {
                id: c.id,
                name: c.name,
                description: None, // Optimized out
                author: c.author,
                avatar: c.avatar,
                category_id: c.category_id,
                tags,
                rating: c.rating,
                cover_blur: c.cover_blur,
                version: c.version,
                created_at: chrono::Utc.from_utc_datetime(&c.created_at),
                updated_at: chrono::Utc.from_utc_datetime(&c.updated_at), // Added
                deleted_at: c.deleted_at.map(|d| chrono::Utc.from_utc_datetime(&d)),
            }
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}

/// GET /api/cards/:id - 获取角色卡详情
pub async fn get_details(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<Json<character_card::Model>, (StatusCode, String)> {
    let card = character_card::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "角色卡不存在".to_string()))?;

    // 如果还没有 token 统计数据（或者是旧数据），则计算并更新
    if card.token_count_total.is_none() {
        let json: Value = serde_json::from_str(&card.data).unwrap_or(Value::Null);
        let counts = calculate_card_tokens(&json);

        let mut active: character_card::ActiveModel = card.clone().into();
        active.token_count_total = Set(Some(counts.total));
        active.token_count_spec = Set(Some(counts.spec));
        active.token_count_wb = Set(Some(counts.wb));
        active.token_count_other = Set(Some(counts.other));

        let updated = active
            .update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        return Ok(Json(updated));
    }

    Ok(Json(card))
}

#[derive(Deserialize)]
pub struct UpdateCardRequest {
    pub category_id: Option<Option<Uuid>>,
    pub tags: Option<Vec<String>>,
    pub rating: Option<f64>,
    pub cover_blur: Option<bool>,
    // New fields
    pub name: Option<String>,
    pub description: Option<String>,
    pub first_mes: Option<String>,
    pub alternate_greetings: Option<Vec<String>>,
    pub mes_example: Option<String>,
    pub scenario: Option<String>,
    pub personality: Option<String>,
    pub creator: Option<String>, // 创作者（仅 source=local 时可编辑）
    pub creator_notes: Option<String>,
    pub system_prompt: Option<String>,
    pub character_version: Option<String>,
    pub user_note: Option<String>,
    pub custom_summary: Option<String>,
    pub character_book: Option<Value>,
    pub extensions: Option<Value>,
    // Partial extension update: only update regex_scripts
    pub regex_scripts: Option<Value>,
    // 认领作品: 将 source 从 import 改为 local
    pub source: Option<String>,
}

/// PATCH /api/cards/:id - 更新角色卡
pub async fn update(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCardRequest>,
) -> Result<Json<character_card::Model>, (StatusCode, String)> {
    let existing = character_card::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "角色卡不存在".to_string()))?;

    let mut active: character_card::ActiveModel = existing.clone().into();
    let mut spec_modified = false;
    let mut current_json: Value = serde_json::from_str(&existing.data).unwrap_or(Value::Null);

    // Update fields
    if let Some(cat_id) = payload.category_id {
        active.category_id = Set(cat_id);
    }
    if let Some(rating) = payload.rating {
        active.rating = Set(rating);
    }
    if let Some(cover_blur) = payload.cover_blur {
        active.cover_blur = Set(cover_blur);
    }
    if let Some(note) = payload.user_note {
        active.user_note = Set(Some(note));
    }
    if let Some(summary) = payload.custom_summary {
        active.custom_summary = Set(Some(summary));
    }
    // 认领作品：更新 source 字段
    if let Some(source) = payload.source {
        active.source = Set(source);
    }

    // Helper closure to update JSON field at specific path
    // path: "key" (root) or "data.key" (inside data object)
    let mut update_json = |key: &str, value: Value| {
        if key.starts_with("data.") {
            let real_key = &key[5..];
            // Update inside data object (V2)
            if let Some(data) = current_json.get_mut("data") {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert(real_key.to_string(), value);
                }
            }
        } else {
            // Update at root (V1/V3)
            if let Some(obj) = current_json.as_object_mut() {
                obj.insert(key.to_string(), value);
            }
        }
    };

    // --- Sync Logic Start ---

    // 1. Tags: DB column + JSON root + JSON data.data
    if let Some(tags) = payload.tags {
        let tags_json = serde_json::to_string_pretty(&tags)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("标签格式错误: {}", e)))?;
        active.tags = Set(tags_json.clone());

        update_json("tags", serde_json::json!(tags));
        update_json("data.tags", serde_json::json!(tags));

        spec_modified = true;
    }

    // 2. Name: DB column + JSON root + JSON data.data
    if let Some(name) = payload.name {
        active.name = Set(name.clone());
        update_json("name", Value::String(name.clone()));
        update_json("data.name", Value::String(name));
        spec_modified = true;
    }

    // 3. Description: DB column + JSON root + JSON data.data
    if let Some(desc) = payload.description {
        active.description = Set(Some(desc.clone()));
        update_json("description", Value::String(desc.clone()));
        update_json("data.description", Value::String(desc));
        spec_modified = true;
    }

    // 4. First Message: JSON root (first_mes) + JSON data.data (first_mes)
    if let Some(val) = payload.first_mes {
        update_json("first_mes", Value::String(val.clone()));
        update_json("data.first_mes", Value::String(val));
        spec_modified = true;
    }

    // 5. Alternate Greetings: JSON data.data (alternate_greetings)
    // Note: V2 specific, no V1 equivalent usually, but we check if root supports it? usually not.
    if let Some(val) = payload.alternate_greetings {
        update_json("data.alternate_greetings", serde_json::json!(val));
        // No root update for alternate_greetings as per mapping plan
        spec_modified = true;
    }

    // 6. Mes Example: JSON root (mes_example) + JSON data.data (mes_example)
    if let Some(val) = payload.mes_example {
        update_json("mes_example", Value::String(val.clone()));
        update_json("data.mes_example", Value::String(val));
        spec_modified = true;
    }

    // 7. Scenario: JSON root (scenario) + JSON data.data (scenario)
    if let Some(val) = payload.scenario {
        update_json("scenario", Value::String(val.clone()));
        update_json("data.scenario", Value::String(val));
        spec_modified = true;
    }

    // 7.5. Personality: JSON root (personality) + JSON data.data (personality)
    if let Some(val) = payload.personality {
        update_json("personality", Value::String(val.clone()));
        update_json("data.personality", Value::String(val));
        spec_modified = true;
    }

    // 8. Creator Notes: JSON root (creatorcomment) + JSON data.data (creator_notes)
    if let Some(val) = payload.creator_notes {
        update_json("creatorcomment", Value::String(val.clone()));
        update_json("data.creator_notes", Value::String(val));
        spec_modified = true;
    }

    // 8.5. Creator: JSON data.data (creator) + DB author field
    if let Some(val) = payload.creator {
        update_json("data.creator", Value::String(val.clone()));
        active.author = Set(Some(val));
        spec_modified = true;
    }

    // 9. System Prompt: JSON data.data (system_prompt)
    if let Some(val) = payload.system_prompt {
        update_json("data.system_prompt", Value::String(val));
        spec_modified = true;
    }

    // 10. Character Version: JSON data.data (character_version)
    if let Some(val) = payload.character_version {
        update_json("data.character_version", Value::String(val));
        spec_modified = true;
    }

    // 11. Character Book (World Info)
    if let Some(val) = payload.character_book {
        update_json("data.character_book", val);
        spec_modified = true;
    }

    // 12. Extensions (World Name etc)
    if let Some(val) = payload.extensions {
        update_json("data.extensions", val);
        spec_modified = true;
    }

    // 12.5. Regex Scripts (Partial extension update)
    // This allows updating ONLY regex_scripts without affecting other extension fields
    if let Some(regex_scripts_val) = payload.regex_scripts {
        // Get current extensions object BEFORE using update_json to avoid borrow conflicts
        let current_extensions = if let Some(data) = current_json.get("data") {
            data.get("extensions")
                .cloned()
                .unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Create updated extensions with only regex_scripts modified
        let mut updated_extensions = current_extensions;
        if let Some(obj) = updated_extensions.as_object_mut() {
            obj.insert("regex_scripts".to_string(), regex_scripts_val);
        }

        // Save back to data.extensions
        if let Some(data) = current_json.get_mut("data") {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("extensions".to_string(), updated_extensions);
            }
        }
        spec_modified = true;
    }

    // --- Sync Logic End ---

    // Save JSON changes if needed
    if spec_modified {
        active.metadata_modified = Set(true);
        let new_json_str = serde_json::to_string_pretty(&current_json).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("JSON 序列化失败: {}", e),
            )
        })?;
        active.data = Set(new_json_str);

        // Recalculate tokens
        let counts = calculate_card_tokens(&current_json);
        active.token_count_total = Set(Some(counts.total));
        active.token_count_spec = Set(Some(counts.spec));
        active.token_count_wb = Set(Some(counts.wb));
        active.token_count_other = Set(Some(counts.other));
    }

    active.updated_at = Set(chrono::Utc::now().naive_utc());

    let updated_model = active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(Json(updated_model))
}

/// POST /api/cards/:id/cover - Update cover image
#[axum::debug_handler]
pub async fn update_cover(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<StatusCode, (StatusCode, String)> {
    let card = character_card::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Card not found".to_string()))?;

    while let Ok(Some(field)) = multipart.next_field().await {
        let data = field
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        let storage_dir = crate::utils::paths::get_data_path(&format!("cards/{}", id));
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        // Load image
        let img = image::load_from_memory(&data)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Image load failed: {}", e)))?;

        // Resize to 512x768 for both Source (PNG) and Thumbnail (WebP)
        // This satisfies user requirement: "Original PNG must also be cropped/resized to 512x768"
        let resized = img.resize_to_fill(512, 768, image::imageops::FilterType::Lanczos3);

        // 1. Save as Source PNG (v1_source.png)
        let png_name = "v1_source.png";
        let png_path = storage_dir.join(png_name);

        // Write RESIZED image to PNG
        let mut png_data = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut png_data);
            resized
                .write_to(&mut cursor, image::ImageOutputFormat::Png)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("PNG conversion failed: {}", e),
                    )
                })?;
        }

        fs::write(&png_path, &png_data).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Write source png failed: {}", e),
            )
        })?;

        // 2. Save as WebP (Avatar) - 必须生成 WebP，不允许回退到 PNG
        // 使用重试机制确保 WebP 生成成功
        let webp_path = storage_dir.join("v1_thumbnail.webp");
        let max_retries = 3;
        let mut last_error = String::new();

        for attempt in 1..=max_retries {
            match (|| -> Result<(), String> {
                let encoder = webp::Encoder::from_image(&resized)
                    .map_err(|e| format!("WebP encoder init failed: {}", e))?;
                let webp_data = encoder.encode(85.0);

                // Validate WebP data is not empty
                if webp_data.is_empty() {
                    return Err("WebP encoding produced empty data".to_string());
                }

                // Use std::fs for sync write
                std::fs::write(&webp_path, &*webp_data)
                    .map_err(|e| format!("Write thumbnail failed: {}", e))?;

                // Verify file was written
                if !webp_path.exists() {
                    return Err("WebP file was not created".to_string());
                }

                Ok(())
            })() {
                Ok(_) => {
                    tracing::info!(
                        "WebP thumbnail generated successfully on attempt {}",
                        attempt
                    );
                    break;
                }
                Err(e) => {
                    last_error = e.clone();
                    tracing::warn!(
                        "WebP thumbnail generation failed on attempt {}/{}: {}",
                        attempt,
                        max_retries,
                        e
                    );
                    if attempt < max_retries {
                        // Brief delay before retry
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }

        // Check if WebP was successfully created
        if !webp_path.exists() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "WebP thumbnail generation failed after {} attempts: {}",
                    max_retries, last_error
                ),
            ));
        }

        let avatar_path = format!("/cards/{}/v1_thumbnail.webp", id);

        // Update DB
        let mut active: character_card::ActiveModel = card.into();
        active.avatar = Set(Some(avatar_path));
        // Mark as modified since source image changed (and likely needs new metadata injected on export)
        active.metadata_modified = Set(true);
        active.updated_at = Set(chrono::Utc::now().naive_utc());

        active
            .update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        return Ok(StatusCode::OK);
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

/// GET /api/cards/:id/export - Export card
async fn _get_card_file_data(
    db: &DatabaseConnection,
    card: character_card::Model,
) -> Result<(String, Vec<u8>), String> {
    let storage_dir = crate::utils::paths::get_data_path(&format!("cards/{}", card.id));
    let png_path = storage_dir.join("v1_source.png");

    let safe_name = card
        .name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>();

    if card.metadata_modified && png_path.exists() {
        // Inject JSON
        let file_data = fs::read(&png_path)
            .await
            .map_err(|e| format!("Read PNG failed: {}", e))?;

        let decoder = png::Decoder::new(Cursor::new(&file_data));
        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("PNG Decode Error: {}", e))?;

        // Allocate buffer for image data
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|e| format!("PNG Frame Error: {}", e))?;
        let bytes = &buf[..info.buffer_size()];

        // Encode new PNG with updated metadata
        let mut output_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut output_data, info.width, info.height);
            encoder.set_color(info.color_type);
            encoder.set_depth(info.bit_depth);

            // Add tEXt chunk
            let json_base64 = general_purpose::STANDARD.encode(card.data.as_bytes());
            encoder
                .add_text_chunk("ccv3".to_string(), json_base64)
                .map_err(|e| format!("PNG Text Error: {}", e))?;

            let mut writer = encoder
                .write_header()
                .map_err(|e| format!("PNG Header Error: {}", e))?;
            writer
                .write_image_data(bytes)
                .map_err(|e| format!("PNG Write Error: {}", e))?;
        }

        // Write back to source
        if let Err(e) = fs::write(&png_path, &output_data).await {
            tracing::error!("Failed to overwrite updated PNG to source file: {}", e);
        } else {
            // Update DB
            let mut active: character_card::ActiveModel = card.into();
            active.metadata_modified = Set(false);
            if let Err(e) = active.update(db).await {
                tracing::error!("Failed to reset metadata_modified flag: {}", e);
            }
        }

        Ok((format!("{}.png", safe_name), output_data))
    } else if png_path.exists() {
        // Read directly
        let file_data = fs::read(&png_path)
            .await
            .map_err(|e| format!("Read PNG failed: {}", e))?;
        Ok((format!("{}.png", safe_name), file_data))
    } else {
        // JSON Fallback
        Ok((format!("{}.json", safe_name), card.data.into_bytes()))
    }
}

pub async fn export_card(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let card = character_card::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Card not found".to_string()))?;

    let (filename, data) = _get_card_file_data(&db, card)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
    );
    if filename.ends_with(".json") {
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    } else {
        headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    }

    Ok((headers, Body::from(data)))
}

#[derive(Deserialize)]
pub struct BatchExportRequest {
    pub ids: Vec<Uuid>,
}

pub async fn batch_export_cards(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchExportRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if payload.ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No cards selected".to_string()));
    }

    // Find all cards
    let cards = character_card::Entity::find()
        .filter(character_card::Column::Id.is_in(payload.ids))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Process concurrently
    let results: Vec<Result<(String, Vec<u8>), String>> = futures::stream::iter(cards)
        .map(|card| {
            let db = db.clone();
            async move { _get_card_file_data(&db, card).await }
        })
        .buffer_unordered(10)
        .collect()
        .await;

    // Create Zip
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);

    let mut names = std::collections::HashSet::new();

    for res in results {
        if let Ok((mut filename, data)) = res {
            // Deduplicate logic
            let mut i = 1;
            // Handle filenames without logic stems carefully ifneeded, but safe_name ensures alphanumeric.
            // Split extension
            let (stem, ext) = if let Some(idx) = filename.rfind('.') {
                (&filename[..idx], &filename[idx + 1..])
            } else {
                (filename.as_str(), "")
            };

            let stem = stem.to_string();
            let ext = ext.to_string();

            while names.contains(&filename) {
                filename = format!("{}_{}.{}", stem, i, ext);
                i += 1;
            }
            names.insert(filename.clone());

            let _ = zip_writer.start_file(filename, options);
            let _ = zip_writer.write_all(&data);
        }
    }

    let cursor = zip_writer.finish().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Zip error: {}", e),
        )
    })?;
    let zip_buffer = cursor.into_inner();

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"batch_export.zip\"".parse().unwrap(),
    );
    headers.insert(header::CONTENT_TYPE, "application/zip".parse().unwrap());

    Ok((headers, Body::from(zip_buffer)))
}

#[derive(Deserialize)]
pub struct BatchUpdateCategoryRequest {
    pub ids: Vec<Uuid>,
    pub category_id: Option<Uuid>,
}

pub async fn batch_update_category(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchUpdateCategoryRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if payload.ids.is_empty() {
        return Ok(StatusCode::OK);
    }

    // 批量更新
    character_card::Entity::update_many()
        .col_expr(
            character_card::Column::CategoryId,
            payload.category_id.into(),
        )
        .filter(character_card::Column::Id.is_in(payload.ids))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<Uuid>,
}

pub async fn batch_soft_delete(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if payload.ids.is_empty() {
        return Ok(StatusCode::OK);
    }

    character_card::Entity::update_many()
        .col_expr(
            character_card::Column::DeletedAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now().naive_utc()),
        )
        .filter(character_card::Column::Id.is_in(payload.ids))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(StatusCode::OK)
}

/// DELETE /api/cards/:id - 软删除
pub async fn soft_delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut active = character_card::ActiveModel {
        id: Set(id),
        ..Default::default()
    };
    active.deleted_at = Set(Some(chrono::Utc::now().naive_utc()));

    active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(StatusCode::OK)
}

/// POST /api/cards/{id}/overwrite - 覆盖现有角色卡
/// 删除旧记录并重新导入新文件

// ============ 回收站 API ============

/// GET /api/trash/cards - 回收站列表
#[derive(Debug, FromQueryResult)]
struct TrashCardRow {
    pub id: Uuid,
    pub name: String,
    // pub description: Option<String>, // Removed for performance
    pub author: Option<String>,
    pub avatar: Option<String>,
    pub category_id: Option<Uuid>,
    pub tags: String,
    pub rating: f64,
    pub cover_blur: bool,
    pub version: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime, // Added
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

pub async fn list_trash(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<CardListItem>>, (StatusCode, String)> {
    // 仅选择必要的列，避免加载 data 和 description
    let cards = character_card::Entity::find()
        .select_only()
        .column(character_card::Column::Id)
        .column(character_card::Column::Name)
        // .column(character_card::Column::Description)
        .column(character_card::Column::Author)
        .column(character_card::Column::Avatar)
        .column(character_card::Column::CategoryId)
        .column(character_card::Column::Tags)
        .column(character_card::Column::Rating)
        .column(character_card::Column::CoverBlur)
        .column(character_card::Column::Version)
        .column(character_card::Column::CreatedAt)
        .column(character_card::Column::UpdatedAt)
        .column(character_card::Column::DeletedAt)
        .filter(character_card::Column::DeletedAt.is_not_null())
        .order_by_desc(character_card::Column::DeletedAt)
        .into_model::<TrashCardRow>()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response: Vec<CardListItem> = cards
        .into_iter()
        .map(|c| {
            let tags: Vec<String> = serde_json::from_str(&c.tags).unwrap_or_default();
            
            // Safety check: Detect if avatar is Base64 (huge string)
            let avatar = if let Some(ref a) = c.avatar {
                if a.len() > 1024 { // Threshold: 1KB. Paths are rarely this long.
                    tracing::warn!("Card {} (Trash) has oversized avatar field ({} bytes). Omitting to prevent lag.", c.id, a.len());
                    None
                } else {
                    c.avatar
                }
            } else {
                None
            };

            CardListItem {
                id: c.id,
                name: c.name,
                description: None, // Optimized out
                author: c.author,
                avatar,
                category_id: c.category_id,
                tags,
                rating: c.rating,
                cover_blur: c.cover_blur,
                version: c.version,
                created_at: chrono::Utc.from_utc_datetime(&c.created_at),
                updated_at: chrono::Utc.from_utc_datetime(&c.updated_at),
                deleted_at: c.deleted_at.map(|d| chrono::Utc.from_utc_datetime(&d)),
            }
        })
        .collect();

    Ok(Json(response))
}

/// POST /api/trash/cards/:id/restore - 恢复
pub async fn restore_card(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut active = character_card::ActiveModel {
        id: Set(id),
        ..Default::default()
    };
    active.deleted_at = Set(None);

    active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(StatusCode::OK)
}

/// DELETE /api/trash/cards/:id - 永久删除
pub async fn permanent_delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. 删除关联文件
    // 删除 data/cards/[id] 目录，清理所有相关图片和数据
    let storage_dir = crate::utils::paths::get_data_path("cards");
    let card_path = storage_dir.join(id.to_string());
    if card_path.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&card_path).await {
            tracing::warn!("Failed to delete card directory: {}", e);
        }
    }

    character_card::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(StatusCode::OK)
}

/// POST /api/trash/cards/batch-delete - 批量永久删除
#[derive(Deserialize)]
pub struct BatchTrashDeleteRequest {
    pub ids: Vec<Uuid>,
}

pub async fn batch_delete_trash(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchTrashDeleteRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if payload.ids.is_empty() {
        return Ok(StatusCode::OK);
    }

    let storage_dir = crate::utils::paths::get_data_path("cards");

    // 删除每个卡片的文件目录
    for id in &payload.ids {
        let card_path = storage_dir.join(id.to_string());
        if card_path.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(&card_path).await {
                tracing::warn!("Failed to delete card directory {}: {}", id, e);
            }
        }
    }

    // 批量删除数据库记录
    character_card::Entity::delete_many()
        .filter(character_card::Column::Id.is_in(payload.ids))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(StatusCode::OK)
}

/// DELETE /api/trash/cards/clear - 清空回收站
#[derive(Serialize)]
pub struct ClearTrashResponse {
    pub deleted_count: u64,
}

pub async fn clear_trash(
    State(db): State<DatabaseConnection>,
) -> Result<Json<ClearTrashResponse>, (StatusCode, String)> {
    // 1. 获取所有已删除卡片的 ID
    let cards = character_card::Entity::find()
        .filter(character_card::Column::DeletedAt.is_not_null())
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ids: Vec<Uuid> = cards.iter().map(|c| c.id).collect();
    let count = ids.len() as u64;

    if ids.is_empty() {
        return Ok(Json(ClearTrashResponse { deleted_count: 0 }));
    }

    // 2. 删除所有关联文件
    let storage_dir = crate::utils::paths::get_data_path("cards");
    for id in &ids {
        let card_path = storage_dir.join(id.to_string());
        if card_path.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(&card_path).await {
                tracing::warn!("Failed to delete card directory {}: {}", id, e);
            }
        }
    }

    // 3. 批量删除数据库记录
    character_card::Entity::delete_many()
        .filter(character_card::Column::Id.is_in(ids))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    invalidate_cache();
    Ok(Json(ClearTrashResponse {
        deleted_count: count,
    }))
}
