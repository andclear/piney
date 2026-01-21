use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::entities::world_info;

#[derive(Deserialize)]
pub struct UpdateWorldInfoSchema {
    pub name: Option<String>,
    pub data: Option<Value>,
}

#[derive(Deserialize)]
pub struct ListWorldInfoQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Serialize)]
pub struct PaginatedWorldInfoResponse {
    pub items: Vec<world_info::Model>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub file_name: String,
    pub status: String, // "success" | "error"
    pub reason: Option<String>,
}

// ... List, Get Details, Update, Delete unchanged ...

// --- Import ---
pub async fn import(
    State(db): State<DatabaseConnection>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut results = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unknown.json").to_string();

        // Read bytes
        let data = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                results.push(ImportResult {
                    file_name,
                    status: "error".to_string(),
                    reason: Some(e.to_string()),
                });
                continue;
            }
        };

        // Parse JSON
        let json_string = match String::from_utf8(data.to_vec()) {
            Ok(s) => s,
            Err(_) => {
                results.push(ImportResult {
                    file_name,
                    status: "error".to_string(),
                    reason: Some("Invalid JSON encoding".to_string()),
                });
                continue;
            }
        };

        let json_data: Value = match serde_json::from_str(&json_string) {
            Ok(v) => v,
            Err(e) => {
                results.push(ImportResult {
                    file_name,
                    status: "error".to_string(),
                    reason: Some(format!("Invalid JSON: {}", e)),
                });
                continue;
            }
        };

        // Save
        let name = std::path::Path::new(&file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported World Info")
            .to_string();

        match save_world_info_to_db(&db, name, json_data).await {
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
                    reason: Some(e),
                });
            }
        }
    }

    Ok(Json(results))
}

async fn save_world_info_to_db(
    db: &DatabaseConnection,
    name: String,
    json: Value,
) -> Result<(), String> {
    let uuid = Uuid::new_v4();

    // 格式化 JSON（保持原始键顺序）
    let pretty_json_str =
        serde_json::to_string_pretty(&json).map_err(|e| format!("格式化 JSON 失败: {}", e))?;

    let active_model = world_info::ActiveModel {
        id: Set(uuid),
        name: Set(name),
        data: Set(pretty_json_str),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    };

    active_model
        .insert(db)
        .await
        .map_err(|e| format!("DB Error: {}", e))?;
    Ok(())
}

// --- List ---
pub async fn list(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ListWorldInfoQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let paginator = world_info::Entity::find()
        .order_by_desc(world_info::Column::UpdatedAt)
        .paginate(&db, page_size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_pages = paginator
        .num_pages()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PaginatedWorldInfoResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}

// --- Get Details ---
pub async fn get_details(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let item = world_info::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "World Info not found".to_string()))?;

    Ok(Json(item))
}

// --- Update ---
pub async fn update(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateWorldInfoSchema>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut item: world_info::ActiveModel = world_info::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "World Info not found".to_string()))?
        .into();

    if let Some(name) = payload.name {
        item.name = Set(name);
    }

    if let Some(data) = payload.data {
        let json_str =
            serde_json::to_string(&data).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        item.data = Set(json_str);
    }

    item.updated_at = Set(chrono::Utc::now().naive_utc());

    let updated = item
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(updated))
}

// --- Delete ---
pub async fn delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    world_info::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
