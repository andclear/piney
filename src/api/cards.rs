use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use base64::{engine::general_purpose, Engine as _};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Cursor;
use std::path::Path as StdPath;
use tokio::fs;
use tracing::warn;
use uuid::Uuid;

use crate::entities::character_card;

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
    let storage_dir = "data/cards";
    if !StdPath::new(storage_dir).exists() {
        fs::create_dir_all(storage_dir)
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
            match process_png_card(&data, storage_dir).await {
                Ok((uuid, json_str)) => {
                    // 头像路径指向版本化文件夹中的缩略图 (v1)
                    let avatar_path = format!("/cards/{}/v1_thumbnail.webp", uuid);

                    match save_card_to_db(&db, uuid, json_str, Some(avatar_path)).await {
                        Ok(_) => {
                            results.push(ImportResult {
                                file_name,
                                status: "success".to_string(),
                                reason: None,
                            });
                        }
                        Err(e) => {
                            results.push(ImportResult {
                                file_name,
                                status: "error".to_string(),
                                reason: Some(format!("数据库保存失败: {}", e)),
                            });
                        }
                    }
                }
                Err(e) => {
                    results.push(ImportResult {
                        file_name,
                        status: "error".to_string(),
                        reason: Some(e),
                    });
                }
            }
        } else if is_json {
            // 处理 JSON 角色卡
            let json_result = (|| {
                let json_string =
                    String::from_utf8(data.to_vec()).map_err(|_| "JSON 编码无效".to_string())?;
                // 验证 JSON
                let v: Value = serde_json::from_str(&json_string)
                    .map_err(|e| format!("无效的 JSON: {}", e))?;

                // 检查是否为世界书 (World Info)
                // 世界书特征: 根节点包含 "entries"，且不包含角色卡特有的 "data" (V2) 或 "name" (V1)
                if v.get("entries").is_some() && v.get("data").is_none() && v.get("name").is_none()
                {
                    return Err("检测到世界书文件，请在世界书页面进行导入".to_string());
                }

                // 检查是否为有效的角色卡
                // 必须包含 "data" (V2/V3) 或 "name" (V1)
                if v.get("data").is_none() && v.get("name").is_none() {
                    return Err("无效的角色卡格式：缺少必要的 'data' 或 'name' 字段".to_string());
                }

                Ok(json_string)
            })();

            match json_result {
                Ok(json_str) => {
                    // JSON 导入通常没有头像，生成随机 UUID
                    let uuid = Uuid::new_v4();

                    // 为 JSON 卡片也创建专属目录，方便后续添加封面
                    let card_path = StdPath::new(storage_dir).join(uuid.to_string());
                    if !card_path.exists() {
                        if let Err(e) = fs::create_dir_all(&card_path).await {
                            results.push(ImportResult {
                                file_name: file_name.clone(),
                                status: "error".to_string(),
                                reason: Some(format!("创建目录失败: {}", e)),
                            });
                            continue;
                        }
                    }

                    match save_card_to_db(&db, uuid, json_str, Some("/default.webp".to_string()))
                        .await
                    {
                        Ok(_) => {
                            results.push(ImportResult {
                                file_name,
                                status: "success".to_string(),
                                reason: None,
                            });
                        }
                        Err(e) => {
                            results.push(ImportResult {
                                file_name,
                                status: "error".to_string(),
                                reason: Some(format!("数据库保存失败: {}", e)),
                            });
                        }
                    }
                }
                Err(e) => {
                    results.push(ImportResult {
                        file_name,
                        status: "error".to_string(),
                        reason: Some(e),
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

    Ok(Json(results))
}

async fn process_png_card(data: &[u8], storage_dir: &str) -> Result<(Uuid, String), String> {
    // 1. 手动解析 PNG Chunks (参考 user 提供的可靠逻辑)
    // 这种方法不依赖外部库对 chunk 顺序的严格校验，更健壮
    let mut extracted_json: Option<String> = None;

    // PNG 签名校验
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if data.len() < 8 || &data[..8] != PNG_SIGNATURE {
        return Err("非法的 PNG 文件签名".to_string());
    }

    let mut offset = 8;
    while offset + 8 <= data.len() {
        // 读取 Chunk 长度 (Big Endian)
        let length_bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
        let length = u32::from_be_bytes(length_bytes) as usize;

        // 读取 Chunk 类型
        let chunk_type = &data[offset + 4..offset + 8];

        // 数据区域范围
        let data_start = offset + 8;
        let data_end = data_start + length;

        // 确保不越界
        if data_end > data.len() {
            warn!("PNG Chunk 越界，停止解析");
            break;
        }

        // 处理 tEXt 区块
        if chunk_type == b"tEXt" {
            let chunk_data = &data[data_start..data_end];
            // tEXt 格式: Keyword + \0 + Text
            // 查找 null separator
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                let keyword_bytes = &chunk_data[..null_pos];
                let text_bytes = &chunk_data[null_pos + 1..]; // 跳过 \0

                if let Ok(keyword) = String::from_utf8(keyword_bytes.to_vec()) {
                    // 检查 keyword
                    if keyword == "ccv3" || keyword == "chara" {
                        // 尝试 Base64 解码
                        if let Ok(decoded) = general_purpose::STANDARD.decode(text_bytes) {
                            if let Ok(s) = String::from_utf8(decoded) {
                                extracted_json = Some(s);
                                // 优先使用 ccv3，如果找到直接退出
                                if keyword == "ccv3" {
                                    break;
                                }
                            }
                        } else {
                            // Base64 失败，尝试直接解析 (Fallback 逻辑)
                            if let Ok(s) = String::from_utf8(text_bytes.to_vec()) {
                                warn!("{} Base64 解码失败，回退为直接文本", keyword);
                                extracted_json = Some(s);
                                if keyword == "ccv3" {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 遇到 IEND 也可以继续，以防 tEXt 在 IEND 之后 (虽然不标准但为了兼容)
        // 但通常 IEND 就是结束。参考文档逻辑是“遍历 Chunk”。
        // 如果我们要在 IEND 后继续，只需不 break 即可。
        // 为了安全起见，我们继续遍历直到文件末尾。

        // 移动到下一个 Chunk (包含 4字节 CRC)
        offset = data_end + 4;
    }

    if extracted_json.is_none() {
        warn!("手动遍历 Chunk 未找到 ccv3 或 chara 元数据");
        // 保留之前的暴力扫描作为最后的防线?
        // 既然已经完整遍历了结构，如果没有，那可能真没有，或者结构极其损坏。
        // 参考文档没有提暴力扫描，只提了 Chunk 遍历。
        // 此处不再回退，直接报错，避免误判。
    }

    let json_str = extracted_json.ok_or("无效的角色卡图片：未找到元数据 (ccv3/chara)")?;
    // 验证一下 JSON 格式是否合法，但不改变内容
    let _: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("元数据 JSON 无效: {}", e))?;

    // 2. 生成 UUID 并创建专属文件夹
    let uuid = Uuid::new_v4();
    let card_dir = StdPath::new(storage_dir).join(uuid.to_string());

    if !card_dir.exists() {
        fs::create_dir_all(&card_dir)
            .await
            .map_err(|e| format!("创建角色卡目录失败: {}", e))?;
    }

    // 3. 保存原始 PNG (v1_source.png)
    let png_name = "v1_source.png";
    let png_path = card_dir.join(png_name);
    fs::write(&png_path, data)
        .await
        .map_err(|e| format!("保存原始 PNG 失败: {}", e))?;

    // 4. 生成 WebP 缩略图 (v1_thumbnail.webp)
    let img = image::load_from_memory(data).map_err(|e| format!("图片加载失败: {}", e))?;

    // 保持原始尺寸和比例 (512x768)，不进行强制缩放
    // 仅转为 WebP 格式以优化体积 (75% 质量)
    let encoder = webp::Encoder::from_image(&img).map_err(|e| format!("WebP 编码失败: {}", e))?;
    let webp_data = encoder.encode(75.0).to_vec();

    let webp_name = "v1_thumbnail.webp";
    let webp_path = card_dir.join(webp_name);
    fs::write(&webp_path, &webp_data)
        .await
        .map_err(|e| format!("保存 WebP 缩略图失败: {}", e))?;

    Ok((uuid, json_str))
}

async fn save_card_to_db(
    db: &DatabaseConnection,
    uuid: Uuid,
    json_str: String,
    avatar: Option<String>,
) -> Result<(), String> {
    // 解析 JSON 仅用于提取索引字段，data 字段直接存原始字符串（不做任何修改）
    let json: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("解析 JSON 失败: {}", e))?;

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

    // 版本号独立管理，不从角色卡 JSON 中提取，默认为 None（前端显示为 1.0）
    let active_model = character_card::ActiveModel {
        id: Set(uuid),
        name: Set(name),
        description: Set(description),
        author: Set(author),
        avatar: Set(avatar),
        spec: Set(spec),
        spec_version: Set(spec_version),
        data: Set(pretty_json_str),
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
    };

    active_model
        .insert(db)
        .await
        .map_err(|e| format!("数据库错误: {}", e))?;
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

    let storage_dir = "data/cards";
    if !StdPath::new(storage_dir).exists() {
        match fs::create_dir_all(storage_dir).await {
            Ok(_) => logs.push(format!("创建存储目录 {} 成功", storage_dir)),
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
                        match save_card_to_db(&db, uuid, json_str.clone(), None).await {
                            Ok(_) => {
                                logs.push("保存成功。".to_string());
                                logs.push(format!("UUID: {}", uuid));

                                // 4. 从数据库回读验证
                                logs.push("正在从数据库回读 verify...".to_string());
                                match character_card::Entity::find_by_id(uuid).one(&db).await {
                                    Ok(Some(model)) => {
                                        logs.push("数据库读取成功。".to_string());
                                        let db_data_len = model.data.len();
                                        logs.push(format!("DB中 data 字段长度: {}", db_data_len));

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
                                                        "验证失败：数据内容不一致！".to_string(),
                                                    );
                                                }
                                            }
                                            _ => logs.push(
                                                "验证警告：无法解析 JSON 进行比对。".to_string(),
                                            ),
                                        }

                                        saved_json = Some(model.data);
                                    }
                                    Ok(None) => {
                                        logs.push("错误：保存后无法查找到记录！".to_string())
                                    }
                                    Err(e) => logs.push(format!("回读查询失败: {}", e)),
                                }
                            }
                            Err(e) => logs.push(format!("保存数据库失败: {}", e)),
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

// ============ 列表和更新 API ============

#[derive(Deserialize)]
pub struct ListCardsQuery {
    pub category_id: Option<Uuid>,
    pub search: Option<String>,
    pub tags: Option<String>, // 逗号分隔的标签
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
    pub created_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

/// GET /api/cards - 获取角色卡列表
pub async fn list(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ListCardsQuery>,
) -> Result<Json<Vec<CardListItem>>, (StatusCode, String)> {
    let mut select =
        character_card::Entity::find().filter(character_card::Column::DeletedAt.is_null());

    // 按分类筛选
    if let Some(cat_id) = query.category_id {
        select = select.filter(character_card::Column::CategoryId.eq(cat_id));
    }

    // 按名称搜索
    if let Some(search) = &query.search {
        if !search.is_empty() {
            select = select.filter(character_card::Column::Name.contains(search));
        }
    }

    let cards = select
        .order_by_desc(character_card::Column::CreatedAt)
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response: Vec<CardListItem> = cards
        .into_iter()
        .map(|c| {
            let tags: Vec<String> = serde_json::from_str(&c.tags).unwrap_or_default();
            CardListItem {
                id: c.id,
                name: c.name,
                description: c.description,
                author: c.author,
                avatar: c.avatar,
                category_id: c.category_id,
                tags,
                rating: c.rating,
                cover_blur: c.cover_blur,
                version: c.version,
                created_at: c.created_at,
                deleted_at: c.deleted_at,
            }
        })
        .collect();

    Ok(Json(response))
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
    pub creator_notes: Option<String>,
    pub system_prompt: Option<String>,
    pub character_version: Option<String>,
    pub user_note: Option<String>,
    pub custom_summary: Option<String>,
    pub character_book: Option<Value>,
    pub extensions: Option<Value>,
}

/// PATCH /api/cards/:id - 更新角色卡
pub async fn update(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCardRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
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
    }

    active.updated_at = Set(chrono::Utc::now().naive_utc());

    active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
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

        // Load and Resize (Synchronous block)
        let webp_data = {
            let img = image::load_from_memory(&data)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Image load failed: {}", e)))?;

            // 512x768 (Fill/Crop to ratio? Or Force Resize?)
            let resized = img.resize_to_fill(512, 768, image::imageops::FilterType::Lanczos3);

            // Save as WebP (Avatar)
            let encoder = webp::Encoder::from_image(&resized).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("WebP init failed: {}", e),
                )
            })?;
            encoder.encode(85.0).to_vec()
        }; // img, resized, encoder dropped here

        let storage_dir = format!("data/cards/{}", id);
        if !StdPath::new(&storage_dir).exists() {
            fs::create_dir_all(&storage_dir)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        let file_path = format!("{}/v1_thumbnail.webp", storage_dir); // Keeping naming convention
        fs::write(&file_path, &*webp_data).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Write file failed: {}", e),
            )
        })?;

        // Update DB if avatar path was missing (though import sets it)
        let mut active: character_card::ActiveModel = card.into();
        let expected_path = format!("/cards/{}/v1_thumbnail.webp", id);
        active.avatar = Set(Some(expected_path));
        active
            .update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        return Ok(StatusCode::OK);
    }

    Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()))
}

