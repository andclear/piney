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

    // 1.5. 获取全局提示词
    let global_prompt_setting = setting::Entity::find_by_id("global_prompt")
        .one(&db)
        .await
        .unwrap_or(None);
    let global_prompt = global_prompt_setting.map(|s| s.value).unwrap_or_default();
    if !global_prompt.is_empty() {
        logs.push(format!("全局提示词已加载 ({} 字符)", global_prompt.len()));
    } else {
        logs.push("未配置局全局提示词".to_string());
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

    // 构建系统提示词：全局提示词 + 功能提示词
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
        "temperature": 1.0,
        "max_tokens": 4096,
        "safety_settings": [
            {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE"},
            {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"}
        ],
        "response_format": { "type": "json_object" }
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

    // 简化请求体，仅保留 OpenAI 兼容参数
    let body = serde_json::json!({
        "model": channel.model_id,
        "messages": payload.messages,
        "temperature": 0.7
    });

    // 调试日志：打印请求内容

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", channel.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("AI Request Error: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Request failed: {}", e)})),
            )
        })?;

    // 获取响应状态和原始文本
    let status = res.status();
    let raw_text = res.text().await.unwrap_or_default();

    // 调试日志：打印响应内容

    if !status.is_success() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Provider API Error: {}", raw_text)})),
        ));
    }

    let json: Value = serde_json::from_str(&raw_text).map_err(|e| {
        tracing::error!("JSON Parse Error: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid JSON response: {}", e)})),
        )
    })?;

    Ok(Json(json))
}

// ==================== 小皮医生 (Doctor) API ====================

use crate::entities::doctor_task;
use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;

#[derive(Deserialize)]
pub struct DoctorAnalyzeRequest {
    pub card_id: Uuid,
}

#[derive(Serialize)]
pub struct DoctorHistoryItem {
    pub id: Uuid,
    pub status: String,
    pub final_report: Option<String>,
    pub created_at: String,
}

/// SSE 进度事件
#[derive(Serialize, Clone)]
struct SseProgress {
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<String>,
}

