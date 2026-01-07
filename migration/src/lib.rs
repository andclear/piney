//! 数据库迁移库入口

pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_tables;
mod m20240102_000002_create_cards;
mod m20240103_000003_add_card_fields;
mod m20240103_000004_add_deleted_at;

mod m20240103_000005_add_detail_fields;
mod m20250104_000001_create_ai_channels;
mod m20250104_000002_add_default_global_prompt;
mod m20250105_000001_refactor_world_info;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_tables::Migration),
            Box::new(m20240102_000002_create_cards::Migration),
            Box::new(m20240103_000003_add_card_fields::Migration),
            Box::new(m20240103_000004_add_deleted_at::Migration),
            Box::new(m20240103_000005_add_detail_fields::Migration),
            Box::new(m20250104_000001_create_ai_channels::Migration),
            Box::new(m20250104_000002_add_default_global_prompt::Migration),
            Box::new(m20250105_000001_refactor_world_info::Migration),
        ]
    }
}
