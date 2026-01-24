use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::info;

const CONFIG_HEADER: &str = r#"# =========================================================
# Piney 配置文件
# =========================================================
# 密码重置说明:
# 如果需要修改密码，直接修改 password 字段即可
# 重启程序后立即生效
# =========================================================
"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub username: String,
    pub password: String, // 明文密码
                          // jwt_secret 已移除，改为独立管理
}

#[derive(Clone)]
pub struct ConfigState {
    config: Arc<RwLock<Option<AppConfig>>>,
    jwt_secret: String, // 在内存中保存
    file_path: String,
}

impl ConfigState {
    pub fn new(file_path: &str) -> Self {
        // 1. 获取/生成 JWT Secret
        let jwt_secret = crate::utils::secret::get_jwt_secret();

        // 2. 尝试读取配置文件
        let mut config: Option<AppConfig> = if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path).ok();
            if let Some(c) = content.as_ref() {
                // 定义检查结构体，宽容解析
                #[derive(Deserialize)]
                struct ConfigCheck {
                    username: String,
                    password: Option<String>,
                    password_hash: Option<String>, // 旧版字段
                    jwt_secret: Option<String>,    // 待移除字段
                }

                if let Ok(check) = serde_yaml::from_str::<ConfigCheck>(c) {
                    let mut needs_save = false;
                    let final_password: String;

                    // 检查 A: 是否包含 password_hash (需要重置密码)
                    if check.password_hash.is_some() {
                        info!("检测到旧版配置(password_hash)，执行重置...");
                        final_password = "12345678".to_string();
                        needs_save = true;

                        // 强制让所有旧 Token 失效：删除 .jwt_secret 文件
                        // 这样下次获取 secret 时会生成新的，从而使旧 Token 签名无效
                        let secret_path = crate::utils::paths::get_data_path(".jwt_secret");
                        if secret_path.exists() {
                            let _ = fs::remove_file(secret_path);
                            info!("已移除旧密钥，强制以前的登录失效");
                        }
                    }
                    // 检查 B: 获取现有密码
                    else if let Some(pwd) = check.password {
                        final_password = pwd;
                    } else {
                        // 既没 hash 也没 password，异常
                        // 我们将重置为 12345678 以修复它
                        final_password = "12345678".to_string();
                        needs_save = true;
                    }

                    // 检查 C: 是否包含 jwt_secret (需要清理)
                    if check.jwt_secret.is_some() {
                        info!("检测到配置文件包含 jwt_secret，正在移除...");
                        needs_save = true;
                    }

                    // 构造新配置
                    let new_config = AppConfig {
                        username: check.username,
                        password: final_password.clone(),
                    };

                    // 如果需要，执行保存 (清理字段)
                    if needs_save {
                        if let Ok(yaml) = serde_yaml::to_string(&new_config) {
                            let content_to_write = format!("{}\n{}", CONFIG_HEADER, yaml);
                            if let Err(e) = fs::write(file_path, content_to_write) {
                                eprintln!("警告: 无法保存迁移后的配置: {}", e);
                            } else {
                                info!("配置已迁移并清理 (已移除 jwt_secret/password_hash)");
                            }
                        }
                    }

                    Some(new_config)
                } else {
                    tracing::warn!("无法解析配置文件，将尝试重新初始化");
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // 3. 如果未获取到配置 (首次运行或解析失败)，尝试从环境变量初始化
        if config.is_none() {
            let env_username = std::env::var("ADMIN_USERNAME").ok();
            let env_password = std::env::var("ADMIN_PASSWORD").ok();

            if let (Some(u), Some(p)) = (env_username, env_password) {
                info!("检测到环境变量配置，自动初始化...");

                let new_config = AppConfig {
                    username: u,
                    password: p,
                };

                let yaml = serde_yaml::to_string(&new_config).unwrap_or_default();
                let content = format!("{}\n{}", CONFIG_HEADER, yaml);
                let _ = fs::write(file_path, content);

                config = Some(new_config);
            }
        }

        Self {
            config: Arc::new(RwLock::new(config)),
            jwt_secret,
            file_path: file_path.to_string(),
        }
    }

    pub fn get(&self) -> Option<AppConfig> {
        self.config.read().unwrap().clone()
    }

    // 获取 JWT Secret
    pub fn get_jwt_secret(&self) -> String {
        self.jwt_secret.clone()
    }

    pub fn is_initialized(&self) -> bool {
        self.config.read().unwrap().is_some()
    }

    pub fn save(&self, username: String, password: String) -> Result<()> {
        let new_config = AppConfig { username, password };

        let yaml = serde_yaml::to_string(&new_config)?;
        let content = format!("{}\n{}", CONFIG_HEADER, yaml);
        fs::write(&self.file_path, content)?;

        *self.config.write().unwrap() = Some(new_config);
        Ok(())
    }
}
