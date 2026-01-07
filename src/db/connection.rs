//! 数据库连接管理
//!
//! 提供连接池和事务支持

use sea_orm::DatabaseConnection;

/// 数据库连接状态
pub struct DbState {
    pub conn: DatabaseConnection,
}

impl DbState {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }
}
