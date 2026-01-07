//! Piney 后端服务入口
//!
//! 支持两种运行模式：
//! - Server Mode (Docker): 托管静态文件 + RESTful API
//! - App Mode (Tauri): 仅提供 API，前端由 Tauri 加载

use piney::{config::ConfigState, create_app, db, utils::mode_detect::RunMode};
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "piney=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载环境变量
    dotenvy::dotenv().ok();

    // 检测运行模式
    let mode = RunMode::detect();
    info!("运行模式: {:?}", mode);

    // 初始化 Config
    let config_path =
        std::env::var("CONFIG_FILE").unwrap_or_else(|_| "data/config.yml".to_string());
    let config = ConfigState::new(&config_path);
    info!(
        "配置初始化完成 (路径: {}, 已加载: {})",
        config_path,
        config.is_initialized()
    );

    // 初始化数据库
    let db = db::init_database().await?;
    info!("数据库初始化完成");

    // 获取端口配置
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "9696".to_string())
        .parse()
        .unwrap_or(9696);

    // 创建应用
    let app = create_app(db, mode, config).await;

    // 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("服务器启动于 http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
