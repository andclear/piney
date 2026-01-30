//! 数据库迁移库入口

pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_tables;
mod m20240102_000002_create_cards;
mod m20240103_000003_add_card_fields;
mod m20240103_000004_add_deleted_at;
mod m20240103_000005_add_detail_fields;
mod m20250104_000001_create_ai_channels;
mod m20250105_000001_refactor_world_info;
mod m20260108_000001_create_chat_histories;
mod m20260108_000002_add_source_file_to_history;
mod m20260109_000001_add_history_pagination_settings;
mod m20260109_000002_add_regex_scripts_to_history;
mod m20260113_000001_create_character_versions;
mod m20260113_000002_add_data_hash;
mod m20260114_000001_add_token_counts;
mod m20260120_000001_create_doctor_tasks;
mod m20260121_000001_add_source_field;
mod m20260122_000001_add_dashboard_indices;
mod m20260124_000001_create_quick_replies;
mod m20260124_000002_create_theaters;
mod m20260129_000001_create_frontend_styles;
mod m20260130_000001_create_image_categories;
mod m20260130_000002_create_images;

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
            Box::new(m20250105_000001_refactor_world_info::Migration),
            Box::new(m20260108_000001_create_chat_histories::Migration),
            Box::new(m20260108_000002_add_source_file_to_history::Migration),
            Box::new(m20260109_000001_add_history_pagination_settings::Migration),
            Box::new(m20260109_000002_add_regex_scripts_to_history::Migration),
            Box::new(m20260113_000001_create_character_versions::Migration),
            Box::new(m20260113_000002_add_data_hash::Migration),
            Box::new(m20260114_000001_add_token_counts::Migration),
            Box::new(m20260120_000001_create_doctor_tasks::Migration),
            Box::new(m20260121_000001_add_source_field::Migration),
            Box::new(m20260122_000001_add_dashboard_indices::Migration),
            Box::new(m20260124_000001_create_quick_replies::Migration),
            Box::new(m20260124_000002_create_theaters::Migration),
            Box::new(m20260129_000001_create_frontend_styles::Migration),
            Box::new(m20260130_000001_create_image_categories::Migration),
            Box::new(m20260130_000002_create_images::Migration),
        ]
    }
}
