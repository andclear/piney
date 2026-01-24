use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs;

use tracing::info;

/// 获取 JWT Secret
/// 逻辑：
/// 1. 检查数据目录下是否存在 .jwt_secret 文件
/// 2. 如果存在，直接读取
/// 3. 如果不存在，生成一个随机 32 位字符串，写入文件并返回
pub fn get_jwt_secret() -> String {
    let secret_path = crate::utils::paths::get_data_path(".jwt_secret");

    if secret_path.exists() {
        match fs::read_to_string(&secret_path) {
            Ok(s) => {
                let trimmed = s.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
            Err(e) => {
                tracing::warn!("无法读取 JWT Secret 文件: {}, 将重新生成", e);
            }
        }
    }

    // 生成新密钥
    let secret: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    if let Err(e) = fs::write(&secret_path, &secret) {
        tracing::error!("无法写入 JWT Secret 文件: {}", e);
    } else {
        info!("已生成新的 JWT Secret 并保存至 {:?}", secret_path);
    }

    secret
}
