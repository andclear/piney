use crate::entities::{ai_channel, character_card, setting};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    #[serde(default = "default_active")]
    pub is_active: bool,
}

fn default_active() -> bool {
    true
}

#[derive(Serialize)]
pub struct ChannelResponse {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub model_id: String,
    pub is_active: bool,
    // Sensitive data excluded
}

#[derive(Deserialize)]
pub struct TestConnectionRequest {
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
}

#[derive(Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model_id: Option<String>,
    pub is_active: Option<bool>,
}

/// GET /api/ai/channels - List all channels
pub async fn list_channels(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let channels = ai_channel::Entity::find()
        .order_by_desc(ai_channel::Column::CreatedAt)
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let res: Vec<ChannelResponse> = channels
        .into_iter()
        .map(|c| ChannelResponse {
            id: c.id,
            name: c.name,
            base_url: c.base_url,
            model_id: c.model_id,
            is_active: c.is_active,
        })
        .collect();

    Ok(Json(res))
}

/// POST /api/ai/channels - Create channel
pub async fn create_channel(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    // Generate UUID upfront to avoid last_insert_id issues with SQLite
    let channel_id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();

    let new_channel = ai_channel::ActiveModel {
        id: Set(channel_id),
        name: Set(payload.name.clone()),
        base_url: Set(payload.base_url.clone()),
        api_key: Set(payload.api_key),
        model_id: Set(payload.model_id.clone()),
        is_active: Set(payload.is_active),
        created_at: Set(now),
        updated_at: Set(now),
    };

    // Use insert without relying on return value (SQLite + UUID fix)
    ai_channel::Entity::insert(new_channel)
        .exec_without_returning(&db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create channel: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    // Return the response using the data we already have
    Ok(Json(ChannelResponse {
        id: channel_id,
        name: payload.name,
        base_url: payload.base_url,
        model_id: payload.model_id,
        is_active: payload.is_active,
    }))
}

/// DELETE /api/ai/channels/:id - Delete channel
pub async fn delete_channel(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    ai_channel::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    Ok((StatusCode::OK, Json(serde_json::json!({}))))
}

/// PUT /api/ai/channels/:id - Update channel
pub async fn update_channel(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateChannelRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    // Find existing channel
    let existing = ai_channel::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Channel not found"})),
            )
        })?;

    // Build update model
    let mut update_model: ai_channel::ActiveModel = existing.into();

    if let Some(name) = payload.name {
        update_model.name = Set(name);
    }
    if let Some(base_url) = payload.base_url {
        update_model.base_url = Set(base_url);
    }
    if let Some(api_key) = payload.api_key {
        update_model.api_key = Set(api_key);
    }
    if let Some(model_id) = payload.model_id {
        update_model.model_id = Set(model_id);
    }
    if let Some(is_active) = payload.is_active {
        update_model.is_active = Set(is_active);
    }
    update_model.updated_at = Set(chrono::Utc::now().naive_utc());

    let updated = update_model.update(&db).await.map_err(|e| {
        tracing::error!("Failed to update channel: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(ChannelResponse {
        id: updated.id,
        name: updated.name,
        base_url: updated.base_url,
        model_id: updated.model_id,
        is_active: updated.is_active,
    }))
}
pub async fn test_connection(
    Json(payload): Json<TestConnectionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();

    // Construct Chat Completion request
    // URL: base_url + /chat/completions (user provides full path including /v1)
    let base = payload.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    let body = serde_json::json!({
        "model": payload.model_id,
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "max_tokens": 5
    });

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", payload.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Request failed: {}", e)})),
            )
        })?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        tracing::error!("AI Connection Test Failed: {}", err_text);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("API Error: {}", err_text)})),
        ));
    }

    // Parse response to ensure it's valid JSON
    let _json: Value = res.json().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid JSON response: {}", e)})),
        )
    })?;

    let latency_ms = start_time.elapsed().as_millis() as u64;
    Ok(Json(serde_json::json!({
        "success": true,
        "latency_ms": latency_ms
    })))
}

/// GET /api/ai/models - List Models (Proxy)
/// Query params: base_url, api_key (Transient, not saved)
#[derive(Deserialize)]
pub struct ListModelsQuery {
    pub base_url: String,
    pub api_key: String,
}

pub async fn list_models_proxy(
    axum::extract::Query(query): axum::extract::Query<ListModelsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let client = reqwest::Client::new();
    let base = query.base_url.trim_end_matches('/');
    let url = format!("{}/models", base);

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", query.api_key))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Request failed: {}", e)})),
            )
        })?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("API Error: {}", err_text)})),
        ));
    }

    let json: Value = res.json().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid JSON response: {}", e)})),
        )
    })?;

    Ok(Json(json))
}

