//! Tauri 库入口

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|_app| {
            // 设置环境变量标记 Tauri 模式
            std::env::set_var("TAURI_ENV", "1");

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
    // TODO: 启动 Axum 后端服务
    // 这里会调用主程序的逻辑
    Ok(())
}
