use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

use crate::entities::{character_card, setting, world_info};

#[derive(Serialize, FromQueryResult)]
pub struct SimpleCard {
    pub id: Uuid,
    pub name: String,
    pub avatar: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
pub struct LuckyCardInfo {
    pub id: Uuid,
    pub name: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_characters: u64,
    pub total_world_info: u64,
    pub total_tokens_k: f64,
    pub db_size_mb: f64,
    pub recent_cards: Vec<SimpleCard>,
    pub lucky_card: Option<LuckyCardInfo>,
    pub username: String, // Adding username placeholder
}

pub async fn get_dashboard_stats(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. 基本统计
    let total_characters = character_card::Entity::find()
        .filter(character_card::Column::DeletedAt.is_null())
        .count(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_world_info = world_info::Entity::find()
        .count(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Token 总数
    // 使用 SQL Sum
    #[derive(FromQueryResult)]
    struct TokenSumResult {
        total: Option<i64>,
    }

    let token_sum: Option<TokenSumResult> = character_card::Entity::find()
        .select_only()
        .column_as(character_card::Column::TokenCountTotal.sum(), "total")
        .filter(character_card::Column::DeletedAt.is_null())
        .into_model::<TokenSumResult>()
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_tokens_val = token_sum.and_then(|r| r.total).unwrap_or(0);
    let total_tokens_k = (total_tokens_val as f64) / 1000.0;

    // 2. DB Size
    let db_path = Path::new("./data/piney.db");
    let db_size_mb = if db_path.exists() {
        let size = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
        (size as f64) / (1024.0 * 1024.0)
    } else {
        0.0
    };

    // 3. 最近编辑 (Top 5)
    let recent_cards = character_card::Entity::find()
        .select_only()
        .columns([
            character_card::Column::Id,
            character_card::Column::Name,
            character_card::Column::Avatar,
            character_card::Column::UpdatedAt,
        ])
        .filter(character_card::Column::DeletedAt.is_null())
        .order_by_desc(character_card::Column::UpdatedAt)
        .limit(5)
        .into_model::<SimpleCard>()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 4. 每日好运角色
    let lucky_card = get_daily_lucky_card(&db).await?;

    Ok(Json(DashboardStats {
        total_characters,
        total_world_info,
        total_tokens_k: (total_tokens_k * 10.0).round() / 10.0, // Keeping 1 decimal
        db_size_mb: (db_size_mb * 100.0).round() / 100.0,       // Keeping 2 decimals
        recent_cards,
        lucky_card,
        username: "Admin".to_string(), // TODO: Fetch from auth/user logic if available
    }))
}

async fn get_daily_lucky_card(
    db: &DatabaseConnection,
) -> Result<Option<LuckyCardInfo>, (StatusCode, String)> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // 1. Check Payload
    let date_setting = setting::Entity::find_by_id("daily_action_date".to_string())
        .one(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id_setting = setting::Entity::find_by_id("daily_lucky_id".to_string())
        .one(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut target_id = None;

    if let (Some(d), Some(i)) = (date_setting, id_setting) {
        if d.value == today {
            if let Ok(uuid) = Uuid::parse_str(&i.value) {
                target_id = Some(uuid);
            }
        }
    }

    // Check if target_id exists and is valid
    if let Some(uuid) = target_id {
        let exists = character_card::Entity::find_by_id(uuid)
            .filter(character_card::Column::DeletedAt.is_null())
            .one(db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(card) = exists {
            return Ok(Some(LuckyCardInfo {
                id: card.id,
                name: card.name,
                avatar: card.avatar,
                description: card.description,
            }));
        }
    }

    // Need new random card
    // Note: Simple RANDOM order for SQLite
    let random_card = character_card::Entity::find()
        .filter(character_card::Column::DeletedAt.is_null())
        .order_by(
            sea_orm::sea_query::SimpleExpr::Custom("RANDOM()".into()),
            sea_orm::Order::Asc,
        )
        .one(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(card) = random_card {
        // Update Settings
        save_setting(db, "daily_action_date", &today).await?;
        save_setting(db, "daily_lucky_id", &card.id.to_string()).await?;

        Ok(Some(LuckyCardInfo {
            id: card.id,
            name: card.name,
            avatar: card.avatar,
            description: card.description,
        }))
    } else {
        Ok(None)
    }
}

async fn save_setting(
    db: &DatabaseConnection,
    key: &str,
    value: &str,
) -> Result<(), (StatusCode, String)> {
    let setting = setting::ActiveModel {
        key: Set(key.to_string()),
        value: Set(value.to_string()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    // Upsert equivalent (SeaORM insert with on conflict or simple checks)
    // Since `key` is primary key, we can try insert, if fails update.
    // Or check exists.
    // SeaORM has `save` but usually requires finding first.
    // Let's rely on standard upsert if possible or simple find-then-update.

    let exists = setting::Entity::find_by_id(key.to_string())
        .one(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(_existing) = exists {
        let mut active: setting::ActiveModel = _existing.into();
        active.value = Set(value.to_string());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active
            .update(db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        setting
            .insert(db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(())
}
