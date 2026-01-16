//! 系统设置 API

use crate::entities::{prelude::*, setting};
use axum::{extract::State, http::StatusCode, Json};
use sea_orm::DatabaseConnection;
use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    /// AI 服务提供商
    pub ai_provider: Option<String>,
    /// AI API 端点
    pub ai_endpoint: Option<String>,
    /// AI API 密钥（返回时隐藏）
    pub ai_api_key_set: bool,
    /// AI 模型名称
    pub ai_model: Option<String>,
    /// 主题模式
    pub theme: String,
    /// 语言
    pub language: String,
    /// 默认视图模式
    pub default_view: String,
    /// 每页显示数量
    pub items_per_page: i32,
    /// 用户头像
    pub avatar: Option<String>,
    /// 全局 AI 默认渠道 ID
    pub ai_config_global: Option<String>,
    /// 全局提示词
    pub global_prompt: Option<String>,
}

/// 获取设置
pub async fn get(State(db): State<DatabaseConnection>) -> Json<Settings> {
    // Load all settings from DB
    let settings_list = Setting::find().all(&db).await.unwrap_or_default();

    // Default values
    let mut s = Settings {
        ai_provider: None,
        ai_endpoint: None,
        ai_api_key_set: false,
        ai_model: None,
        theme: "system".to_string(),
        language: "zh-CN".to_string(),
        default_view: "grid".to_string(),
        items_per_page: 20,
        avatar: None,
        ai_config_global: None,
        global_prompt: None,
    };

    // Apply values from DB
    for setting in settings_list {
        match setting.key.as_str() {
            "ai_provider" => s.ai_provider = Some(setting.value),
            "ai_endpoint" => s.ai_endpoint = Some(setting.value),
            "ai_model" => s.ai_model = Some(setting.value),
            "theme" => s.theme = setting.value,
            "language" => s.language = setting.value,
            "default_view" => s.default_view = setting.value,
            "items_per_page" => s.items_per_page = setting.value.parse().unwrap_or(20),
            "user_avatar" => s.avatar = Some(setting.value),
            "ai_config_global" => s.ai_config_global = Some(setting.value),
            "global_prompt" => s.global_prompt = Some(setting.value),
            _ => {}
        }
    }

    Json(s)
}

/// 更新设置
pub async fn update(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<Value>,
) -> Result<Json<Settings>, StatusCode> {
    if let Value::Object(map) = payload {
        for (k, v) in map {
            let val_str = match v {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => continue,
            };

            let key_db = match k.as_str() {
                "ai_provider" => "ai_provider",
                "ai_endpoint" => "ai_endpoint",
                "ai_model" => "ai_model",
                "theme" => "theme",
                "language" => "language",
                "default_view" => "default_view",
                "items_per_page" => "items_per_page",
                "avatar" => "user_avatar", // Map 'avatar' to 'user_avatar'
                "ai_config_global" => "ai_config_global",
                "global_prompt" => "global_prompt",
                _ => continue,
            };

            // If updating avatar, delete the old file
            if key_db == "user_avatar" {
                if let Ok(Some(old_setting)) =
                    setting::Entity::find_by_id("user_avatar".to_string())
                        .one(&db)
                        .await
                {
                    let old_url = old_setting.value;

                    // Helper to get clean filename
                    let get_clean_filename = |url: &str| -> Option<String> {
                        if url.starts_with("/uploads/") {
                            url.strip_prefix("/uploads/")
                                .map(|s| s.split('?').next().unwrap_or(s).to_string())
                        } else {
                            None
                        }
                    };

                    let old_clean = get_clean_filename(&old_url);
                    let new_clean = get_clean_filename(&val_str);

                    // Only delete if filenames are different (different files)
                    // If same file (e.g. avatar.webp vs avatar.webp?t=2), do NOT delete
                    if let (Some(old_f), Some(new_f)) = (&old_clean, &new_clean) {
                        if old_f != new_f {
                            let path = crate::utils::paths::get_data_path("uploads").join(old_f);
                            if path.exists() {
                                if let Err(e) = std::fs::remove_file(&path) {
                                    tracing::warn!(
                                        "Failed to delete old avatar file {:?}: {}",
                                        path,
                                        e
                                    );
                                } else {
                                    tracing::info!("Deleted old avatar file: {:?}", path);
                                }
                            }
                        }
                    } else if let Some(old_f) = old_clean {
                        // case: new url is not local upload (e.g. empty)
                        if val_str.is_empty() {
                            let path = crate::utils::paths::get_data_path("uploads").join(old_f);
                            if path.exists() {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }

            // Upsert
            let active_model = setting::ActiveModel {
                key: Set(key_db.to_string()),
                value: Set(val_str),
                updated_at: Set(chrono::Local::now().naive_local()),
            };

            setting::Entity::insert(active_model)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column(setting::Column::Key)
                        .update_columns([setting::Column::Value, setting::Column::UpdatedAt])
                        .to_owned(),
                )
                .exec(&db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to save setting {}: {}", key_db, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    // Return updated settings
    Ok(get(State(db)).await)
}
