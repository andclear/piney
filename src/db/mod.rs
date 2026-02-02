//! 数据库模块
//!
//! 管理数据库连接和迁移

pub mod connection;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use sea_orm_migration::MigratorTrait;
use tracing::info;

/// 检测并清理旧版或不完整的迁移记录
///
/// 处理以下情况：
/// 1. 存在旧版迁移记录（非 m000001 开头）- 清空让新脚本运行
/// 2. 存在 m000001 记录但缺少必要的表 - 清空让新脚本重新运行以补全缺失表
async fn auto_upgrade_migrations(db: &DatabaseConnection) -> anyhow::Result<()> {
    // 检查 seaql_migrations 表是否存在
    let migrations_table_exists = db
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name='seaql_migrations';"
                .to_owned(),
        ))
        .await?;

    if migrations_table_exists.is_none() {
        return Ok(()); // 表不存在，是全新数据库，无需清理
    }

    // 检查是否有旧版迁移记录（非 m000001 开头的）
    let old_migrations = db
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version NOT LIKE 'm000001%';".to_owned(),
        ))
        .await?;

    if !old_migrations.is_empty() {
        info!(
            "🔄 检测到 {} 条旧版迁移记录，正在自动升级到 v1.0...",
            old_migrations.len()
        );

        // 清空旧的迁移记录
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "DELETE FROM seaql_migrations;".to_owned(),
        ))
        .await?;

        info!("✅ 旧版迁移记录已清理，将使用新的合并脚本");
        return Ok(());
    }

    // 检查是否存在 m000001 记录但缺少必要的表（不完整的迁移）
    let v1_migration = db
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version LIKE 'm000001%';".to_owned(),
        ))
        .await?;

    if v1_migration.is_some() {
        // 检查 theaters 表是否存在（作为新表的代表）
        let theaters_exists = db
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' AND name='theaters';".to_owned(),
            ))
            .await?;

        if theaters_exists.is_none() {
            info!("🔧 检测到不完整的 v1 迁移（缺少 theaters 表），正在修复...");

            // 清空迁移记录，让新脚本重新运行以创建缺失的表
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "DELETE FROM seaql_migrations;".to_owned(),
            ))
            .await?;

            info!("✅ 迁移记录已清理，新脚本将补全缺失的表");
        }
    }

    Ok(())
}

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
    // Windows 下路径包含反斜杠，会导致 URL 解析错误，必须转换为正斜杠
    let db_path_str = db_path.to_string_lossy().replace('\\', "/");

    // 关键修正：手动创建文件，避免依赖 URL query 的 ?mode=rwc 解析（这在 Windows 下极易出错）
    // 这种方式兼容 Win/Mac/Linux/Android
    if !db_path.exists() {
        info!("数据库文件不存在，预创建空文件: {:?}", db_path);
        std::fs::File::create(&db_path)?;
    }

    // 策略 A: 相对路径 (首选，避开盘符问题)
    let current_dir = std::env::current_dir().unwrap_or_default();
    let relative_url = if let Ok(rel_path) = db_path.strip_prefix(&current_dir) {
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");
        Some(format!("sqlite:./{}", rel_str))
    } else {
        None
    };

    // 策略 B: 绝对路径 (备选，标准 URI)
    let absolute_url = if cfg!(windows) {
        format!("sqlite:///{}", db_path_str) // 3 slashes for Windows
    } else {
        format!("sqlite://{}", db_path_str) // 2 slashes for Unix
    };

    info!("尝试数据库连接策略 A (相对路径): {:?}", relative_url);

    // 执行连接尝试
    let db = if let Some(url) = relative_url {
        match Database::connect(&url).await {
            Ok(conn) => {
                info!("策略 A 连接成功");
                conn
            }
            Err(e) => {
                tracing::warn!(
                    "策略 A 连接失败 ({}), 切换到策略 B (绝对路径): {}",
                    e,
                    absolute_url
                );
                Database::connect(&absolute_url).await?
            }
        }
    } else {
        info!("直接使用策略 B (绝对路径): {}", absolute_url);
        Database::connect(&absolute_url).await?
    };

    // 开启 WAL 模式以提高并发性能，并设置 busy_timeout 防止锁竞争导致 500
    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA journal_mode=WAL;".to_owned(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA busy_timeout=5000;".to_owned(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "PRAGMA foreign_keys = ON;".to_owned(),
    ))
    .await?;

    // 自动升级：检测并清理旧版迁移记录
    auto_upgrade_migrations(&db).await?;

    // 运行迁移
    info!("检查数据库迁移...");
    migration::Migrator::up(&db, None).await?;
    info!("数据库迁移完成");

    Ok(db)
}
