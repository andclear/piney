//! 初始数据库表创建迁移
//!
//! 只保留 Settings 表。其他表将在后续功能开发时增量添加。

use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 系统设置表
        manager
            .create_table(
                Table::create()
                    .table(Settings::Table)
                    .if_not_exists()
                    .col(string(Settings::Key).primary_key())
                    .col(text(Settings::Value).not_null())
                    .col(timestamp(Settings::UpdatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除系统设置表
        manager
            .drop_table(Table::drop().table(Settings::Table).to_owned())
            .await?;

        Ok(())
    }
}

// 表定义
#[derive(DeriveIden)]
enum Settings {
    Table,
    Key,
    Value,
    UpdatedAt,
}
