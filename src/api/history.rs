use crate::entities::{chat_history, prelude::*};
use anyhow::Result;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ChatHistoryDto {
    pub id: Uuid,
    pub card_id: Uuid,
    pub file_name: String,
    pub display_name: String,
    pub source_file_name: Option<String>,
    pub file_size: i64,
    pub format: String,
    pub progress: i32,
    pub created_at: String,
    pub updated_at: String,
    pub current_page: i32,
    pub reading_settings: Option<String>,
    pub regex_scripts: String,
}

impl From<chat_history::Model> for ChatHistoryDto {
    fn from(model: chat_history::Model) -> Self {
        Self {
            id: model.id,
            card_id: model.card_id,
            file_name: model.file_name,
            display_name: model.display_name,
            source_file_name: model.source_file_name,
            file_size: model.file_size,
            format: model.format,
            progress: model.progress,
            current_page: model.current_page,
            reading_settings: model.reading_settings,
            regex_scripts: model.regex_scripts,
            created_at: model.created_at.and_utc().to_rfc3339(),
            updated_at: model.updated_at.and_utc().to_rfc3339(),
        }
    }
}

pub async fn list_history(
    State(db): State<DatabaseConnection>,
    Path(card_id): Path<Uuid>,
) -> Result<Json<Vec<ChatHistoryDto>>, (StatusCode, String)> {
    let histories = ChatHistory::find()
        .filter(chat_history::Column::CardId.eq(card_id))
        .order_by_desc(chat_history::Column::CreatedAt)
        .all(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let dtos: Vec<ChatHistoryDto> = histories.into_iter().map(ChatHistoryDto::from).collect();
    Ok(Json(dtos))
}

pub async fn upload_history(
    State(db): State<DatabaseConnection>,
    Path(card_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ChatHistoryDto>, (StatusCode, String)> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let card_dir = PathBuf::from(&data_dir)
        .join("cards")
        .join(card_id.to_string());

    if !card_dir.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            "Character card directory not found".to_string(),
        ));
    }

    let mut file_data: Option<(String, Vec<u8>)> = None;
    let mut source_file_data: Option<(String, Vec<u8>)> = None;
    let mut is_wind_mode = false;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown.txt").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            file_data = Some((file_name, data.to_vec()));
        } else if name == "source_file" {
            let file_name = field.file_name().unwrap_or("unknown.jsonl").to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            source_file_data = Some((file_name, data.to_vec()));
        } else if name == "wind_mode" {
            let val = field.text().await.unwrap_or("false".to_string());
            is_wind_mode = val == "true";
        }
    }

    let (file_name, data) =
        file_data.ok_or((StatusCode::BAD_REQUEST, "Missing file field".to_string()))?;

    // Wind Mode: Allow .jsonl, map as main file.
    let format = if is_wind_mode {
        if !file_name.ends_with(".jsonl") {
            return Err((
                StatusCode::BAD_REQUEST,
                "Wind Mode requires .jsonl file".to_string(),
            ));
        }
        "jsonl"
    } else {
        // Normal Mode: Validates .txt
        if !file_name.ends_with(".txt") {
            return Err((
                StatusCode::BAD_REQUEST,
                "Main file must be .txt".to_string(),
            ));
        }
        "txt"
    };

    let file_size = data.len() as i64;

    // Generate unique filename for main file
    let mut save_name = file_name.clone();
    let mut counter = 1;
    while card_dir.join(&save_name).exists() {
        let stem = std::path::Path::new(&file_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let ext = std::path::Path::new(&file_name)
            .extension()
            .unwrap()
            .to_str()
            .unwrap();
        save_name = format!("{}_{}.{}", stem, counter, ext);
        counter += 1;
    }

    let file_path = card_dir.join(&save_name);
    fs::write(&file_path, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Handle source file if present (only relevant for TXT mode usually, but maybe user uploads both in wind mode? Probably not.)
    let mut saved_source_name = None;
    if let Some((source_name, source_data)) = source_file_data {
        let stem = std::path::Path::new(&save_name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let source_ext = std::path::Path::new(&source_name)
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("jsonl");

        let source_save_name = format!("{}.source.{}", stem, source_ext);
        let source_path = card_dir.join(&source_save_name);

        fs::write(&source_path, &source_data)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        saved_source_name = Some(source_save_name);
    }

    let now = Utc::now().naive_utc();
    let history = chat_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        card_id: Set(card_id),
        file_name: Set(save_name.clone()),
        display_name: Set(save_name),
        source_file_name: Set(saved_source_name),
        file_size: Set(file_size),
        format: Set(format.to_string()),
        progress: Set(0),
        current_page: Set(1),
        reading_settings: Set(None),
        regex_scripts: Set("[]".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = history
        .insert(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ChatHistoryDto::from(saved)))
}

#[derive(Deserialize)]
pub struct UpdateHistoryReq {
    pub display_name: Option<String>,
    pub progress: Option<i32>,
    pub current_page: Option<i32>,
    pub reading_settings: Option<String>,
    pub regex_scripts: Option<String>,
}

pub async fn update_history(
    State(db): State<DatabaseConnection>,
    Path((_card_id, history_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateHistoryReq>,
) -> Result<Json<ChatHistoryDto>, (StatusCode, String)> {
    let history = ChatHistory::find_by_id(history_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "History not found".to_string()))?;

    let history_model = history.clone(); // Clone for usage in path building if needed, though we have history
    let mut active: chat_history::ActiveModel = history.into();

    if let Some(name) = payload.display_name {
        // If name changes, rename the file
        if name != history_model.display_name {
            let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
            let card_dir = PathBuf::from(&data_dir)
                .join("cards")
                .join(history_model.card_id.to_string());

            let old_file_name = history_model.file_name.clone();
            let old_path = card_dir.join(&old_file_name);

            // Determine extension
            let ext = std::path::Path::new(&old_file_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("txt");

            // Sanitize name lightly (allow spaces/CN chars, just avoid path separators)
            let safe_name = name.replace(['/', '\\', ':', '*'], "_");

            let mut new_file_name = format!("{}.{}", safe_name, ext);

            // Handle collision
            if new_file_name != old_file_name {
                let mut counter = 1;
                while card_dir.join(&new_file_name).exists() {
                    new_file_name = format!("{}_{}.{}", safe_name, counter, ext);
                    counter += 1;
                }

                let new_path = card_dir.join(&new_file_name);

                if old_path.exists() {
                    fs::rename(&old_path, &new_path).await.map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to rename file: {}", e),
                        )
                    })?;
                }

                active.file_name = Set(new_file_name.clone());

                // Rename source file if exists
                if let Some(old_source_name) = &history_model.source_file_name {
                    let old_source_path = card_dir.join(old_source_name);
                    if old_source_path.exists() {
                        let new_stem = std::path::Path::new(&new_file_name)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(&safe_name);

                        let source_ext = std::path::Path::new(old_source_name)
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("jsonl");

                        let new_source_name = format!("{}.source.{}", new_stem, source_ext);
                        let new_source_path = card_dir.join(&new_source_name);

                        fs::rename(&old_source_path, &new_source_path)
                            .await
                            .map_err(|e| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Failed to rename source file: {}", e),
                                )
                            })?;

                        active.source_file_name = Set(Some(new_source_name));
                    }
                }
            }
        }
        active.display_name = Set(name);
    }
    if let Some(prog) = payload.progress {
        active.progress = Set(prog);
    }
    if let Some(page) = payload.current_page {
        active.current_page = Set(page);
    }
    if let Some(settings) = payload.reading_settings {
        active.reading_settings = Set(Some(settings));
    }
    if let Some(scripts) = payload.regex_scripts {
        active.regex_scripts = Set(scripts);
    }
    active.updated_at = Set(Utc::now().naive_utc());

    let updated = active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ChatHistoryDto::from(updated)))
}