/// GET /api/cards/:id/export - Export card
pub async fn export_card(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let card = character_card::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Card not found".to_string()))?;

    let storage_dir = format!("data/cards/{}", id);
    let png_path = std::path::Path::new(&storage_dir).join("v1_source.png");

    // Determine filename
    let filename = format!(
        "{}.png",
        card.name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
            .collect::<String>()
    );

    // Headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}.png\"", filename)
            .parse()
            .unwrap(), // Safe ascii fallback needed?
    );
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());

    // Logic:
    // If metadata_modified == true AND PNG exists -> Inject new JSON.
    // Else -> Stream original PNG (or generate from DB if no PNG? Assuming PNG exists from import).

    if card.metadata_modified && png_path.exists() {
        // Inject JSON
        let file_data = fs::read(&png_path).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Read PNG failed: {}", e),
            )
        })?;

        let decoder = png::Decoder::new(Cursor::new(&file_data));
        let mut reader = decoder.read_info().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("PNG Decode Error: {}", e),
            )
        })?;

        // Allocate buffer for image data
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("PNG Frame Error: {}", e),
            )
        })?;
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
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("PNG Text Error: {}", e),
                    )
                })?;

            let mut writer = encoder.write_header().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("PNG Header Error: {}", e),
                )
            })?;
            writer.write_image_data(bytes).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("PNG Write Error: {}", e),
                )
            })?;
        }

        return Ok((headers, Body::from(output_data)));
    } else if png_path.exists() {
        // Stream original
        let file = tokio::fs::File::open(png_path)
            .await
            .map_err(|e| (StatusCode::NOT_FOUND, format!("File open failed: {}", e)))?;
        let stream = tokio_util::io::ReaderStream::new(file);
        let body = Body::from_stream(stream);

        return Ok((headers, body));
    }

    Err((StatusCode::NOT_FOUND, "Source file not found".to_string()))
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

    Ok(StatusCode::OK)
}

