//! API 模块入口
//!
//! 定义所有 RESTful API 路由

pub mod ai;
pub mod backup;
pub mod cards;
pub mod categories;
pub mod dashboard;
pub mod frontend_style;
pub mod history;
pub mod image_categories;
pub mod images;
pub mod quick_reply;
pub mod settings;
pub mod theater;
pub mod upload;
pub mod versions;
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
        // 仪表盘
        .route("/dashboard", get(dashboard::get_dashboard_stats))
        .route("/gacha/draw", post(dashboard::start_gacha))
        .route("/gacha/reveal", post(dashboard::reveal_gacha))
        .route("/gacha/confirm", post(dashboard::confirm_gacha))
        // 上传
        .route("/upload", post(upload::upload_image))
        // 角色卡
        .route("/cards", get(cards::list))
        .route("/cards/import", post(cards::import))
        .route("/cards/debug_import", post(cards::debug_import))
        .route("/cards/create", post(cards::create_card))
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
        .route("/cards/batch/export", post(cards::batch_export_cards))
        // 角色卡版本管理
        .route(
            "/cards/{id}/versions",
            get(versions::list_versions).post(versions::create_version),
        )
        .route(
            "/cards/{id}/versions/{version_id}/restore",
            post(versions::restore_version),
        )
        .route(
            "/cards/{id}/versions/{version_id}",
            delete(versions::delete_version),
        )
        // 聊天记录
        .route(
            "/cards/{id}/history",
            get(history::list_history).post(history::upload_history),
        )
        .route(
            "/cards/{id}/history/{history_id}",
            patch(history::update_history).delete(history::delete_history),
        )
        .route(
            "/cards/{id}/history/{history_id}/content",
            get(history::get_history_content).put(history::update_history_content),
        )
        // 快速回复
        .route(
            "/cards/{id}/quick_reply",
            get(quick_reply::list_quick_replies).post(quick_reply::upload_quick_reply),
        )
        .route(
            "/cards/{id}/quick_reply/{qr_id}",
            patch(quick_reply::update_quick_reply).delete(quick_reply::delete_quick_reply),
        )
        .route(
            "/cards/{id}/quick_reply/{qr_id}/export",
            get(quick_reply::export_quick_reply),
        )
        // 回收站
        .route("/trash/cards", get(cards::list_trash))
        .route("/trash/cards/{id}/restore", post(cards::restore_card))
        .route("/trash/cards/{id}", delete(cards::permanent_delete))
        .route("/trash/cards/batch-delete", post(cards::batch_delete_trash))
        .route("/trash/cards/clear", delete(cards::clear_trash))
        // 分类
        .route("/categories", get(categories::list))
        .route("/categories", post(categories::create))
        .route("/categories/reorder", put(categories::reorder))
        .route(
            "/categories/{id}",
            patch(categories::update).delete(categories::delete),
        )
        // 图库分类
        .route(
            "/image-categories",
            get(image_categories::list).post(image_categories::create),
        )
        .route("/image-categories/reorder", put(image_categories::reorder))
        .route(
            "/image-categories/{id}",
            patch(image_categories::update).delete(image_categories::delete),
        )
        // 图库
        .route("/images", get(images::list).post(images::import))
        .route("/images/batch/delete", post(images::batch_delete))
        .route("/images/batch/category", put(images::batch_category))
        .route("/images/batch/update", patch(images::batch_update))
        .route("/images/batch/export", post(images::batch_export))
        .route(
            "/images/{id}",
            get(images::get)
                .patch(images::update)
                .delete(images::delete),
        )
        .route("/images/{id}/export", get(images::export))
        // 世界书
        .route("/world_info/import", post(world_info::import))
        .route("/world_info", get(world_info::list))
        .route(
            "/world_info/{id}",
            get(world_info::get_details)
                .patch(world_info::update)
                .delete(world_info::delete),
        )
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
        .route("/ai/execute", post(ai::execute_feature))
        // 小皮医生
        .route("/ai/doctor/analyze", post(ai::doctor_analyze))
        .route("/ai/doctor/history/{card_id}", get(ai::doctor_history))
        .route(
            "/ai/doctor/history/item/{id}",
            delete(ai::doctor_history_delete),
        )
        // 数据备份
        .route("/backup/export", get(backup::export_backup))
        .route("/backup/import", post(backup::import_backup))
        // 小剧场
        .route(
            "/theaters",
            get(theater::list_theaters).post(theater::create_theater),
        )
        .route("/theaters/import", post(theater::import_theaters))
        .route("/theaters/export", get(theater::export_theaters))
        .route("/theaters/categories", get(theater::list_categories))
        .route("/theaters/batch", delete(theater::batch_delete_theaters))
        .route(
            "/theaters/{id}",
            get(theater::get_theater)
                .patch(theater::update_theater)
                .delete(theater::delete_theater),
        )
        // 前端样式
        .route(
            "/frontend-styles",
            get(frontend_style::list_styles).post(frontend_style::create_style),
        )
        .route(
            "/frontend-styles/{id}",
            get(frontend_style::get_style)
                .put(frontend_style::update_style)
                .delete(frontend_style::delete_style),
        )
        .with_state(db)
}