pub async fn delete_history(
    State(db): State<DatabaseConnection>,
    Path((card_id, history_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let history = ChatHistory::find_by_id(history_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "History not found".to_string()))?;

    // Delete file
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let file_path = PathBuf::from(&data_dir)
        .join("cards")
        .join(card_id.to_string())
        .join(&history.file_name);

    if file_path.exists() {
        fs::remove_file(file_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Delete Source file if exists
    if let Some(source_name) = &history.source_file_name {
        let source_path = PathBuf::from(&data_dir)
            .join("cards")
            .join(card_id.to_string())
            .join(source_name);

        if source_path.exists() {
            let _ = fs::remove_file(source_path).await;
        }
    }

    // Delete DB record
    history
        .delete(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

use axum::extract::Query;

use regex::Regex;

#[derive(Serialize)]
pub struct PaginatedContent {
    pub total_pages: usize,
    pub current_page: usize,
    pub floors: Vec<ChatMessage>,
    pub detected_tags: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct ChatMessage {
    pub floor: i32,
    pub name: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct GetContentQuery {
    pub source: Option<bool>,
    pub page: Option<usize>,
}

pub async fn get_history_content(
    State(db): State<DatabaseConnection>,
    Path((card_id, history_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<GetContentQuery>,
) -> Result<Body, (StatusCode, String)> {
    let history = ChatHistory::find_by_id(history_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "History not found".to_string()))?;

    let target_file_name = if query.source.unwrap_or(false) {
        history.source_file_name.ok_or((
            StatusCode::NOT_FOUND,
            "No source file available".to_string(),
        ))?
    } else {
        history.file_name
    };

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let file_path = PathBuf::from(&data_dir)
        .join("cards")
        .join(card_id.to_string())
        .join(&target_file_name);

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found on disk".to_string()));
    }

    // If 'page' is None, behavior = existing raw download
    if query.page.is_none() {
        let file = fs::File::open(file_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let stream = ReaderStream::new(file);
        return Ok(Body::from_stream(stream));
    }

    // Pagination Logic
    let page = query.page.unwrap().max(1);
    // Let's use 30 for txt and 2 for jsonl (as users requested).

    let is_jsonl = history.format == "jsonl" || target_file_name.ends_with(".jsonl");
    let current_page_size = if is_jsonl { 2 } else { 30 };

    let content = fs::read_to_string(file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Global Tag Detection
    // Scan all unique tags from the raw content to support global filtering in UI
    // Global Tag Detection
    // Scan all unique tags from the raw content to support global filtering in UI
    let detected_tags: Vec<String> = {
        let tag_regex = Regex::new(r"</?([a-zA-Z0-9_\-\.\u4e00-\u9fa5]+)(?:\s[^>]*)?>").unwrap();
        let mut tags_set = std::collections::HashSet::new();
        // Expanded Common HTML tags to ignore (Blocklist)
        let ignore = std::collections::HashSet::from([
            "html",
            "head",
            "body",
            "script",
            "style",
            "div",
            "p",
            "span",
            "br",
            "hr",
            "img",
            "a",
            "b",
            "i",
            "u",
            "s",
            "strike",
            "del",
            "strong",
            "em",
            "code",
            "pre",
            "blockquote",
            "thead",
            "tbody",
            "tfoot",
            "tr",
            "th",
            "td",
            "caption",
            "ul",
            "ol",
            "li",
            "dl",
            "dt",
            "dd",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "form",
            "input",
            "button",
            "textarea",
            "select",
            "option",
            "label",
            "fieldset",
            "legend",
            "iframe",
            "svg",
            "path",
            "canvas",
            "audio",
            "video",
            "source",
            "track",
            "embed",
            "object",
            "nav",
            "header",
            "footer",
            "main",
            "section",
            "article",
            "aside",
            "dialog",
        ]);

        let mut stack: Vec<String> = Vec::new();
        // We only care about the structure, so we iterate through tags in order
        for cap in tag_regex.captures_iter(&content) {
            if let Some(m) = cap.get(0) {
                let full_tag = m.as_str();
                let is_close = full_tag.starts_with("</");
                let tag_name_raw = cap.get(1).unwrap().as_str(); // Capture 1 is name
                let tag_name = tag_name_raw.to_lowercase();

                if ignore.contains(tag_name.as_str()) {
                    continue; // Skip common HTML completely (treated as text)
                }

                if is_close {
                    // Try to pop matching tag from stack (handle auto-closing / mismatch)
                    // If we find the tag in the stack, pop everything up to it
                    if let Some(pos) = stack.iter().rposition(|t| t == &tag_name) {
                        stack.truncate(pos);
                    }
                } else {
                    // Open Tag
                    // Logic:
                    // 1. If stack is empty -> Top Level -> Add
                    // 2. If stack contains "content" -> Inside Content -> Add as "content_Name"
                    // 3. Else -> Nested -> Ignore

                    if !full_tag.trim().ends_with("/>") {
                        stack.push(tag_name);
                    }

                    tags_set.insert(tag_name_raw.to_string());
                }
            }
        }

        let mut v: Vec<String> = tags_set.into_iter().collect();
        v.sort(); // Consistent order
        v
    };

    let mut all_floors = Vec::new();

    if is_jsonl {
        // Line-by-line parsing
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        let total_floors = lines.len();
        let total_pages = (total_floors as f64 / current_page_size as f64).ceil() as usize;
        let actual_page = page.min(total_pages).max(1);

        if total_floors == 0 {
            return Ok(Body::from(
                serde_json::to_string(&PaginatedContent {
                    total_pages: 1,
                    current_page: 1,
                    floors: vec![],
                    detected_tags: detected_tags.clone(),
                })
                .unwrap(),
            ));
        }

        let start_idx = (actual_page - 1) * current_page_size;
        let end_idx = (start_idx + current_page_size).min(total_floors);

        for (idx, line) in lines[start_idx..end_idx].iter().enumerate() {
            // Parse line as JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                // Try to find name and content fields
                // Common formats: SillyTavern uses "name", "mes" or "message"
                let name = json
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let content = json
                    .get("mes")
                    .or_else(|| json.get("message"))
                    .or_else(|| json.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                all_floors.push(ChatMessage {
                    floor: (start_idx + idx + 1) as i32,
                    name,
                    content,
                });
            }
        }

        let result = PaginatedContent {
            total_pages,
            current_page: actual_page,
            floors: all_floors,
            detected_tags,
        };
        return Ok(Body::from(serde_json::to_string(&result).unwrap()));
    }

    // Default TXT Parsing Logic
    // ... existing logic ...

    // Regex: Match the header line: [#123] 【Name】
    // Then we capture everything until the next header or EOF.
    let re_header = Regex::new(r"(?m)^\[#(\d+)\]\s*【(.*?)】\s*").unwrap();

    let mut headers = Vec::new();
    for mat in re_header.find_iter(&content) {
        let caps = re_header.captures(mat.as_str()).unwrap();
        let floor = caps[1].parse::<i32>().unwrap_or(0);
        let name = caps[2].trim().to_string();
        headers.push((mat.start(), mat.end(), floor, name));
    }

    for i in 0..headers.len() {
        let (_start, end, floor, name) = headers[i].clone();

        let content_end = if i + 1 < headers.len() {
            headers[i + 1].0
        } else {
            content.len()
        };

        let body = content[end..content_end].trim().to_string();

        all_floors.push(ChatMessage {
            floor,
            name,
            content: body,
        });
    }

    let total_floors = all_floors.len();
    let total_pages = (total_floors as f64 / current_page_size as f64).ceil() as usize;
    let actual_page = page.min(total_pages).max(1);

    // If no floors found (e.g. empty file or format mismatch), handle gracefully
    if total_floors == 0 {
        return Ok(Body::from(
            serde_json::to_string(&PaginatedContent {
                total_pages: 1,
                current_page: 1,
                floors: vec![],
                detected_tags: detected_tags.clone(),
            })
            .unwrap(),
        ));
    }

    let start_idx = (actual_page - 1) * current_page_size;
    let end_idx = (start_idx + current_page_size).min(total_floors);

    let page_floors = if start_idx < total_floors {
        all_floors[start_idx..end_idx].to_vec()
    } else {
        vec![]
    };

    let result = PaginatedContent {
        total_pages,
        current_page: actual_page,
        floors: page_floors,
        detected_tags,
    };

    Ok(Body::from(serde_json::to_string(&result).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?))
}

pub async fn update_history_content(
    State(db): State<DatabaseConnection>,
    Path((card_id, history_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<Json<ChatHistoryDto>, (StatusCode, String)> {
    let history = ChatHistory::find_by_id(history_id)
        .one(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "History not found".to_string()))?;

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let card_dir = PathBuf::from(&data_dir)
        .join("cards")
        .join(card_id.to_string());

    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or((StatusCode::BAD_REQUEST, "Missing file content".to_string()))?;
    let file_size = data.len() as i64;

    // Overwrite existing file
    let file_path = card_dir.join(&history.file_name);
    fs::write(&file_path, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update DB
    let mut active: chat_history::ActiveModel = history.into();
    active.file_size = Set(file_size);
    active.updated_at = Set(Utc::now().naive_utc());

    let updated = active
        .update(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ChatHistoryDto::from(updated)))
}
