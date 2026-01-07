//! API 模块入口
//!
//! 定义所有 RESTful API 路由

pub mod ai;
pub mod cards;
pub mod categories;
pub mod settings;
pub mod upload;
pub mod world_info;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use sea_orm::DatabaseConnection;

/// 创建 API 路由 (Protected)
pub fn routes(db: DatabaseConnection) -> Router {
    Router::new()
        // 设置
        .route("/settings", patch(settings::update))
        // 上传
        .route("/upload", post(upload::upload_image))
        // 角色卡
        .route("/cards", get(cards::list))
        .route("/cards/import", post(cards::import))
        .route("/cards/debug_import", post(cards::debug_import))
        .route(
            "/cards/{id}",
            get(cards::get_details)
                .patch(cards::update)
                .delete(cards::soft_delete),
        )
        .route("/cards/{id}/cover", post(cards::update_cover))
        .route("/cards/{id}/export", get(cards::export_card))
        .route("/cards/batch/category", put(cards::batch_update_category))
        .route("/cards/batch/delete", post(cards::batch_soft_delete))
        // 回收站
        .route("/trash/cards", get(cards::list_trash))
        .route("/trash/cards/{id}/restore", post(cards::restore_card))
        .route("/trash/cards/{id}", delete(cards::permanent_delete))
        // 分类
        .route("/categories", get(categories::list))
        .route("/categories", post(categories::create))
        .route("/categories/reorder", put(categories::reorder))
        .route(
            "/categories/{id}",
            patch(categories::update).delete(categories::delete),
        )
        // 世界书
        .route("/world_info/import", post(world_info::import))
        .route("/world_info", get(world_info::list))
        .route(
            "/world_info/{id}",
            get(world_info::get_details)
                .patch(world_info::update)
                .delete(world_info::delete),
        )
        // 健康检查
        .route("/health", get(|| async { "OK" }))
        // AI
        .route(
            "/ai/channels",
            get(ai::list_channels).post(ai::create_channel),
        )
        .route("/ai/channels/test", post(ai::test_saved_channels)) // Batch test
        .route(
            "/ai/channels/{id}",
            delete(ai::delete_channel).put(ai::update_channel),
        )
        .route("/ai/test", post(ai::test_connection))
        .route("/ai/models", get(ai::list_models_proxy))
        .route("/ai/card/overview", post(ai::generate_overview))
        .with_state(db)
}
