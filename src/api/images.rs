//! 图库 API
//!
//! 提供图片的导入、管理、批量操作等功能

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{TimeZone, Utc};
use image::{DynamicImage, GenericImageView};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

use crate::entities::image as image_entity;

// ==================== 请求/响应结构 ====================

#[derive(Deserialize)]
pub struct ListQuery {
    pub category_id: Option<Uuid>,
    pub color_category: Option<String>,
    pub is_ai: Option<bool>,
    pub is_authorized: Option<bool>,
    pub is_favorite: Option<bool>,
    pub search: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Deserialize)]
pub struct UpdateImageRequest {
    pub title: Option<String>,
    pub category_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub is_ai: Option<bool>,
    pub ai_platform: Option<String>,
    pub ai_prompt: Option<String>,
    pub ai_negative_prompt: Option<String>,
    pub is_authorized: Option<bool>,
    pub is_favorite: Option<bool>,
    pub user_notes: Option<String>,
}

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct BatchCategoryRequest {
    pub ids: Vec<Uuid>,
    pub category_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct BatchUpdateRequest {
    pub ids: Vec<Uuid>,
    pub is_ai: Option<bool>,
    pub is_authorized: Option<bool>,
}

#[derive(Serialize)]
pub struct ImageResponse {
    pub id: Uuid,
    pub title: String,
    pub category_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub file_path: String,
    pub thumbnail_path: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub color_category: Option<String>,
    pub is_ai: bool,
    pub ai_platform: Option<String>,
    pub ai_prompt: Option<String>,
    pub ai_negative_prompt: Option<String>,
    pub is_authorized: bool,
    pub is_favorite: bool,
    pub user_notes: Option<String>,
    /// 关联的角色卡 ID 列表
    pub char_cards: Vec<Uuid>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ImageListItem {
    pub id: Uuid,
    pub title: String,
    pub thumbnail_path: String,
    pub width: i32,
    pub height: i32,
    pub is_favorite: bool,
    pub is_ai: bool,
    pub is_authorized: bool,
    pub color_category: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct PaginatedResponse {
    pub items: Vec<ImageListItem>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

impl From<image_entity::Model> for ImageResponse {
    fn from(m: image_entity::Model) -> Self {
        let tags: Vec<String> = serde_json::from_str(&m.tags).unwrap_or_default();
        let char_cards: Vec<Uuid> = serde_json::from_str(&m.char_cards).unwrap_or_default();
        Self {
            id: m.id,
            title: m.title,
            category_id: m.category_id,
            tags,
            file_path: m.file_path,
            thumbnail_path: m.thumbnail_path,
            width: m.width,
            height: m.height,
            file_size: m.file_size,
            color_category: m.color_category,
            is_ai: m.is_ai,
            ai_platform: m.ai_platform,
            ai_prompt: m.ai_prompt,
            ai_negative_prompt: m.ai_negative_prompt,
            is_authorized: m.is_authorized,
            is_favorite: m.is_favorite,
            user_notes: m.user_notes,
            char_cards,
            created_at: Utc.from_utc_datetime(&m.created_at).to_rfc3339(),
        }
    }
}

// ==================== 颜色分类逻辑 ====================

/// 将 RGB 转换为 HSV
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v)
}

/// 根据 HSV 归类到 9 色方案
fn classify_color(h: f32, s: f32, v: f32) -> &'static str {
    // 第一步：极暗判定
    if v < 0.15 {
        return "black";
    }

    // 第二步：无彩色系判定
    if s < 0.10 {
        if v < 0.25 {
            return "black";
        } else if v > 0.80 {
            return "white";
        } else {
            return "gray";
        }
    }

    // 第三步：彩色系判定
    if h <= 15.0 || h > 330.0 {
        "red"
    } else if h <= 45.0 {
        "orange"
    } else if h <= 70.0 {
        "yellow"
    } else if h <= 165.0 {
        "green"
    } else if h <= 200.0 {
        "cyan"
    } else if h <= 260.0 {
        "blue"
    } else {
        "purple"
    }
}

/// 计算图片的主色调
fn calculate_dominant_color(img: &DynamicImage) -> String {
    let (width, height) = img.dimensions();
    let sample_step = ((width * height) as f32 / 1000.0).sqrt().max(1.0) as u32;

    let mut color_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for y in (0..height).step_by(sample_step as usize) {
        for x in (0..width).step_by(sample_step as usize) {
            let pixel = img.get_pixel(x, y);
            let (h, s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
            let color = classify_color(h, s, v);
            *color_counts.entry(color).or_insert(0) += 1;
        }
    }

    color_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color.to_string())
        .unwrap_or_else(|| "gray".to_string())
}

// ==================== AI Prompt 提取 ====================

/// 从 PNG 元数据中提取 AI prompt
fn extract_ai_metadata(data: &[u8]) -> Option<(String, Option<String>, Option<String>)> {
    // 尝试解析 PNG chunks
    if data.len() < 8 {
        return None;
    }

    // 检查 PNG 签名
    let png_sig = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if &data[..8] != png_sig {
        return None;
    }

    let mut offset = 8;
    let mut prompt: Option<String> = None;
    let mut negative_prompt: Option<String> = None;
    let mut platform: Option<String> = None;

    while offset + 12 <= data.len() {
        let length = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        let chunk_type = &data[offset + 4..offset + 8];

        if chunk_type == b"tEXt" || chunk_type == b"iTXt" {
            let chunk_data = &data[offset + 8..offset + 8 + length];
            let text = String::from_utf8_lossy(chunk_data).to_string();

            // NAI 格式: Comment JSON
            if text.starts_with("Comment\0") {
                if let Some(json_str) = text.strip_prefix("Comment\0") {
                    if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                        if let Some(p) = json.get("prompt").and_then(|v| v.as_str()) {
                            prompt = Some(p.to_string());
                            platform = Some("NovelAI".to_string());
                        }
                        if let Some(uc) = json.get("uc").and_then(|v| v.as_str()) {
                            negative_prompt = Some(uc.to_string());
                        }
                    }
                }
            }

            // ComfyUI 格式: prompt
            if text.starts_with("prompt\0") {
                if let Some(json_str) = text.strip_prefix("prompt\0") {
                    if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                        // ComfyUI format is a map of NodeID -> NodeData
                        if let Some(nodes) = json.as_object() {
                            // Find KSampler node
                            for (_, node) in nodes {
                                if let Some(class_type) =
                                    node.get("class_type").and_then(|v| v.as_str())
                                {
                                    if class_type == "KSampler" || class_type == "KSamplerAdvanced"
                                    {
                                        platform = Some("ComfyUI".to_string());

                                        // Helper to find text from input reference
                                        let find_text = |input_key: &str| -> Option<String> {
                                            if let Some(inputs) = node.get("inputs") {
                                                // input is usually [node_id, slot_index]
                                                if let Some(link) =
                                                    inputs.get(input_key).and_then(|v| v.as_array())
                                                {
                                                    if let Some(from_node_id) =
                                                        link.first().and_then(|v| v.as_str())
                                                    {
                                                        if let Some(source_node) =
                                                            nodes.get(from_node_id)
                                                        {
                                                            if let Some(source_class) = source_node
                                                                .get("class_type")
                                                                .and_then(|v| v.as_str())
                                                            {
                                                                if source_class == "CLIPTextEncode"
                                                                {
                                                                    return source_node
                                                                        .get("inputs")
                                                                        .and_then(|i| i.get("text"))
                                                                        .and_then(|t| t.as_str())
                                                                        .map(|s| s.to_string());
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            None
                                        };

                                        if let Some(p) = find_text("positive") {
                                            prompt = Some(p);
                                        }
                                        if let Some(n) = find_text("negative") {
                                            negative_prompt = Some(n);
                                        }

                                        // Find one sampler is enough usually
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // SD 格式: parameters
            if text.starts_with("parameters\0") {
                if let Some(params) = text.strip_prefix("parameters\0") {
                    // 解析 SD 格式
                    let lines: Vec<&str> = params.lines().collect();
                    if !lines.is_empty() {
                        prompt = Some(lines[0].to_string());
                        platform = Some("StableDiffusion".to_string());

                        for line in &lines[1..] {
                            if line.starts_with("Negative prompt:") {
                                negative_prompt = Some(
                                    line.trim_start_matches("Negative prompt:")
                                        .trim()
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
            }
        }

        offset += 8 + length + 4; // header + data + CRC
    }

    if platform.is_some() {
        Some((platform.unwrap(), prompt, negative_prompt))
    } else {
        None
    }
}

// ==================== API 端点 ====================

/// GET /api/images - 获取图片列表
pub async fn list(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    use sea_orm::PaginatorTrait;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let mut select = image_entity::Entity::find();

    // 筛选条件
    if let Some(category_id) = query.category_id {
        select = select.filter(image_entity::Column::CategoryId.eq(category_id));
    }
    if let Some(color) = &query.color_category {
        select = select.filter(image_entity::Column::ColorCategory.eq(color.clone()));
    }
    if let Some(is_ai) = query.is_ai {
        select = select.filter(image_entity::Column::IsAi.eq(is_ai));
    }
    if let Some(is_authorized) = query.is_authorized {
        select = select.filter(image_entity::Column::IsAuthorized.eq(is_authorized));
    }
    if let Some(is_favorite) = query.is_favorite {
        select = select.filter(image_entity::Column::IsFavorite.eq(is_favorite));
    }
    if let Some(search) = &query.search {
        use sea_orm::Condition;
        let search_condition = Condition::any()
            .add(image_entity::Column::Title.contains(search))
            .add(image_entity::Column::Tags.contains(search))
            .add(image_entity::Column::AiPrompt.contains(search));
        select = select.filter(search_condition);
    }

    // 排序：收藏优先，然后按导入时间
    select = select
        .order_by_desc(image_entity::Column::IsFavorite)
        .order_by_desc(image_entity::Column::CreatedAt);

    // 分页
    let paginator = select.paginate(&db, page_size);
    let total = paginator.num_items().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    let total_pages = paginator.num_pages().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    let images = paginator.fetch_page(page - 1).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    let items: Vec<ImageListItem> = images
        .into_iter()
        .map(|img| ImageListItem {
            id: img.id,
            title: img.title,
            thumbnail_path: img.thumbnail_path,
            width: img.width,
            height: img.height,
            is_favorite: img.is_favorite,
            is_ai: img.is_ai,
            is_authorized: img.is_authorized,
            color_category: img.color_category,
            created_at: Utc.from_utc_datetime(&img.created_at).to_rfc3339(),
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

/// GET /api/images/:id - 获取单个图片详情
pub async fn get(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let image = image_entity::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "图片不存在" })),
            )
        })?;

    Ok(Json(ImageResponse::from(image)))
}

/// POST /api/images - 导入图片
pub async fn import(
    State(db): State<DatabaseConnection>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let mut imported_ids: Vec<Uuid> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("解析表单失败: {}", e) })),
        )
    })? {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let _content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        // 检查文件类型
        let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
        if !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
            continue;
        }

        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("读取文件失败: {}", e) })),
            )
        })?;

        let id = Uuid::new_v4();
        let storage_dir = crate::utils::paths::get_data_path("images").join(id.to_string());
        fs::create_dir_all(&storage_dir).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("创建目录失败: {}", e) })),
            )
        })?;

        // 保存原图
        let original_path = storage_dir.join(format!("original.{}", ext));
        fs::write(&original_path, &data).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("保存原图失败: {}", e) })),
            )
        })?;

        // 解析图片
        let img = image::load_from_memory(&data).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("解析图片失败: {}", e) })),
            )
        })?;

        let (width, height) = img.dimensions();
        let file_size = data.len() as i64;

        // 计算主色调
        let color_category = calculate_dominant_color(&img);

        // 提取 AI 元数据
        let (ai_platform, ai_prompt, ai_negative_prompt) = extract_ai_metadata(&data)
            .map(|(p, prompt, neg)| (Some(p), prompt, neg))
            .unwrap_or((None, None, None));
        let is_ai = ai_platform.is_some();

        // 生成缩略图 (非 GIF)
        let thumbnail_path = if ext != "gif" {
            let thumb = img.thumbnail(512, 768);
            let encoder = webp::Encoder::from_image(&thumb).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("WebP 编码失败: {}", e) })),
                )
            })?;
            let webp_data = encoder.encode(85.0).to_vec();
            let thumb_path = storage_dir.join("thumbnail.webp");
            fs::write(&thumb_path, &webp_data).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("保存缩略图失败: {}", e) })),
                )
            })?;
            format!("/images/{}/thumbnail.webp", id)
        } else {
            // GIF 直接使用原图作为缩略图
            format!("/images/{}/original.{}", id, ext)
        };

        // 写入数据库
        let title = file_name
            .rsplit('/')
            .next()
            .unwrap_or(&file_name)
            .rsplit('.')
            .skip(1)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(".");
        let title = if title.is_empty() {
            file_name.clone()
        } else {
            title
        };

        let new_image = image_entity::ActiveModel {
            id: Set(id),
            title: Set(title),
            category_id: Set(None),
            tags: Set("[]".to_string()),
            file_path: Set(format!("/images/{}/original.{}", id, ext)),
            thumbnail_path: Set(thumbnail_path),
            width: Set(width as i32),
            height: Set(height as i32),
            file_size: Set(file_size),
            color_category: Set(Some(color_category)),
            is_ai: Set(is_ai),
            ai_platform: Set(ai_platform),
            ai_prompt: Set(ai_prompt),
            ai_negative_prompt: Set(ai_negative_prompt),
            is_authorized: Set(false),
            is_favorite: Set(false),
            user_notes: Set(None),
            char_cards: Set("[]".to_string()),
            created_at: Set(Utc::now().naive_utc()),
        };

        new_image.insert(&db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("保存到数据库失败: {}", e) })),
            )
        })?;

        imported_ids.push(id);
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({ "imported": imported_ids.len(), "ids": imported_ids })),
    ))
}