#[derive(Serialize)]
pub struct ChannelTestResult {
    pub id: Uuid,
    pub name: String,
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

/// POST /api/ai/channels/test - Test all saved channels
pub async fn test_saved_channels(
    State(db): State<DatabaseConnection>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let channels = ai_channel::Entity::find()
        .filter(ai_channel::Column::IsActive.eq(true))
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let mut results = Vec::new();
    let client = reqwest::Client::new();

    // Parallel testing could be better, but sequential is safer for rate limits
    // and simplicity for now.
    for channel in channels {
        let base = channel.base_url.trim_end_matches('/');
        let url = format!("{}/chat/completions", base);

        let body = serde_json::json!({
            "model": channel.model_id,
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 5
        });

        let start_time = std::time::Instant::now();
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", channel.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    results.push(ChannelTestResult {
                        id: channel.id,
                        name: channel.name,
                        success: true,
                        message: "OK".to_string(),
                        latency_ms: Some(latency_ms),
                    });
                } else {
                    let err_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    results.push(ChannelTestResult {
                        id: channel.id,
                        name: channel.name,
                        success: false,
                        message: err_text,
                        latency_ms: Some(latency_ms),
                    });
                }
            }
            Err(e) => {
                results.push(ChannelTestResult {
                    id: channel.id,
                    name: channel.name,
                    success: false,
                    message: e.to_string(),
                    latency_ms: None,
                });
            }
        }
    }

    Ok(Json(results))
}

#[derive(Deserialize)]
pub struct GenerateOverviewRequest {
    pub card_id: Uuid,
}

#[derive(Serialize)]
pub struct OverviewResponse {
    pub summary: String,
    pub tags: Option<Vec<String>>,
    pub logs: Vec<String>,
}

#[derive(Deserialize)]
struct AiOverviewJson {
    summary: String,
    tags: Option<Vec<String>>,
}

