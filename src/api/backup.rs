//! 数据备份与恢复 API
//!
//! 导出：打包整个 data/ 目录为 .tar.gz (返回为 .piney)
//! 导入：解压 .piney 文件覆盖 data/ 目录

use axum::{
    body::Body,
    extract::Multipart,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Local;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use tar::{Archive, Builder};

/// 获取 data 目录路径
fn get_data_dir() -> std::path::PathBuf {
    crate::utils::paths::get_data_path("")
}

/// GET /api/backup/export - 导出系统数据为 .piney 文件
pub async fn export_backup() -> Result<impl IntoResponse, (StatusCode, String)> {
    let data_dir = get_data_dir();

    if !data_dir.exists() {
        return Err((StatusCode::NOT_FOUND, "数据目录不存在".to_string()));
    }

    // 1. 创建临时目录 data/temp
    let temp_dir = data_dir.join("temp");
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("无法创建临时目录: {}", e),
            )
        })?;
    }

    // 2. 清理超过 1 小时的旧临时文件
    let _ = tokio::task::spawn_blocking({
        let temp_dir = temp_dir.clone();
        move || {
            if let Ok(entries) = fs::read_dir(temp_dir) {
                for entry in entries.flatten() {
                    // 激进清理：每次导出前，清空 temp 目录下的所有文件
                    // 这样目录里永远只会有当前这一个备份文件
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    });

    // 3. 生成临时文件路径
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("piney_backup_{}.piney", timestamp);
    let temp_filepath = temp_dir.join(&filename);

    // 4. 将 tar.gz 写入临时文件
    let temp_filepath_clone = temp_filepath.clone();
    let data_dir_clone = data_dir.clone();

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // 创建文件
        let file = fs::File::create(&temp_filepath_clone)
            .map_err(|e| format!("无法创建临时文件: {}", e))?;

        // Gzip 编码器
        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar_builder = Builder::new(encoder);

        // 递归添加 data 目录下的所有文件
        // 排除 temp 目录本身，防止递归死循环
        if let Ok(entries) = fs::read_dir(&data_dir_clone) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.file_name().and_then(|n| n.to_str()) == Some("temp") {
                    continue;
                }

                // 计算相对路径
                let relative_path = path
                    .strip_prefix(&data_dir_clone)
                    .map_err(|e| format!("路径错误: {}", e))?;

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

        // 完成写入
        let encoder = tar_builder
            .into_inner()
            .map_err(|e| format!("Tar finalize failed: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Gzip finish failed: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("任务执行失败: {}", e),
        )
    })?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // 5. 打开文件并流式返回
    let file = tokio::fs::File::open(&temp_filepath).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("无法读取临时文件: {}", e),
        )
    })?;

    let stream = tokio_util::io::ReaderStream::new(file);
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
pub async fn import_backup(mut multipart: Multipart) -> Result<StatusCode, (StatusCode, String)> {
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

    // 2. 验证是否为有效的 tar.gz
    let is_valid = {
        let data_clone = data.clone();
        tokio::task::spawn_blocking(move || {
            let decoder = GzDecoder::new(&data_clone[..]);
            let mut archive = Archive::new(decoder);
            // 尝试读取条目来验证
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

    // 3. 清空 data 目录（除了正在使用的数据库文件，稍后处理）
    // 注意：这里我们先删除非数据库文件，数据库文件最后处理
    let data_clone = data.clone();
    let data_dir_clone = data_dir.clone();

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        // 读取目录内容
        let entries =
            fs::read_dir(&data_dir_clone).map_err(|e| format!("读取数据目录失败: {}", e))?;

        // 删除所有内容（SQLite 文件可能被锁定，我们需要特殊处理）
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name().unwrap_or_default().to_string_lossy();

            // 跳过 .DS_Store 之类的系统文件
            if filename.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .map_err(|e| format!("删除目录 {} 失败: {}", path.display(), e))?;
            } else {
                // 对于 SQLite 数据库，尝试删除
                // 如果失败（被锁定），我们稍后会覆盖它
                let _ = fs::remove_file(&path);
            }
        }

        // 4. 解压备份到 data 目录
        let decoder = GzDecoder::new(&data_clone[..]);
        let mut archive = Archive::new(decoder);

        archive
            .unpack(&data_dir_clone)
            .map_err(|e| format!("解压失败: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("任务执行失败: {}", e),
        )
    })?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // 5. 返回成功
    // 注意：数据库连接可能需要重启服务才能生效
    // 这里我们返回成功，前端应提示用户刷新页面或重启服务
    Ok(StatusCode::OK)
}