/// PATCH /api/images/:id - 更新图片元数据
pub async fn update(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateImageRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let image = image_entity::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "图片不存在" })),
            )
        })?;

    let mut active: image_entity::ActiveModel = image.into();

    if let Some(title) = payload.title {
        active.title = Set(title);
    }
    if let Some(category_id) = payload.category_id {
        active.category_id = Set(Some(category_id));
    }
    if let Some(tags) = payload.tags {
        active.tags = Set(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()));
    }
    if let Some(is_ai) = payload.is_ai {
        active.is_ai = Set(is_ai);
    }
    if let Some(platform) = payload.ai_platform {
        active.ai_platform = Set(Some(platform));
    }
    if let Some(prompt) = payload.ai_prompt {
        active.ai_prompt = Set(Some(prompt));
    }
    if let Some(neg) = payload.ai_negative_prompt {
        active.ai_negative_prompt = Set(Some(neg));
    }
    if let Some(is_authorized) = payload.is_authorized {
        active.is_authorized = Set(is_authorized);
    }
    if let Some(is_favorite) = payload.is_favorite {
        active.is_favorite = Set(is_favorite);
    }
    if let Some(notes) = payload.user_notes {
        active.user_notes = Set(Some(notes));
    }

    let result = active.update(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(ImageResponse::from(result)))
}