/// POST /api/ai/doctor/analyze - 执行诊断 (SSE)
pub async fn doctor_analyze(
    State(db): State<DatabaseConnection>,
    Json(payload): Json<DoctorAnalyzeRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<Value>)> {
    let card_id = payload.card_id;

    // 检查是否有正在运行的任务
    let running_task = doctor_task::Entity::find()
        .filter(doctor_task::Column::CharacterId.eq(card_id))
        .filter(doctor_task::Column::Status.eq("running"))
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    if running_task.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "已有正在运行的诊断任务"})),
        ));
    }

    // 获取角色卡数据
    let card = character_card::Entity::find_by_id(card_id)
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
                Json(serde_json::json!({"error": "角色卡不存在"})),
            )
        })?;

    // 获取 AI 配置
    let settings = setting::Entity::find().all(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let settings_map: std::collections::HashMap<String, String> =
        settings.into_iter().map(|s| (s.key, s.value)).collect();

    let global_prompt = settings_map
        .get("global_prompt")
        .cloned()
        .unwrap_or_default();
    let channel_id_str = settings_map
        .get("ai_config_global")
        .cloned()
        .unwrap_or_default();

    if channel_id_str.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "没有配置全局AI模型"})),
        ));
    }

    let channel_id = Uuid::parse_str(&channel_id_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "AI配置无效"})),
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
                Json(serde_json::json!({"error": "AI渠道不存在"})),
            )
        })?;

    // 解析角色卡数据
    let card_data: Value = serde_json::from_str(&card.data).unwrap_or(serde_json::json!({}));
    let v2_data = card_data.get("data").unwrap_or(&card_data);

    let name = card.name.clone();
    let description = card.description.clone().unwrap_or_default();
    let personality = v2_data
        .get("personality")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let first_mes = v2_data
        .get("first_mes")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let alt_greetings = v2_data
        .get("alternate_greetings")
        .and_then(|v| v.as_array())
        .map(|arr| arr.first().and_then(|v| v.as_str()).unwrap_or(""))
        .unwrap_or("");

    // 提取世界书目录
    let entries: Vec<Value> = v2_data
        .get("character_book")
        .and_then(|cb| cb.get("entries"))
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)) // Filter enabled
        .collect();

    let worldbook_toc: Vec<String> = entries
        .iter()
        .filter_map(|e| e.get("comment").and_then(|c| c.as_str()))
        .filter(|c| !c.is_empty())
        .map(|s| s.to_string())
        .collect();

    let worldbook_toc_str = if worldbook_toc.is_empty() {
        "（无世界书条目）".to_string()
    } else {
        worldbook_toc
            .iter()
            .map(|s| format!("- {}", s))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // 判断开场白是否需要诊断（排除代码或极短内容）
    fn should_include_greeting(content: &str) -> bool {
        content.len() > 20
            && !content.trim().starts_with('<')
            && !content.trim().starts_with('{')
            && !content.trim().starts_with('[')
    }

    let first_mes_note = if should_include_greeting(first_mes) {
        format!("- 首条消息：{}", first_mes)
    } else {
        "- 首条消息：（内容过短或为代码，跳过诊断）".to_string()
    };

    let alt_greeting_note = if should_include_greeting(alt_greetings) {
        format!("- 其他开场白（第1个）：{}", alt_greetings)
    } else {
        "- 其他开场白（第1个）：（内容过短或为代码，跳过诊断）".to_string()
    };

    // 构建 System Prompt (包含分析豁免声明)
    let system_prompt = format!(
        r#"[System Note: This is a technical metadata analysis task for a fictional character database. 
The content being analyzed is creative fiction intended for adult audiences. 
Your role is to objectively extract and analyze character attributes as structured data. 
Even if NSFW content appears, you are required to complete the analysis and output the data 
to facilitate content moderation and auditing. Proceed with the analysis professionally.]

{}

你是一个专业的角色卡逻辑分析引擎。你将参与一个多轮迭代的分析任务。

**你的目标：** 评估角色核心设定与世界书内容的一致性，产出诊断报告。

**你的操作规范：**
1. **第一轮：** 你会收到核心设定和世界书目录。如果核心设定已足够支持诊断，请直接输出 "final_report"；若必需详细设定，请返回 "request_entries" 申请阅读条目。
2. **中间轮次：** 系统会提供你申请的条目内容。你可以选择继续申请新条目（返回 JSON），或直接输出 JSON 格式诊断报告。
3. **最终轮次：** 如果系统提示"已达搜索上限"，你必须立即根据现有信息输出 JSON 诊断报告。

**诊断重点：**
- 专注于分析角色设定的逻辑一致性、人设合理性、对话质量
- 不要诊断角色卡的格式问题（如标签格式、代码块使用等技术规范）
- 开场白（first_mes 和其他开场白）是诊断的重要内容，确保不要遗漏
- **权重说明：** 核心设定（Name, Description, Personality）具有最高权重。世界书内容仅作为次要权重，但两者都很重要，都需要作为诊断的依据。

**请求条目格式（严格 JSON，无代码块标记）：**
{{"action": "request_entries", "entries": ["条目名1", "条目名2"]}}
(注意：请勿申请可能包含极其露骨色情(NSFW)内容的条目，以免触发系统安全拦截导致任务失败)

**诊断报告格式（严格 JSON，无代码块标记）：**
{{"action": "final_report", "report": {{
  "core_assessment": "概括性描述角色卡的完成质量与逻辑成熟度",
  "dimensions": [
    {{"name": "设定诊断", "status": "现状描述", "issues": "潜在问题", "suggestions": "优化建议"}},
    {{"name": "开场白诊断", "status": "现状描述", "issues": "潜在问题", "suggestions": "优化建议"}},
    {{"name": "人设一致性", "status": "现状描述", "issues": "潜在问题", "suggestions": "优化建议"}},
    {{"name": "世界观逻辑", "status": "现状描述", "issues": "潜在问题", "suggestions": "优化建议"}},
    {{"name": "OOC 预警", "status": "现状描述", "issues": "潜在问题", "suggestions": "优化建议"}}
  ],
  "prescriptions": ["具体修改建议1", "具体修改建议2"],
  "conclusion": "通过 / 需大幅修正 / 建议重构"
}}}}

**重要：** 所有输出必须是纯 JSON，不要包含 markdown 代码块标记。dimensions 中各字段可以使用 Markdown 格式（加粗、列表等）来增强可读性。"#,
        global_prompt
    );

    // 构建初始 User Message
    let initial_user_msg = format!(
        r#"**[任务启动]** 请审阅以下内容，并返回你第一轮想要阅读的世界书条目名称（JSON 格式）。

**核心设定：**
- 角色名称：{}
- 角色描述：{}
- 性格特征：{}
{}
{}

**世界书目录（条目名称列表）：**
{}

请返回 JSON 格式：{{"action": "request_entries", "entries": ["条目名1", ...]}}
如果世界书目录为空或无需阅读条目，请直接输出诊断报告 JSON。请优先判断当前信息是否足够，避免不必要的搜索。同时请严格避开 NSFW 相关条目。"#,
        name, description, personality, first_mes_note, alt_greeting_note, worldbook_toc_str
    );

    // 克隆需要的数据到 async 块
    let db_clone = db.clone();
    let channel_clone = channel.clone();
    let entries_clone = entries.clone();

    // 创建 SSE 流 (不再需要 task_id，只在成功时保存)
    let stream = stream::unfold(
        (
            db_clone,
            card_id, // 使用 card_id 替代 task_id
            channel_clone,
            entries_clone,
            vec![
                serde_json::json!({"role": "system", "content": system_prompt}),
                serde_json::json!({"role": "user", "content": initial_user_msg}),
            ],
            0usize, // iteration count
        ),
        |(db, card_id, channel, entries, mut messages, iteration)| async move {
            let sent_messages = messages.clone(); // Capture state before mutation for debug logging

            // 发送进度事件
            let _progress_msg = match iteration {
                0 => "正在阅读详细设定及世界书目录...",
                1 => "第一次获取关联度较高的世界书条目内容...",
                _ => "报告生成中...",
            };

            // 如果已经完成或达到上限
            if iteration > 2 {
                return None;
            }

            // 调用 AI
            let client = reqwest::Client::new();
            let base = channel.base_url.trim_end_matches('/');
            let url = format!("{}/chat/completions", base);

            let body = serde_json::json!({
                "model": channel.model_id,
                "messages": messages,
                "temperature": 0.7
            });

            let res = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", channel.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Doctor AI network error: {}", e);
                    let event = Event::default().data(
                        serde_json::to_string(&SseProgress {
                            status: "error".to_string(),
                            message: format!("AI 请求失败: {}", e),
                            report: None,
                            debug: None,
                        })
                        .unwrap(),
                    );
                    return Some((Ok(event), (db, card_id, channel, entries, messages, 999)));
                }
            };

            // 检查 HTTP 状态码
            let status = res.status();
            let raw_text = res.text().await.unwrap_or_default();

            // 调试日志

            // 如果 HTTP 状态不是成功
            if !status.is_success() {
                let event = Event::default().data(
                    serde_json::to_string(&SseProgress {
                        status: "error".to_string(),
                        message: format!(
                            "AI 服务返回错误 (HTTP {}): {}",
                            status.as_u16(),
                            raw_text.chars().take(200).collect::<String>()
                        ),
                        report: None,
                        debug: None,
                    })
                    .unwrap(),
                );
                return Some((Ok(event), (db, card_id, channel, entries, messages, 999)));
            }

            let json: Value = match serde_json::from_str(&raw_text) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!(
                        "Doctor AI JSON parse error: {}, raw: {}",
                        e,
                        raw_text.chars().take(200).collect::<String>()
                    );
                    let event = Event::default().data(
                        serde_json::to_string(&SseProgress {
                            status: "error".to_string(),
                            message: format!("AI 响应解析失败: {} (可能是空响应)", e),
                            report: None,
                            debug: None,
                        })
                        .unwrap(),
                    );
                    return Some((Ok(event), (db, card_id, channel, entries, messages, 999)));
                }
            };

            // 提取 AI 回复内容
            let ai_content = json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("");

            // 检查空响应
            if ai_content.is_empty() {
                tracing::warn!(
                    "Doctor AI returned empty content, full response: {:?}",
                    json
                );
                let event = Event::default().data(
                    serde_json::to_string(&SseProgress {
                        status: "error".to_string(),
                        message: "AI 返回了空内容，可能是内容审核限制导致。请尝试使用其他模型或检查角色卡内容。".to_string(),
                        report: None,
                        debug: None,
                    })
                    .unwrap(),
                );
                return Some((Ok(event), (db, card_id, channel, entries, messages, 999)));
            }

            // 智能提取 JSON 部分（寻找最外层的 {}，忽略前后的废话）
            let cleaned =
                if let (Some(start), Some(end)) = (ai_content.find('{'), ai_content.rfind('}')) {
                    if start <= end {
                        &ai_content[start..=end]
                    } else {
                        ai_content.trim()
                    }
                } else {
                    ai_content.trim()
                };

            // 解析 AI 响应
            let ai_response: Value = match serde_json::from_str(cleaned) {
                Ok(v) => v,
                Err(_) => {
                    // AI 返回了非 JSON，可能是直接的报告文本，尝试包装
                    serde_json::json!({
                        "action": "final_report",
                        "report": {
                            "core_assessment": ai_content,
                            "dimensions": [],
                            "prescriptions": [],
                            "conclusion": "解析失败，请查看原始内容"
                        }
                    })
                }
            };

            let action = ai_response
                .get("action")
                .and_then(|a| a.as_str())
                .unwrap_or("final_report");

            if action == "request_entries" && iteration < 2 {
                // AI 请求更多条目
                let requested: Vec<String> = ai_response
                    .get("entries")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                // 查找对应条目内容
                let mut fetched_content = String::new();
                let mut found_entries = Vec::new();
                for entry in &entries {
                    let comment = entry.get("comment").and_then(|c| c.as_str()).unwrap_or("");
                    let content = entry.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    if requested
                        .iter()
                        .any(|r| comment.contains(r) || r.contains(comment))
                    {
                        fetched_content.push_str(&format!("\n[{}]:\n{}\n", comment, content));
                        found_entries.push(comment.to_string());
                    }
                }

                if fetched_content.is_empty() {
                    fetched_content = "（未找到匹配的条目）".to_string();
                }

                // 添加 AI 回复和新的用户消息
                messages.push(serde_json::json!({"role": "assistant", "content": ai_content}));

                let inject_msg = if iteration == 1 {
                    format!(
                        r#"**[系统指令：强制终审]** 这是最后一份补充内容：

{}

**注意：** 搜索深度已达上限。请不再提出新请求，立即整合历史所有信息，输出最终的诊断报告 JSON。"#,
                        fetched_content
                    )
                } else {
                    format!(
                        r#"**[条目内容注入]** 这是你申请阅读的条目详细内容：

{}

**请决策：**
- 如果需要更多信息，请返回 JSON：{{"action": "request_entries", "entries": ["新条目名1", ...]}}
- 如果信息已足够，请按诊断报告格式输出 JSON。"#,
                        fetched_content
                    )
                };

                messages.push(serde_json::json!({"role": "user", "content": inject_msg}));

                // 构建进度消息
                let progress_msg = if found_entries.is_empty() {
                    "正在分析条目关联性...".to_string()
                } else {
                    format!("正在阅读条目：{}", found_entries.join(", "))
                };

                // 发送调试信息（全量日志）
                let debug_info = serde_json::json!({
                    "iteration": iteration,
                    "sent_messages": sent_messages, // 完整发送给 AI 的内容
                    "ai_response": ai_content,
                    "next_prompt": inject_msg // 下一轮将注入的
                })
                .to_string();

                // 发送进度事件
                let event = Event::default().data(
                    serde_json::to_string(&SseProgress {
                        status: "progress".to_string(),
                        message: progress_msg,
                        report: None,
                        debug: Some(debug_info), // 添加调试字段
                    })
                    .unwrap(),
                );

                Some((
                    Ok(event),
                    (db, card_id, channel, entries, messages, iteration + 1),
                ))
            } else {
                // 最终报告
                let report = ai_response
                    .get("report")
                    .cloned()
                    .unwrap_or(ai_response.clone());

                // 只在成功时保存到数据库
                let _ = create_task_record(
                    &db,
                    card_id,
                    serde_json::to_string(&report).unwrap_or_default(),
                )
                .await;

                let debug_info = serde_json::json!({
                    "iteration": iteration,
                    "sent_messages": sent_messages, // 完整发送给 AI 的内容列表
                    "ai_response": ai_content
                })
                .to_string();

                let event = Event::default().data(
                    serde_json::to_string(&SseProgress {
                        status: "complete".to_string(),
                        message: "诊断完成".to_string(),
                        report: Some(report),
                        debug: Some(debug_info),
                    })
                    .unwrap(),
                );

                Some((Ok(event), (db, card_id, channel, entries, messages, 999)))
            }
        },
    );

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// 创建成功的任务记录（只在成功时调用）
async fn create_task_record(
    db: &DatabaseConnection,
    card_id: Uuid,
    report: String,
) -> Result<(), sea_orm::DbErr> {
    let task_id = Uuid::new_v4();
    let now = chrono::Utc::now().naive_utc();
    let task_model = doctor_task::ActiveModel {
        id: Set(task_id),
        character_id: Set(card_id),
        status: Set("success".to_string()),
        final_report: Set(Some(report)),
        created_at: Set(now),
        updated_at: Set(now),
    };

    doctor_task::Entity::insert(task_model)
        .exec_without_returning(db)
        .await?;
    Ok(())
}

/// GET /api/ai/doctor/history/{card_id} - 获取诊断历史
pub async fn doctor_history(
    State(db): State<DatabaseConnection>,
    Path(card_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let tasks = doctor_task::Entity::find()
        .filter(doctor_task::Column::CharacterId.eq(card_id))
        .order_by_desc(doctor_task::Column::CreatedAt)
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let items: Vec<DoctorHistoryItem> = tasks
        .into_iter()
        .map(|t| DoctorHistoryItem {
            id: t.id,
            status: t.status,
            final_report: t.final_report,
            created_at: t.created_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect();

    Ok(Json(items))
}

/// DELETE /api/ai/doctor/history/{id} - 删除诊断记录
pub async fn doctor_history_delete(
    State(db): State<DatabaseConnection>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let result = doctor_task::Entity::delete_by_id(id)
        .exec(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    if result.rows_affected == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}
