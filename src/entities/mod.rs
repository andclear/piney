//! 实体模块入口
//!
//! 导出所有 SeaORM 实体定义

pub mod ai_channel;
pub mod category;
pub mod character_card;
pub mod setting;
pub mod world_info;

pub mod prelude {
    pub use super::ai_channel::Entity as AiChannel;
    pub use super::category::Entity as Category;
    pub use super::character_card::Entity as CharacterCard;
    pub use super::setting::Entity as Setting;
    pub use super::world_info::Entity as WorldInfo;
}