/// POST /api/ai/card/overview - Generate overview for a card
pub async fn generate_overview(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<GenerateOverviewRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let mut logs: Vec<String> = Vec::new();
    logs.push("开始处理生成概览请求...".to_string());

    // 1. 获取 AI 配置
    logs.push("正在获取 全局 AI 配置 (ai_config_global)...".to_string());
    let config_setting = setting::Entity::find_by_id("ai_config_global")
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let channel_id_str = match config_setting {
        Some(s) => s.value,
        None => {
            let msg = "未配置 全局 AI 渠道 (ai_config_global)。请在系统设置中指定默认模型。";
            logs.push(format!("错误: {}", msg));
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": msg, "logs": logs})),
            ));
        }
    };

    let channel_id = Uuid::parse_str(&channel_id_str).map_err(|_| {
        let msg = "AI 配置 ID 格式无效";
        logs.push(format!("错误: {}", msg));
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": msg, "logs": logs})),
        )
    })?;

    let channel = ai_channel::Entity::find_by_id(channel_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            let msg = "配置的 AI 渠道不存在";
            logs.push(format!("错误: {}", msg));
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": msg, "logs": logs})),
            )
        })?;
    logs.push(format!(
        "使用渠道: {} (Model: {})",
        channel.name, channel.model_id
    ));

    // 1.5. 获取破限提示词
    let global_prompt_setting = setting::Entity::find_by_id("global_prompt")
        .one(&db)
        .await
        .unwrap_or(None);
    let global_prompt = global_prompt_setting.map(|s| s.value).unwrap_or_default();
    if !global_prompt.is_empty() {
        logs.push(format!("破限提示词已加载 ({} 字符)", global_prompt.len()));
    } else {
        logs.push("未配置破限提示词".to_string());
    }

    // 2. 获取角色卡数据
    logs.push(format!("正在获取角色卡: {}", payload.card_id));
    let card = character_card::Entity::find_by_id(payload.card_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            let msg = "角色卡不存在";
            logs.push(format!("错误: {}", msg));
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": msg, "logs": logs})),
            )
        })?;

    // 解析 JSON data
    let card_data: Value = serde_json::from_str(&card.data).unwrap_or(serde_json::json!({}));

    // 提取字段
    let name = card.name.clone();
    let description = card.description.clone().unwrap_or_default();
    let personality = card_data["personality"].as_str().unwrap_or("").to_string();
    let scenario = card_data["scenario"].as_str().unwrap_or("").to_string();
    let first_mes = card_data["first_mes"].as_str().unwrap_or("").to_string();
    let mes_example = card_data["mes_example"].as_str().unwrap_or("").to_string();
    let creatorcomment = card_data["creatorcomment"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let system_prompt = card_data["system_prompt"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let post_history_instructions = card_data["post_history_instructions"]
        .as_str()
        .unwrap_or("")
        .to_string();

    logs.push("字段提取完成:".to_string());
    logs.push(format!("- Name: {}", name));
    logs.push(format!("- Description length: {}", description.len()));
    logs.push(format!("- Personality length: {}", personality.len()));
    logs.push(format!("- Scenario length: {}", scenario.len()));
    logs.push(format!("- First Mes length: {}", first_mes.len()));

    // 3. 检查标签策略
    let current_tags_json: Vec<String> = serde_json::from_str(&card.tags).unwrap_or_default();
    let generate_tags = current_tags_json.is_empty();

    let mut system_tags_str = String::new();
    if generate_tags {
        logs.push("当前无标签，将生成标签。正在获取系统标签库...".to_string());
        // 获取所有 tags
        let all_cards = character_card::Entity::find()
            .all(&db)
            .await
            .unwrap_or_default();
        let mut all_tags = std::collections::HashSet::new();
        for c in all_cards {
            if let Ok(tags) = serde_json::from_str::<Vec<String>>(&c.tags) {
                for t in tags {
                    all_tags.insert(t);
                }
            }
        }
        let tags_vec: Vec<String> = all_tags.into_iter().collect();
        system_tags_str = serde_json::to_string(&tags_vec).unwrap_or_default();
        logs.push(format!("系统标签库共 {} 个标签", tags_vec.len()));
    } else {
        logs.push("当前已有标签，跳过标签生成。".to_string());
    }

    // 4. 构建 Prompt
    let task_instruction = if generate_tags {
        format!(
            r#"
[任务与约束]
1. 概览总结：150字以内，精炼概括角色核心特征。
2. 标签生成：最多5个。必须优先从以下[系统现有标签]中选择；仅当无匹配时才生成新标签。
   [系统现有标签]: {}

[回复格式]
请严格仅返回 JSON，不要使用代码块：
{{"summary": "...", "tags": ["tag1", "tag2"]}}
"#,
            system_tags_str
        )
    } else {
        r#"
[任务与约束]
1. 概览总结：150字以内，精炼概括角色核心特征。

[回复格式]
请严格仅返回 JSON，不要使用代码块：
{"summary": "..."}
"#
        .to_string()
    };

    let user_content = format!(
        r#"请深入分析以下角色卡数据：

[角色元数据]
Name: {}
Description: {}

[详细设定]
Personality: {}
Scenario: {}
First Message: {}
Example Dialogue: {}
System Prompt: {}
Post Instructions: {}
Creator Comment: {}

{}"#,
        name,
        description,
        personality,
        scenario,
        first_mes,
        mes_example,
        system_prompt,
        post_history_instructions,
        creatorcomment,
        task_instruction
    );

    logs.push("Prompt 构建完成".to_string());
    // logs.push(format!("User Content:\n{}", user_content)); // 若太长可注释

    // 5. 调用 AI
    let client = reqwest::Client::new();
    let base = channel.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    // 构建系统提示词：破限提示词 + 功能提示词
    let base_system_prompt = "你是一位专业的角色卡分析师。请分析角色设定，返回纯 JSON 格式结果，不要包含 markdown 标记。";
    let system_prompt_content = if global_prompt.is_empty() {
        base_system_prompt.to_string()
    } else {
        format!("{}\n\n{}", global_prompt, base_system_prompt)
    };
    logs.push(format!(
        "System Prompt 长度: {} 字符",
        system_prompt_content.len()
    ));

    let body = serde_json::json!({
        "model": channel.model_id,
        "messages": [
            {
                "role": "system",
                "content": system_prompt_content
            },
            {"role": "user", "content": user_content}
        ],
        "temperature": 0.7
    });

    logs.push(format!("正在请求 AI 接口: {}", url));
    let start_time = std::time::Instant::now();

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", channel.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("请求失败: {}", e);
            logs.push(msg.clone());
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": msg, "logs": logs})),
            )
        })?;

    logs.push(format!("AI 响应状态: {}", res.status()));

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        let msg = format!("API 错误: {}", err_text);
        logs.push(msg.clone());
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": msg, "logs": logs})),
        ));
    }

    let json_res: Value = res.json().await.map_err(|e| {
        let msg = format!("无效的 JSON 响应: {}", e);
        logs.push(msg.clone());
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": msg, "logs": logs})),
        )
    })?;

    let latency = start_time.elapsed().as_millis();
    logs.push(format!("请求耗时: {}ms", latency));

    // 记录完整的 AI 响应结构（用于调试）
    logs.push(format!(
        "Raw JSON Response: {}",
        serde_json::to_string(&json_res).unwrap_or_default()
    ));

    // 提取 content
    let content = json_res["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| {
            let msg = "AI 响应无 content 字段".to_string();
            logs.push(msg.clone());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": msg, "logs": logs})),
            )
        })?;

    logs.push(format!("Raw Content: {}", content));

    // 检查空内容（可能是安全过滤导致）
    if content.trim().is_empty() {
        let completion_tokens = json_res["usage"]["completion_tokens"].as_u64().unwrap_or(0);
        let msg = if completion_tokens == 0 {
            "AI 返回空内容 (completion_tokens=0)。可能是模型安全过滤触发，请尝试更换渠道/模型。"
                .to_string()
        } else {
            "AI 返回空内容".to_string()
        };
        logs.push(msg.clone());
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": msg, "logs": logs})),
        ));
    }

    // 清理 markdown code block if present
    let cleaned_content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 解析结果 JSON
    let ai_result: AiOverviewJson = serde_json::from_str(cleaned_content).map_err(|e| {
        let msg = format!("无法解析 AI 返回的 JSON: {}", e);
        logs.push(msg.clone());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": msg, "logs": logs})),
        )
    })?;

    // 6. 更新数据库
    let mut update_model: character_card::ActiveModel = card.clone().into();
    update_model.custom_summary = Set(Some(ai_result.summary.clone()));

    let mut final_tags = None;
    if generate_tags {
        if let Some(ref tags) = ai_result.tags {
            logs.push(format!("生成了 {} 个标签: {:?}", tags.len(), tags));
            let tags_json = serde_json::to_string_pretty(&tags).unwrap_or("[]".to_string());
            update_model.tags = Set(tags_json);

            // 更新 data JSON 中的 tags（V1/V2/V3 兼容）
            let mut current_json: Value =
                serde_json::from_str(&card.data).unwrap_or(serde_json::json!({}));

            // V2 spec: data.data.tags
            if let Some(data) = current_json.get_mut("data") {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("tags".to_string(), serde_json::json!(tags));
                }
            }

            // V1/V3: 根级 tags
            if let Some(obj) = current_json.as_object_mut() {
                obj.insert("tags".to_string(), serde_json::json!(tags));
            }

            // 写回 data 字段（保持格式化）
            update_model.data =
                Set(serde_json::to_string_pretty(&current_json).unwrap_or(card.data.clone()));
            logs.push("已同步更新 data JSON 中的 tags 字段".to_string());

            final_tags = Some(tags.clone());
        }
    } else {
        logs.push(format!("仅更新概览: {}", ai_result.summary));
    }

    update_model.metadata_modified = Set(true);
    update_model.updated_at = Set(chrono::Utc::now().naive_utc());

    update_model.update(&db).await.map_err(|e| {
        let msg = format!("数据库更新失败: {}", e);
        logs.push(msg.clone());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": msg, "logs": logs})),
        )
    })?;

    logs.push("处理完成!".to_string());

    Ok(Json(OverviewResponse {
        summary: ai_result.summary,
        tags: final_tags,
        logs,
    }))
}

