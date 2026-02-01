//! Tauri 库入口

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|_app| {
            // 设置环境变量标记 Tauri 模式
            std::env::set_var("TAURI_ENV", "1");

            if let Ok(current_dir) = std::env::current_dir() {
                println!("Tauri 启动 CWD: {:?}", current_dir);
            }

            // 移动端：使用系统分配的 App Data 目录（只有这里有读写权限）
            #[cfg(mobile)]
            {
                use tauri::Manager;
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

                // 1. 默认数据目录逻辑
                let mut final_data_path = if cfg!(target_os = "macos") {
                    // macOS: 默认使用 ~/Library/Application Support/com.piney.app
                    _app.path()
                        .app_data_dir()
                        .expect("无法获取 macOS App Data 目录")
                } else {
                    // Windows/Linux: 默认使用当前目录下的 data (便携模式)
                    std::path::PathBuf::from("data")
                };

                // 2. 特殊情况处理
                // 如果当前目录下存在 data 文件夹，强制使用它（支持 macOS 便携模式）
                if std::path::PathBuf::from("data").exists() {
                    final_data_path = std::path::PathBuf::from("data");
                }

                // 开发环境修正：如果在 src-tauri 目录下运行
                if current_dir.ends_with("src-tauri") {
                    let parent_data = current_dir.parent().unwrap().join("data");
                    if parent_data.exists() {
                        println!("检测到处于 src-tauri 目录，优先使用项目根目录 data");
                        final_data_path = parent_data;
                    }
                }

                // 3. 确保目录存在
                if !final_data_path.exists() {
                    std::fs::create_dir_all(&final_data_path).expect("无法创建数据目录");
                }

                let abs_data = std::fs::canonicalize(&final_data_path).unwrap_or(final_data_path);
                println!("最终确定的数据目录 DATA_DIR: {:?}", abs_data);
                std::env::set_var("DATA_DIR", abs_data.to_string_lossy().to_string());
            }

            // 启动后端服务（在单独的线程中）
            std::thread::spawn(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = start_backend().await {
                        eprintln!("后端启动失败: {}", e);
                    }
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用时出错");
}

async fn start_backend() -> anyhow::Result<()> {
    // 1. 初始化数据库
    let db = piney::db::init_database().await?;

    // 2. 运行模式 (Tauri 下强制使用 App 模式)
    let mode = piney::utils::mode_detect::RunMode::App;

    // 3. 初始化 Config
    let config_path = piney::utils::paths::get_data_path("config.yml");

    println!("正在加载配置文件: {:?}", config_path);
    let config = piney::config::ConfigState::new(&config_path.to_string_lossy());

    // 4. 创建 Axum 应用
    let app = piney::create_app(db, mode, config).await;

    // 5. 启动侦听 (仅本地回路)
    let port = 9696;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    println!("Tauri 后端服务启动于 http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
