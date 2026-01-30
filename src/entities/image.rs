//! `SeaORM` Entity - 图库图片

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "image")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub title: String,
    pub category_id: Option<Uuid>,
    #[sea_orm(column_type = "Text")]
    pub tags: String,
    #[sea_orm(column_type = "Text")]
    pub file_path: String,
    #[sea_orm(column_type = "Text")]
    pub thumbnail_path: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub color_category: Option<String>,
    pub is_ai: bool,
    pub ai_platform: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub ai_prompt: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub ai_negative_prompt: Option<String>,
    pub is_authorized: bool,
    pub is_favorite: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub user_notes: Option<String>,
    /// 关联的角色卡 ID 列表 (JSON 数组)
    #[sea_orm(column_type = "Text")]
    pub char_cards: String,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::image_category::Entity",
        from = "Column::CategoryId",
        to = "super::image_category::Column::Id"
    )]
    Category,
}

impl Related<super::image_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Category.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
