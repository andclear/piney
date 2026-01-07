use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::info;

const CONFIG_HEADER: &str = r#"# =========================================================
# Piney 配置文件
# =========================================================
# 密码重置说明:
# 如果忘记密码，请将下面的
# password: null
#
# 改为
# password: 你的新密码
#
# 请注意，英文冒号，且后面有个空格
# 请注意，不要删除username、password_hash和jwt_secret的内容，删除了就废了
# 重启程序后，密码将自动更新，该明文也会被自动删除
# =========================================================
"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub username: String,
    pub password_hash: String,
    pub password: Option<String>, // 明文密码，用于重置，启动后会自动删除
    // Secret for JWT, generated on first run if not present?
    // For simplicity, we can generate one and store it, or just generate on startup (invalidating tokens on restart).
    // Let's store it to keep tokens valid across restarts.
    pub jwt_secret: String,
}

#[derive(Clone)]
pub struct ConfigState {
    // RwLock to allow runtime updates (reset password)
    config: Arc<RwLock<Option<AppConfig>>>,
    file_path: String,
}

impl ConfigState {
    pub fn new(file_path: &str) -> Self {
        let mut config: Option<AppConfig> = if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path).ok();
            content.and_then(|c| serde_yaml::from_str(&c).ok())
        } else {
            None
        };

        // 检查是否需要迁移（明文密码转哈希）
        if let Some(ref mut c) = config {
            if let Some(plain_password) = c.password.take() {
                info!("检测到明文密码，正在执行安全迁移...");

                let salt = SaltString::generate(&mut OsRng);
                let argon2 = Argon2::default();
                if let Ok(hash) = argon2.hash_password(plain_password.as_bytes(), &salt) {
                    c.password_hash = hash.to_string();

                    // 旋转 JWT Secret 以强制注销所有已登录用户
                    c.jwt_secret = thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(32)
                        .map(char::from)
                        .collect();

                    // 立即写回文件以不仅更新内存，还清除文件中的明文
                    if let Ok(yaml) = serde_yaml::to_string(c) {
                        let content = format!("{}\n{}", CONFIG_HEADER, yaml);
                        if let Err(e) = fs::write(file_path, content) {
                            eprintln!("警告: 无法更新配置文件以清除明文密码: {}", e);
                        } else {
                            info!("密码重置成功，明文密码已清除");
                        }
                    }
                }
            }
        }

        Self {
            config: Arc::new(RwLock::new(config)),
            file_path: file_path.to_string(),
        }
    }

    pub fn get(&self) -> Option<AppConfig> {
        self.config.read().unwrap().clone()
    }

    pub fn is_initialized(&self) -> bool {
        self.config.read().unwrap().is_some()
    }

    pub fn save(&self, username: String, password_hash: String, jwt_secret: String) -> Result<()> {
        let new_config = AppConfig {
            username,
            password_hash,
            password: None,
            jwt_secret,
        };

        let yaml = serde_yaml::to_string(&new_config)?;
        let content = format!("{}\n{}", CONFIG_HEADER, yaml);
        fs::write(&self.file_path, content)?;

        *self.config.write().unwrap() = Some(new_config);
        Ok(())
    }
}
