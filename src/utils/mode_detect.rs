//! 运行模式检测
//!
//! 检测应用的运行环境

/// 运行模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// 服务器模式 (Docker)
    Server,
    /// 应用模式 (Tauri)
    App,
}

impl RunMode {
    /// 自动检测运行模式
    pub fn detect() -> Self {
        // 检查环境变量
        if let Ok(mode) = std::env::var("RUN_MODE") {
            match mode.to_lowercase().as_str() {
                "server" | "docker" => return Self::Server,
                "app" | "tauri" => return Self::App,
                _ => {}
            }
        }

        // 检查是否在 Docker 容器中
        if std::path::Path::new("/.dockerenv").exists() {
            return Self::Server;
        }

        // 检查 Tauri 特征
        if std::env::var("TAURI_ENV").is_ok() {
            return Self::App;
        }

        // 默认使用服务器模式
        Self::Server
    }

    /// 是否为服务器模式
    pub fn is_server(&self) -> bool {
        matches!(self, Self::Server)
    }

    /// 是否为应用模式
    pub fn is_app(&self) -> bool {
        matches!(self, Self::App)
    }
}
