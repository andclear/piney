//! 实体模块入口
//!
//! 导出所有 SeaORM 实体定义

pub mod ai_channel;
pub mod category;
pub mod character_card;
pub mod character_versions;
pub mod chat_history;
pub mod doctor_task;
pub mod frontend_style;
pub mod image;
pub mod image_category;
pub mod quick_reply;
pub mod setting;
pub mod theater;
pub mod world_info;

pub mod prelude {
    pub use super::ai_channel::Entity as AiChannel;
    pub use super::category::Entity as Category;
    pub use super::character_card::Entity as CharacterCard;
    pub use super::character_versions::Entity as CharacterVersion;
    pub use super::chat_history::Entity as ChatHistory;
    pub use super::doctor_task::Entity as DoctorTask;
    pub use super::frontend_style::Entity as FrontendStyle;
    pub use super::image::Entity as Image;
    pub use super::image_category::Entity as ImageCategory;
    pub use super::quick_reply::Entity as QuickReply;
    pub use super::setting::Entity as Setting;
    pub use super::theater::Entity as Theater;
    pub use super::world_info::Entity as WorldInfo;
}
