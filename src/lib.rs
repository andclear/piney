//! Piney 库入口
//!
//! 导出所有公共模块供外部使用

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod entities;
pub mod models;
pub mod services;
pub mod utils;

use axum::{extract::DefaultBodyLimit, middleware, routing::get, Router};
use config::ConfigState;
use sea_orm::DatabaseConnection;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use utils::mode_detect::RunMode;

/// 创建 Axum 应用实例
pub async fn create_app(db: DatabaseConnection, mode: RunMode, config: ConfigState) -> Router {
    // CORS 配置
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (Auth + Public Settings)
    let public_api = Router::new()
        .nest("/auth", auth::router(config.clone()))
        .route("/settings", get(api::settings::get).with_state(db.clone()));

    // Protected routes
    let protected_api = api::routes(db.clone()).layer(middleware::from_fn_with_state(
        config.clone(),
        utils::auth_middleware::auth,
    ));

    // Combine
    let mut app = Router::new()
        .nest("/api", public_api.merge(protected_api))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)); // 100MB 文件大小限制

    // Serve uploaded files
    app = app.nest_service(
        "/uploads",
        tower_http::services::ServeDir::new("data/uploads"),
    );
    app = app.nest_service("/cards", tower_http::services::ServeDir::new("data/cards"));

    // Server Mode 下托管静态文件
    if mode == RunMode::Server {
        app = app.fallback_service(tower_http::services::ServeDir::new("static"));
    }

    app
}
