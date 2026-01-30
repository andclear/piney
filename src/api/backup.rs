//! 数据备份与恢复 API
//!
//! 导出：打包整个 data/ 目录为 .tar.gz (返回为 .piney)
//! 导入：解压 .piney 文件覆盖 data/ 目录

use axum::{
    body::Body,
    extract::{Json, Multipart},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Local;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use tar::{Archive, Builder};
use tokio::io::duplex;
use tokio_util::io::{ReaderStream, SyncIoBridge}; // 用于流式传输
use tracing::error;

#[derive(Serialize)]
pub struct ImportResponse {
    username: String,
    message: String,
}

#[derive(Deserialize)]
struct SimpleConfig {
    username: String,
}

/// 获取 data 目录路径
fn get_data_dir() -> std::path::PathBuf {
    crate::utils::paths::get_data_path("")
}

/// GET /api/backup/export - 导出系统数据为 .piney 文件 (流式传输)
pub async fn export_backup() -> Result<impl IntoResponse, (StatusCode, String)> {
    let data_dir = get_data_dir();

    if !data_dir.exists() {
        return Err((StatusCode::NOT_FOUND, "数据目录不存在".to_string()));
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("piney_backup_{}.piney", timestamp);

    // 1. 创建内存管道 (4MB 缓冲区，优化 200MB/s+ 高速下载体验)
    // reader 给 Axum 返回给前端作为 Response Body
    // writer 给 tar::Builder 写入数据
    let (reader, writer) = duplex(4 * 1024 * 1024);

    let data_dir_clone = data_dir.clone();

    // 2. 启动后台任务进行打包
    // 使用 SyncIoBridge 将 异步writer 转换为 同步write，供同步的 tar 库使用
    // spawn_blocking 在专用线程池运行，不会阻塞因为 heavy I/O
    tokio::task::spawn_blocking(move || {
        // SyncIoBridge 会在 drop 时关闭 writer，从而给 reader 发送 EOF
        let bridge = SyncIoBridge::new(writer);
        // 关键优化：使用 BufWriter 减少 tar 的大量小 I/O (?512bytes) 操作导致的频繁 context switch
        // 4MB 缓冲区与 pipe 容量一致，最大化吞吐量
        let buffered_bridge = std::io::BufWriter::with_capacity(4 * 1024 * 1024, bridge);
        let mut tar_builder = Builder::new(buffered_bridge);

        // 使用闭包来捕获 Result，方便统一处理错误
        let result = (|| -> Result<(), String> {
            if let Ok(entries) = fs::read_dir(&data_dir_clone) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    // 忽略 temp 目录（虽然新版不再创建 temp 文件，但防御性编程保留）
                    if path.file_name().and_then(|n| n.to_str()) == Some("temp") {
                        continue;
                    }

                    // 计算相对路径
                    let relative_path = path
                        .strip_prefix(&data_dir_clone)
                        .map_err(|e| format!("路径错误: {}", e))?;

                    // 写入 tar
                    if path.is_dir() {
                        tar_builder
                            .append_dir_all(relative_path, &path)
                            .map_err(|e| format!("打包目录失败 {:?}: {}", path, e))?;
                    } else {
                        tar_builder
                            .append_path_with_name(&path, relative_path)
                            .map_err(|e| format!("打包文件失败 {:?}: {}", path, e))?;
                    }
                }
            }

            // 完成打包
            tar_builder
                .finish()
                .map_err(|e| format!("Tar finish failed: {}", e))?;

            Ok(())
        })();

        if let Err(e) = result {
            // 在流传输过程中发生错误，只能记录日志，无法修改 HTTP 状态码
            error!("备份打包流式传输失败: {}", e);
        }
    });

    // 3. 构建响应流
    // 使用 ReaderStream 将 AsyncRead 转换为 Stream
    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    // 构建响应头
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
    );

    Ok((headers, body))
}

/// POST /api/backup/import - 导入 .piney 备份文件并恢复数据
pub async fn import_backup(
    mut multipart: Multipart,
) -> Result<Json<ImportResponse>, (StatusCode, String)> {
    // 1. 读取上传的文件
    let mut file_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("backup") || field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("读取文件失败: {}", e)))?;
            file_data = Some(data.to_vec());
            break;
        }
    }

    let data = file_data.ok_or((StatusCode::BAD_REQUEST, "未找到备份文件".to_string()))?;

    // 2. 验证是否为有效的备份文件 (Tar 或 TarGz)
    let is_valid = {
        let data_clone = data.clone();
        tokio::task::spawn_blocking(move || {
            // 尝试当作 Tar 读取
            let mut archive = Archive::new(&data_clone[..]);
            if archive.entries().is_ok() {
                return true;
            }

            // 尝试当作 Gzip 读取
            let decoder = GzDecoder::new(&data_clone[..]);
            let mut archive = Archive::new(decoder);
            archive.entries().is_ok()
        })
        .await
        .unwrap_or(false)
    };

    if !is_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            "无效的备份文件格式（不是有效的 tar.gz）".to_string(),
        ));
    }

    let data_dir = get_data_dir();

    // 3. 执行清空、解压、读取配置、清理密钥
    let data_clone = data.clone();
    let data_dir_clone = data_dir.clone();

    let username = tokio::task::spawn_blocking(move || -> Result<String, String> {
        // A. 清空 data 目录
        let entries =
            fs::read_dir(&data_dir_clone).map_err(|e| format!("读取数据目录失败: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name().unwrap_or_default().to_string_lossy();

            // 跳过 .DS_Store
            if filename.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .map_err(|e| format!("删除目录 {} 失败: {}", path.display(), e))?;
            } else {
                let _ = fs::remove_file(&path);
            }
        }

        // B. 解压备份到 data 目录
        // 尝试自动检测格式：如果读取前两个字节是 Gzip 头 (0x1f 0x8b)，则使用 Gzip 解压
        // 否则当作普通 Tar 文件处理
        let is_gzip = data_clone.len() >= 2 && data_clone[0] == 0x1f && data_clone[1] == 0x8b;

        if is_gzip {
            let decoder = GzDecoder::new(&data_clone[..]);
            let mut archive = Archive::new(decoder);
            archive
                .unpack(&data_dir_clone)
                .map_err(|e| format!("Gzip解压失败: {}", e))?;
        } else {
            let mut archive = Archive::new(&data_clone[..]);
            archive
                .unpack(&data_dir_clone)
                .map_err(|e| format!("Tar解压失败: {}", e))?;
        }

        // C. 读取恢复后的 config.yml 中的用户名
        let config_path = data_dir_clone.join("config.yml");
        let mut username = "admin".to_string(); // 默认值

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                // 简单解析 yaml
                if let Ok(cfg) = serde_yaml::from_str::<SimpleConfig>(&content) {
                    username = cfg.username;
                }
            }
        }

        // D. 强制清理 .jwt_secret (确保用户退出登录)
        let secret_path = data_dir_clone.join(".jwt_secret");
        if secret_path.exists() {
            let _ = fs::remove_file(secret_path);
        }

        Ok(username)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("任务执行失败: {}", e),
        )
    })?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // 5. 返回成功信息
    Ok(Json(ImportResponse {
        username,
        message: "数据恢复成功".to_string(),
    }))
}
