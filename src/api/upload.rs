//! 文件上传 API
//!
//! 处理图片上传

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use image::{imageops::FilterType, GenericImageView};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

#[derive(Serialize)]
pub struct UploadResponse {
    pub url: String,
}

/// 上传图片
/// POST /api/upload
/// Content-Type: multipart/form-data
pub async fn upload_image(
    State(_db): State<DatabaseConnection>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Ensure upload directory exists
    let upload_dir = "data/uploads";
    if !Path::new(upload_dir).exists() {
        fs::create_dir_all(upload_dir)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        // 1. Avatar Upload (Special handling: Resize & Lossy WebP & Fixed Name)
        if name == "avatar" {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let timestamp = chrono::Utc::now().timestamp_millis();

            // Process image in blocking task
            let url = tokio::task::spawn_blocking(move || {
                let img = image::load_from_memory(&data)
                    .map_err(|e| format!("Invalid image format: {}", e))?;

                let (w, h) = img.dimensions();

                // Resize if needed (Limit to 500x500)
                let processed = if w > 500 || h > 500 {
                    img.resize(500, 500, FilterType::Lanczos3)
                } else {
                    img
                };

                // Use webp crate for lossy compression (quality 75.0)
                // Convert to RGBA8 first to ensure compatibility across image crate versions
                let (w, h) = processed.dimensions();
                let rgba = processed.to_rgba8();

                // 'webp' crate::Encoder
                // SAFETY: webp::Encoder::from_rgba expects safe slice of bytes
                let encoder = webp::Encoder::from_rgba(&rgba, w, h);
                let memory = encoder.encode(75.0); // 75% quality lossy

                let path = Path::new("data/uploads/avatar.webp");
                std::fs::write(path, &*memory)
                    .map_err(|e| format!("Failed to save webp: {}", e))?;

                Ok::<_, String>(format!("/uploads/avatar.webp?t={}", timestamp))
            })
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

            return Ok(Json(UploadResponse { url }));
        }
        // 2. Generic File Upload
        else if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown.png").to_string();
            let extension = Path::new(&file_name)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("png");

            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            // Generate unique filename
            let new_filename = format!("{}.{}", Uuid::new_v4(), extension);
            let file_path = Path::new(upload_dir).join(&new_filename);

            // Save file
            fs::write(&file_path, data)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            return Ok(Json(UploadResponse {
                url: format!("/uploads/{}", new_filename),
            }));
        }
    }

    Err((
        StatusCode::BAD_REQUEST,
        "No file/avatar field found".to_string(),
    ))
}
