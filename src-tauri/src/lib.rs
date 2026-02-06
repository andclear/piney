//! Tauri 库入口

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|_app| {
            // 0. 全局 Panic 捕获 (调试用)
            std::panic::set_hook(Box::new(|info| {
                let msg = format!("Panic occurred: {:?}", info);
                let _ = std::fs::write("panic_crash.txt", msg);
            }));

            // 设置环境变量标记 Tauri 模式
            std::env::set_var("TAURI_ENV", "1");

            if let Ok(current_dir) = std::env::current_dir() {
                println!("Tauri 启动 CWD: {:?}", current_dir);
            }

            // 移动端：使用系统分配的 App Data 目录（只有这里有读写权限）
            #[cfg(mobile)]
            {
                let path = _app.path().app_data_dir().expect("无法获取 App Data 目录");
                if !path.exists() {
                    std::fs::create_dir_all(&path).expect("无法创建数据目录");
                }
                println!("移动端数据目录 DATA_DIR: {:?}", path);
                std::env::set_var("DATA_DIR", path.to_string_lossy().to_string());
            }

            // 桌面端
            #[cfg(not(mobile))]
            {
                let current_dir = std::env::current_dir().unwrap_or_default();
                let local_data = current_dir.join("data");

                let mut use_local = false;

                // 1. 尝试使用或创建当前目录下的 data (便携模式优先)
                if local_data.exists() {
                    use_local = true;
                } else {
                    // 尝试创建 ./data
                    if let Ok(_) = std::fs::create_dir(&local_data) {
                        println!("成功创建本地 data 目录: {:?}", local_data);
                        use_local = true;
                    } else {
                        println!("无法在当前目录创建 data (可能是权限不足)，将回退到 AppData");
                    }
                }

                let mut final_data_path;
                if use_local {
                    final_data_path = local_data;
                } else {
                    // 2. 回退到系统 AppData
                    final_data_path = _app
                        .path()
                        .app_data_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("data"));
                }

                // 开发环境修正：如果在 src-tauri 目录下运行
                if current_dir.ends_with("src-tauri") {
                    let parent_data = current_dir.parent().unwrap().join("data");
                    if parent_data.exists() {
                        final_data_path = parent_data;
                    }
                }

                // 3. 确保目录存在
                if !final_data_path.exists() {
                    let _ = std::fs::create_dir_all(&final_data_path);
                }

                let abs_data = std::fs::canonicalize(&final_data_path).unwrap_or(final_data_path);
                println!("最终确定的数据目录 DATA_DIR: {:?}", abs_data);
                std::env::set_var("DATA_DIR", abs_data.to_string_lossy().to_string());
            }

            let data_dir_str = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
            let log_path_clone = std::path::PathBuf::from(data_dir_str).join("startup.log");
            let log = move |msg: &str| {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path_clone)
                {
                    let _ = writeln!(
                        file,
                        "[{}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        msg
                    );
                }
            };

            log("Tauri setup completed. Spawning backend thread...");

            // 启动后端服务（在单独的线程中）
            // 使用 AtomicBool 追踪后端状态：
            // - swap(true) 防止 Activity 重建导致重复启动 (和 OnceLock 效果一样)
            // - 崩溃后 store(false) 允许下次重启 (OnceLock 做不到)
            use std::sync::atomic::{AtomicBool, Ordering};
            static BACKEND_RUNNING: AtomicBool = AtomicBool::new(false);

            // swap(true) 返回旧值：如果是 false，说明后端没在运行，我们启动它
            // 如果是 true，说明后端已在运行，跳过
            if !BACKEND_RUNNING.swap(true, Ordering::SeqCst) {
                let log_clone = log.clone();
                log("Spawning backend thread...");
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        log_clone("Backend thread started.");
                        if let Err(e) = start_backend(log_clone.clone()).await {
                            log_clone(&format!("Backend CRASHED: {}", e));
                            eprintln!("后端启动失败: {}", e);
                        } else {
                            log_clone("Backend stopped unexpectedly (or app closed).");
                        }
                        // 关键：后端退出后重置标志，允许下次 Activity 重建时重启
                        BACKEND_RUNNING.store(false, Ordering::SeqCst);
                        log_clone("Backend exited. Flag reset, will restart on next lifecycle.");
                    });
                });
            } else {
                log("Backend already running. Skipping spawn.");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用时出错");
}

async fn start_backend<F>(log: F) -> anyhow::Result<()>
where
    F: Fn(&str) + Clone + Send + Sync + 'static,
{
    // 数据库初始化 (带重试逻辑，解决 Android 上的瞬时故障)
    log("Initializing database...");
    let mut db_retries = 10;
    let db = loop {
        match piney::db::init_database().await {
            Ok(d) => {
                log("Database initialized successfully.");
                break d;
            }
            Err(e) => {
                db_retries -= 1;
                if db_retries <= 0 {
                    log(&format!("Database init failed after 10 retries: {}", e));
                    return Err(e.into());
                }
                log(&format!(
                    "Database init failed: {}, retrying in 1s... ({} retries left)",
                    e, db_retries
                ));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    };

    // 2. 运行模式
    let mode = piney::utils::mode_detect::RunMode::App;

    // 3. 初始化 Config
    let config_path = piney::utils::paths::get_data_path("config.yml");
    log(&format!("Loading config from: {:?}", config_path));

    let config = piney::config::ConfigState::new(&config_path.to_string_lossy());

    // 4. 创建 Axum 应用
    log("Creating Axum app...");
    let app = piney::create_app(db, mode, config).await;

    // 5. 启动侦听
    let port = 9696;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    log(&format!("Binding to address: {}", addr));

    let listener = loop {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => break l,
            Err(e) => {
                log(&format!(
                    "Failed to bind port {}: {}, retrying in 1s...",
                    port, e
                ));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    };

    log("Server listening. Entering loop...");
    axum::serve(listener, app).await?;

    Ok(())
}