/// DELETE /api/images/:id - 删除图片
pub async fn delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let image = image_entity::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "图片不存在" })),
            )
        })?;

    // 删除文件
    let storage_dir = crate::utils::paths::get_data_path("images").join(id.to_string());
    if storage_dir.exists() {
        let _ = fs::remove_dir_all(&storage_dir).await;
    }

    // 删除数据库记录
    image.delete(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/images/batch/delete - 批量删除
pub async fn batch_delete(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    for id in &payload.ids {
        let image = image_entity::Entity::find_by_id(*id).one(&db).await;
        if let Ok(Some(img)) = image {
            // 删除文件
            let storage_dir = crate::utils::paths::get_data_path("images").join(id.to_string());
            if storage_dir.exists() {
                let _ = fs::remove_dir_all(&storage_dir).await;
            }
            let _ = img.delete(&db).await;
        }
    }

    Ok(Json(json!({ "deleted": payload.ids.len() })))
}

/// PUT /api/images/batch/category - 批量移动分类
pub async fn batch_category(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchCategoryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    for id in &payload.ids {
        if let Ok(Some(img)) = image_entity::Entity::find_by_id(*id).one(&db).await {
            let mut active: image_entity::ActiveModel = img.into();
            active.category_id = Set(payload.category_id);
            let _ = active.update(&db).await;
        }
    }

    Ok(Json(json!({ "updated": payload.ids.len() })))
}

/// PATCH /api/images/batch/update - 批量更新
pub async fn batch_update(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchUpdateRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    for id in &payload.ids {
        if let Ok(Some(img)) = image_entity::Entity::find_by_id(*id).one(&db).await {
            let mut active: image_entity::ActiveModel = img.into();
            if let Some(is_ai) = payload.is_ai {
                active.is_ai = Set(is_ai);
            }
            if let Some(is_authorized) = payload.is_authorized {
                active.is_authorized = Set(is_authorized);
            }
            let _ = active.update(&db).await;
        }
    }

    Ok(Json(json!({ "updated": payload.ids.len() })))
}

/// POST /api/images/batch/export - 批量导出
pub async fn batch_export(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    use axum::http::header;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    let mut buf = Vec::new();
    let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
    let options: SimpleFileOptions = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    let mut count = 0;
    for id in &payload.ids {
        if let Ok(Some(img)) = image_entity::Entity::find_by_id(*id).one(&db).await {
            let path =
                crate::utils::paths::get_data_dir().join(img.file_path.trim_start_matches('/'));
            if path.exists() {
                if let Ok(content) = fs::read(&path).await {
                    let file_name = if img.title.trim().is_empty() {
                        // Fallback using uuid or path extraction if title is empty
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown.png")
                            .to_string()
                    } else {
                        img.title.clone()
                    };

                    let mut clean_name = file_name.clone();
                    if !clean_name.to_lowercase().ends_with(".png")
                        && !clean_name.to_lowercase().ends_with(".jpg")
                        && !clean_name.to_lowercase().ends_with(".jpeg")
                        && !clean_name.to_lowercase().ends_with(".webp")
                    {
                        if let Some(ext) = path.extension() {
                            clean_name = format!("{}.{}", clean_name, ext.to_string_lossy());
                        }
                    }

                    let _ = zip.start_file(clean_name, options);
                    let _ = zip.write_all(&content);
                    count += 1;
                }
            }
        }
    }

    let _ = zip.finish().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    if count == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No images found to export" })),
        ));
    }

    let headers = [
        (header::CONTENT_TYPE, "application/zip"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"images_export.zip\"",
        ),
    ];

    Ok((headers, buf))
}

/// GET /api/images/:id/export - 导出原图
pub async fn export(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let image = image_entity::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "图片不存在" })),
            )
        })?;

    // 读取原图
    let file_path =
        crate::utils::paths::get_data_dir().join(image.file_path.trim_start_matches('/'));
    let data = fs::read(&file_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("读取文件失败: {}", e) })),
        )
    })?;

    // 确定 Content-Type
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    let content_type = match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    };

    let encoded_filename = urlencoding::encode(&image.title);
    let disposition = format!(
        "attachment; filename=\"download.{}\"; filename*=UTF-8''{}.{}",
        ext, encoded_filename, ext
    );

    Ok((
        [
            ("Content-Type", content_type.to_string()),
            ("Content-Disposition", disposition),
        ],
        data,
    ))
}
