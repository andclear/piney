//! 快速回复 API 模块
//!
//! 提供快速回复文件的上传、查询、更新、删除和导出功能

use crate::entities::{prelude::*, quick_reply};
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use tokio::fs;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

/// 快速回复 DTO
#[derive(Serialize)]
pub struct QuickReplyDto {
    pub id: Uuid,
    pub card_id: Uuid,
    pub file_name: String,
    pub display_name: String,
    pub file_size: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<quick_reply::Model> for QuickReplyDto {
    fn from(model: quick_reply::Model) -> Self {
        Self {
            id: model.id,
            card_id: model.card_id,
            file_name: model.file_name,
            display_name: model.display_name,
            file_size: model.file_size,
            created_at: model.created_at.and_utc().to_rfc3339(),
            updated_at: model.updated_at.and_utc().to_rfc3339(),
        }
    }
}

/// 获取角色卡的所有快速回复
pub async fn list_quick_replies(
    State(db): State<DatabaseConnection>,
    Path(card_id): Path<Uuid>,
) -> Result<Json<Vec<QuickReplyDto>>, (StatusCode, String)> {
    let quick_replies = QuickReply::find()
        .filter(quick_reply::Column::CardId.eq(card_id))
        .order_by_desc(quick_reply::Column::CreatedAt)
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let dtos: Vec<QuickReplyDto> = quick_replies.into_iter().map(QuickReplyDto::from).collect();
    Ok(Json(dtos))
}

/// 上传快速回复文件
pub async fn upload_quick_reply(
    State(db): State<DatabaseConnection>,
    Path(card_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<QuickReplyDto>, (StatusCode, String)> {
    let card_dir = crate::utils::paths::get_data_path("cards").join(card_id.to_string());

    // 检查角色卡目录是否存在
    if !card_dir.exists() {
        return Err((StatusCode::NOT_FOUND, "角色卡目录不存在".to_string()));
    }

    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown.json").to_string();

            // 验证文件扩展名
            if !file_name.to_lowercase().ends_with(".json") {
                return Err((StatusCode::BAD_REQUEST, "仅支持 JSON 格式文件".to_string()));
            }

            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            file_data = Some((file_name, data.to_vec()));
        }
    }

    let (file_name, data) = file_data.ok_or((StatusCode::BAD_REQUEST, "缺少文件".to_string()))?;

    let file_size = data.len() as i64;

    // 生成唯一文件名（避免冲突）
    let mut save_name = format!("qr_{}", file_name);
    let mut counter = 1;
    while card_dir.join(&save_name).exists() {
        let stem = std::path::Path::new(&file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        save_name = format!("qr_{}_{}.json", stem, counter);
        counter += 1;
    }

    // 保存文件
    let file_path = card_dir.join(&save_name);
    fs::write(&file_path, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 显示名称（去掉 qr_ 前缀和 .json 后缀）
    let display_name = file_name
        .strip_suffix(".json")
        .unwrap_or(&file_name)
        .to_string();

    let now = Utc::now().naive_utc();
    let quick_reply = quick_reply::ActiveModel {
        id: Set(Uuid::new_v4()),
        card_id: Set(card_id),
        file_name: Set(save_name),
        display_name: Set(display_name),
        file_size: Set(file_size),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = quick_reply
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(QuickReplyDto::from(saved)))
}

/// 更新快速回复请求
#[derive(Deserialize)]
pub struct UpdateQuickReplyReq {
    pub display_name: Option<String>,
}

/// 更新快速回复信息
pub async fn update_quick_reply(
    State(db): State<DatabaseConnection>,
    Path((_card_id, qr_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateQuickReplyReq>,
) -> Result<Json<QuickReplyDto>, (StatusCode, String)> {
    let qr = QuickReply::find_by_id(qr_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "快速回复不存在".to_string()))?;

    let mut active: quick_reply::ActiveModel = qr.into();

    if let Some(name) = payload.display_name {
        active.display_name = Set(name);
    }
    active.updated_at = Set(Utc::now().naive_utc());

    let updated = active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(QuickReplyDto::from(updated)))
}

/// 删除快速回复
pub async fn delete_quick_reply(
    State(db): State<DatabaseConnection>,
    Path((card_id, qr_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let qr = QuickReply::find_by_id(qr_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "快速回复不存在".to_string()))?;

    // 删除文件
    let file_path = crate::utils::paths::get_data_path("cards")
        .join(card_id.to_string())
        .join(&qr.file_name);

    if file_path.exists() {
        fs::remove_file(file_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // 删除数据库记录
    qr.delete(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// 导出快速回复文件
pub async fn export_quick_reply(
    State(db): State<DatabaseConnection>,
    Path((card_id, qr_id)): Path<(Uuid, Uuid)>,
) -> Result<Body, (StatusCode, String)> {
    let qr = QuickReply::find_by_id(qr_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "快速回复不存在".to_string()))?;

    let file_path = crate::utils::paths::get_data_path("cards")
        .join(card_id.to_string())
        .join(&qr.file_name);

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "文件不存在".to_string()));
    }

    let file = fs::File::open(file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let stream = ReaderStream::new(file);

    Ok(Body::from_stream(stream))
}
