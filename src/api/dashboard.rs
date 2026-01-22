use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use once_cell::sync::Lazy;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::entities::{character_card, setting, world_info};

#[derive(Serialize, FromQueryResult, Clone)]
pub struct SimpleCard {
    pub id: Uuid,
    pub name: String,
    pub avatar: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Clone)]
pub struct LuckyCardInfo {
    pub id: Uuid,
    pub name: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct DashboardStats {
    pub total_characters: u64,
    pub total_world_info: u64,
    pub total_tokens_k: f64,
    pub db_size_mb: f64,
    pub recent_cards: Vec<SimpleCard>,
    pub lucky_card: Option<LuckyCardInfo>,
    pub username: String, // Adding username placeholder
    pub gacha_remaining: i32,
    pub gacha_confirmed: bool,
}

pub static DASHBOARD_CACHE: Lazy<Arc<RwLock<Option<(String, DashboardStats)>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub fn invalidate_cache() {
    if let Ok(mut cache) = DASHBOARD_CACHE.write() {
        *cache = None;
    }
}

pub async fn get_dashboard_stats(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // 0. Check Cache
    // We will still use the cache for heavy stats, but we MUST refresh the gacha status
    // on every request to support manual DB updates or other out-of-band changes.
    let mut stats_to_return: Option<DashboardStats> = None;

    if let Ok(cache) = DASHBOARD_CACHE.read() {
        if let Some((date, cached_stats)) = &*cache {
            if date == &today {
                stats_to_return = Some(cached_stats.clone());
            }
        }
    }

    // Always fetch fresh Gacha status
    let (gacha_remaining, gacha_confirmed) = check_and_reset_gacha(&db).await?;

    if let Some(mut stats) = stats_to_return {
        // Update the cached stats with fresh gacha info
        stats.gacha_remaining = gacha_remaining;
        stats.gacha_confirmed = gacha_confirmed;

        // Return immediately (we don't need to re-cache here strictly,
        // effectively we treat gacha status as dynamic)
        return Ok(Json(stats));
    }

    // --- If no cache, perform heavy queries ---

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

    // Note: We already grabbed gacha status above

    let stats = DashboardStats {
        total_characters,
        total_world_info,
        total_tokens_k: (total_tokens_k * 10.0).round() / 10.0, // Keeping 1 decimal
        db_size_mb: (db_size_mb * 100.0).round() / 100.0,       // Keeping 2 decimals
        recent_cards,
        lucky_card,
        username: "Admin".to_string(), // TODO: Fetch from auth/user logic if available
        gacha_remaining,
        gacha_confirmed,
    };

    // Write Cache
    if let Ok(mut cache) = DASHBOARD_CACHE.write() {
        *cache = Some((today, stats.clone()));
    }

    Ok(Json(stats))
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

// Gacha Structs
#[derive(serde::Deserialize)]
pub struct GachaConfirmRequest {
    pub card_id: Uuid,
}

// Helper to check and reset gacha state
async fn check_and_reset_gacha(
    db: &DatabaseConnection,
) -> Result<(i32, bool), (StatusCode, String)> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Batch fetch all gacha settings
    let settings = setting::Entity::find()
        .filter(setting::Column::Key.is_in(vec![
            "daily_gacha_date",
            "daily_gacha_count",
            "daily_gacha_confirmed",
        ]))
        .all(db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Map to hashmap for easy lookup
    use std::collections::HashMap;
    let settings_map: HashMap<String, String> =
        settings.into_iter().map(|s| (s.key, s.value)).collect();

    let date_value = settings_map.get("daily_gacha_date").map(|s| s.as_str());

    // Check if reset needed
    let mut needs_reset = true;
    if let Some(d) = date_value {
        if d == today {
            needs_reset = false;
        }
    }

    if needs_reset {
        save_setting(db, "daily_gacha_date", &today).await?;
        save_setting(db, "daily_gacha_count", "0").await?;
        save_setting(db, "daily_gacha_confirmed", "false").await?;
        return Ok((3, false));
    }

    let count = settings_map
        .get("daily_gacha_count")
        .map(|s| s.parse::<i32>().unwrap_or(0))
        .unwrap_or(0);

    let confirmed = settings_map
        .get("daily_gacha_confirmed")
        .map(|s| s == "true")
        .unwrap_or(false);

    Ok((3 - count, confirmed))
}

pub async fn start_gacha(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (remaining, confirmed) = check_and_reset_gacha(&db).await?;

    if confirmed {
        return Err((StatusCode::FORBIDDEN, "今天已经逆天改命过啦".to_string()));
    }
    if remaining <= 0 {
        return Err((StatusCode::FORBIDDEN, "今天的抽卡次数已用完".to_string()));
    }

    // Fetch 3 random cards
    let cards = character_card::Entity::find()
        .select_only()
        .columns([
            character_card::Column::Id,
            character_card::Column::Name,
            character_card::Column::Avatar,
            character_card::Column::UpdatedAt,
        ])
        .filter(character_card::Column::DeletedAt.is_null())
        .order_by(
            sea_orm::sea_query::SimpleExpr::Custom("RANDOM()".into()),
            sea_orm::Order::Asc,
        )
        .limit(3)
        .into_model::<SimpleCard>()
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Note: Count is NOW updated in `reveal_gacha` when user flips a card.
    Ok(Json(cards))
}

pub async fn reveal_gacha(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (remaining, confirmed) = check_and_reset_gacha(&db).await?;

    if confirmed {
        return Err((StatusCode::FORBIDDEN, "今天已经逆天改命过啦".to_string()));
    }
    if remaining <= 0 {
        return Err((StatusCode::FORBIDDEN, "今天的抽卡次数已用完".to_string()));
    }

    // Increment count
    let count_setting = setting::Entity::find_by_id("daily_gacha_count".to_string())
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let current_count = count_setting
        .map(|s| s.value.parse::<i32>().unwrap_or(0))
        .unwrap_or(0);
    save_setting(&db, "daily_gacha_count", &(current_count + 1).to_string()).await?;

    invalidate_cache();
    Ok(Json("Success"))
}

pub async fn confirm_gacha(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<GachaConfirmRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Update Lucky Card
    // Verify card exists
    let card = character_card::Entity::find_by_id(payload.card_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Card not found".to_string()))?;

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Force set lucky card
    save_setting(&db, "daily_action_date", &today).await?;
    save_setting(&db, "daily_lucky_id", &card.id.to_string()).await?;

    // 2. Mark as confirmed
    save_setting(&db, "daily_gacha_confirmed", "true").await?;

    invalidate_cache();
    Ok(Json("Success"))
}

async fn save_setting(
    db: &DatabaseConnection,
    key: &str,
    value: &str,
) -> Result<(), (StatusCode, String)> {
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
        let setting = setting::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };
        setting
            .insert(db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(())
}
