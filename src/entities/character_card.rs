//! `SeaORM` Entity - CharacterCard

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "character_cards")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub avatar: Option<String>,
    pub spec: Option<String>,
    pub spec_version: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub data: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    // 新增字段
    pub category_id: Option<Uuid>,
    #[sea_orm(column_type = "Text")]
    pub tags: String, // JSON 数组格式
    pub rating: f64,
    pub cover_blur: bool,
    pub version: Option<String>,
    pub deleted_at: Option<DateTime>,
    #[sea_orm(column_type = "Text")]
    pub custom_summary: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub user_note: Option<String>,
    pub metadata_modified: bool,
    #[sea_orm(column_type = "Text")]
    pub data_hash: Option<String>,
    // Token 统计
    pub token_count_total: Option<i32>,
    pub token_count_spec: Option<i32>,
    pub token_count_wb: Option<i32>,
    pub token_count_other: Option<i32>,
    // 来源标记：import（导入）| local（本地新建）
    pub source: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