// ============ 回收站 API ============

/// GET /api/trash/cards - 回收站列表
pub async fn list_trash(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<CardListItem>>, (StatusCode, String)> {
    let cards = character_card::Entity::find()
        .filter(character_card::Column::DeletedAt.is_not_null())
        .order_by_desc(character_card::Column::DeletedAt)
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response: Vec<CardListItem> = cards
        .into_iter()
        .map(|c| {
            let tags: Vec<String> = serde_json::from_str(&c.tags).unwrap_or_default();
            CardListItem {
                id: c.id,
                name: c.name,
                description: c.description,
                author: c.author,
                avatar: c.avatar,
                category_id: c.category_id,
                tags,
                rating: c.rating,
                cover_blur: c.cover_blur,
                version: c.version,
                created_at: c.created_at,
                deleted_at: c.deleted_at,
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

    Ok(StatusCode::OK)
}

/// DELETE /api/trash/cards/:id - 永久删除
pub async fn permanent_delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. 删除关联文件
    // 删除 data/cards/[id] 目录，清理所有相关图片和数据
    let storage_dir = "data/cards";
    let card_path = std::path::Path::new(storage_dir).join(id.to_string());
    if card_path.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&card_path).await {
            tracing::warn!("Failed to delete card directory: {}", e);
        }
    }

    character_card::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
