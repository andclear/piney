//! 分类 API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::category;

// ============ 请求/响应结构 ============

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct ReorderRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Serialize)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i32,
}

// ============ API 处理器 ============

/// GET /api/categories - 获取所有分类
pub async fn list(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<CategoryResponse>>, (StatusCode, String)> {
    let categories = category::Entity::find()
        .order_by_asc(category::Column::SortOrder)
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response: Vec<CategoryResponse> = categories
        .into_iter()
        .map(|c| CategoryResponse {
            id: c.id,
            name: c.name,
            sort_order: c.sort_order,
        })
        .collect();

    Ok(Json(response))
}

/// POST /api/categories - 创建分类
pub async fn create(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<Json<CategoryResponse>, (StatusCode, String)> {
    // 获取最大排序值
    let max_order = category::Entity::find()
        .order_by_desc(category::Column::SortOrder)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(|c| c.sort_order)
        .unwrap_or(0);

    let new_category = category::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(payload.name.clone()),
        sort_order: Set(max_order + 1),
        created_at: Set(chrono::Utc::now().naive_utc()),
    };

    let result = new_category
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CategoryResponse {
        id: result.id,
        name: result.name,
        sort_order: result.sort_order,
    }))
}

/// PATCH /api/categories/:id - 更新分类
pub async fn update(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, (StatusCode, String)> {
    let existing = category::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "分类不存在".to_string()))?;

    let mut active: category::ActiveModel = existing.into();

    if let Some(name) = payload.name {
        active.name = Set(name);
    }

    let result = active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CategoryResponse {
        id: result.id,
        name: result.name,
        sort_order: result.sort_order,
    }))
}

/// DELETE /api/categories/:id - 删除分类
pub async fn delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    use crate::entities::character_card;

    // 将该分类下的角色卡移到"未分类"
    let cards = character_card::Entity::find()
        .filter(character_card::Column::CategoryId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for card in cards {
        let mut active: character_card::ActiveModel = card.into();
        active.category_id = Set(None);
        active
            .update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // 删除分类
    category::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/categories/reorder - 批量更新排序
pub async fn reorder(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<ReorderRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    for (index, id) in payload.ids.iter().enumerate() {
        let existing = category::Entity::find_by_id(*id)
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(cat) = existing {
            let mut active: category::ActiveModel = cat.into();
            active.sort_order = Set(index as i32);
            active
                .update(&db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    Ok(StatusCode::OK)
}