#[derive(Deserialize)]
pub struct ExecuteFeatureRequest {
    pub feature_id: String,               // e.g. "overview"
    pub messages: Vec<serde_json::Value>, // [{"role": "user", "content": "..."}]
}

/// POST /api/ai/execute - Execute generic AI task based on feature config
pub async fn execute_feature(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<ExecuteFeatureRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    // 1. Resolve Config Key
    let config_key = "ai_config_global";

    // 2. Get Channel Config
    let config_setting = setting::Entity::find_by_id(config_key)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let channel_id_str = match config_setting {
        Some(s) => s.value,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "没有配置全局AI模型，请到设置页面完成配置"})),
            ));
        }
    };

    let channel_id = Uuid::parse_str(&channel_id_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "没有配置全局AI模型，请到设置页面完成配置"})),
        )
    })?;

    let channel = ai_channel::Entity::find_by_id(channel_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "配置的AI渠道已不存在，请重新配置"})),
            )
        })?;

    // 3. Proxy Request
    let client = reqwest::Client::new();
    let base = channel.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    let body = serde_json::json!({
        "model": channel.model_id,
        "messages": payload.messages,
        "temperature": 0.7
    });

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", channel.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Request failed: {}", e)})),
            )
        })?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Provider API Error: {}", err_text)})),
        ));
    }

    let json: Value = res.json().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid JSON response: {}", e)})),
        )
    })?;

    Ok(Json(json))
}
