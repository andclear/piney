//! 图库分类 API
//!
//! 独立于角色库分类，用于图库的分类管理

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

use crate::entities::image_category;

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

/// GET /api/image-categories - 获取所有图库分类
pub async fn list(
    State(db): State<DatabaseConnection>,
) -> Result<Json<Vec<CategoryResponse>>, (StatusCode, String)> {
    let categories = image_category::Entity::find()
        .order_by_asc(image_category::Column::SortOrder)
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

/// POST /api/image-categories - 创建图库分类
pub async fn create(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<Json<CategoryResponse>, (StatusCode, String)> {
    // 获取最大排序值
    let max_order = image_category::Entity::find()
        .order_by_desc(image_category::Column::SortOrder)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(|c| c.sort_order)
        .unwrap_or(0);

    let new_category = image_category::ActiveModel {
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

/// PATCH /api/image-categories/:id - 更新图库分类
pub async fn update(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, (StatusCode, String)> {
    let existing = image_category::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "分类不存在".to_string()))?;

    let mut active: image_category::ActiveModel = existing.into();

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

/// DELETE /api/image-categories/:id - 删除图库分类
pub async fn delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    use crate::entities::image;

    // 将该分类下的图片移到"未分类"
    let images = image::Entity::find()
        .filter(image::Column::CategoryId.eq(id))
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for img in images {
        let mut active: image::ActiveModel = img.into();
        active.category_id = Set(None);
        active
            .update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // 删除分类
    image_category::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/image-categories/reorder - 批量更新排序
pub async fn reorder(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<ReorderRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    for (index, id) in payload.ids.iter().enumerate() {
        let existing = image_category::Entity::find_by_id(*id)
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(cat) = existing {
            let mut active: image_category::ActiveModel = cat.into();
            active.sort_order = Set(index as i32);
            active
                .update(&db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    Ok(StatusCode::OK)
}
