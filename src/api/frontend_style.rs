//! 前端样式 API
//!
//! 提供前端样式（皮皮美化工作台）的 CRUD 操作

use crate::entities::frontend_style;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{TimeZone, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

// ==================== 请求/响应结构 ====================

#[derive(Deserialize)]
pub struct CreateStyleRequest {
    pub name: String,
    #[serde(default)]
    pub original_text: String,
    #[serde(default)]
    pub regex_pattern: String,
    #[serde(default)]
    pub html_code: String,
    #[serde(default)]
    pub worldinfo_key: String,
    #[serde(default)]
    pub worldinfo_content: String,
}

#[derive(Deserialize)]
pub struct UpdateStyleRequest {
    pub name: Option<String>,
    pub original_text: Option<String>,
    pub regex_pattern: Option<String>,
    pub html_code: Option<String>,
    pub worldinfo_key: Option<String>,
    pub worldinfo_content: Option<String>,
}

#[derive(Serialize)]
pub struct StyleResponse {
    pub id: Uuid,
    pub name: String,
    pub original_text: String,
    pub regex_pattern: String,
    pub html_code: String,
    pub worldinfo_key: String,
    pub worldinfo_content: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<frontend_style::Model> for StyleResponse {
    fn from(m: frontend_style::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            original_text: m.original_text,
            regex_pattern: m.regex_pattern,
            html_code: m.html_code,
            worldinfo_key: m.worldinfo_key,
            worldinfo_content: m.worldinfo_content,
            created_at: Utc.from_utc_datetime(&m.created_at).to_rfc3339(),
            updated_at: Utc.from_utc_datetime(&m.updated_at).to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct StyleListItem {
    pub id: Uuid,
    pub name: String,
    pub updated_at: String,
}

// ==================== API 端点 ====================

/// GET /api/frontend-styles - 获取样式库列表
pub async fn list_styles(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let styles = frontend_style::Entity::find()
        .order_by_desc(frontend_style::Column::UpdatedAt)
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    let items: Vec<StyleListItem> = styles
        .into_iter()
        .map(|s| StyleListItem {
            id: s.id,
            name: s.name,
            updated_at: Utc.from_utc_datetime(&s.updated_at).to_rfc3339(),
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/frontend-styles/:id - 获取单个样式详情
pub async fn get_style(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let style = frontend_style::Entity::find_by_id(id)
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
                Json(json!({ "error": "样式不存在" })),
            )
        })?;

    Ok(Json(StyleResponse::from(style)))
}

/// POST /api/frontend-styles - 创建新样式
pub async fn create_style(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<CreateStyleRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let now = Utc::now().naive_utc();

    let model = frontend_style::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(payload.name),
        original_text: Set(payload.original_text),
        regex_pattern: Set(payload.regex_pattern),
        html_code: Set(payload.html_code),
        worldinfo_key: Set(payload.worldinfo_key),
        worldinfo_content: Set(payload.worldinfo_content),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let result = model.insert(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    Ok((StatusCode::CREATED, Json(StyleResponse::from(result))))
}

/// PUT /api/frontend-styles/:id - 更新样式
pub async fn update_style(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStyleRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let style = frontend_style::Entity::find_by_id(id)
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
                Json(json!({ "error": "样式不存在" })),
            )
        })?;

    let mut active: frontend_style::ActiveModel = style.into();

    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(original_text) = payload.original_text {
        active.original_text = Set(original_text);
    }
    if let Some(regex_pattern) = payload.regex_pattern {
        active.regex_pattern = Set(regex_pattern);
    }
    if let Some(html_code) = payload.html_code {
        active.html_code = Set(html_code);
    }
    if let Some(worldinfo_key) = payload.worldinfo_key {
        active.worldinfo_key = Set(worldinfo_key);
    }
    if let Some(worldinfo_content) = payload.worldinfo_content {
        active.worldinfo_content = Set(worldinfo_content);
    }

    active.updated_at = Set(Utc::now().naive_utc());

    let result = active.update(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(StyleResponse::from(result)))
}

/// DELETE /api/frontend-styles/:id - 删除样式
pub async fn delete_style(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let result = frontend_style::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    if result.rows_affected == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "样式不存在" })),
        ));
    }

    Ok((StatusCode::OK, Json(json!({ "success": true }))))
}
