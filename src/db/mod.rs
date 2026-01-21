//! 数据库模块
//!
//! 管理数据库连接和迁移

pub mod connection;

use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tracing::info;

/// 初始化数据库连接
pub async fn init_database() -> anyhow::Result<DatabaseConnection> {
    // 获取数据目录
    let data_path = crate::utils::paths::get_data_dir();

    // 确保数据目录存在
    if !data_path.exists() {
        std::fs::create_dir_all(&data_path)?;
        info!("创建数据目录: {:?}", data_path);
    }

    // 确保子目录存在
    // Optimization: Only create directories that are actually used
    for subdir in ["cards", "uploads"] {
        let subdir_path = data_path.join(subdir);
        if !subdir_path.exists() {
            std::fs::create_dir_all(&subdir_path)?;
        }
    }

    // 数据库文件路径
    let db_path = data_path.join("piney.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    info!("连接数据库: {}", db_url);

    // 连接数据库
    let db = Database::connect(&db_url).await?;

    // 运行迁移
    info!("检查数据库迁移...");
    migration::Migrator::up(&db, None).await?;
    info!("数据库迁移完成");

    Ok(db)
}
