use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use tracing::warn;

use crate::{auth::Claims, config::ConfigState};

pub async fn auth(
    State(config): State<ConfigState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // 1. 获取 Authorization header 或 query param
    let mut token: Option<String> = None;

    // A. 尝试 Header
    if let Some(auth_layout) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_layout.to_str() {
            if auth_str.starts_with("Bearer ") {
                token = Some(auth_str[7..].to_string());
            }
        }
    }

    // B. 如果 Header 没有，尝试 Query Param (用于文件下载等浏览器原生请求)
    if token.is_none() {
        if let Some(query) = req.uri().query() {
            if let Ok(params) =
                serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(query)
            {
                if let Some(t) = params.get("token") {
                    token = Some(t.clone());
                }
            }
        }
    }

    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;

    // 3. 获取 Config 中的 secret (确保已初始化)
    if !config.is_initialized() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 4. 验证 Token
    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(config.get_jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        warn!("Token validation failed: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // 5. 将 claims 放入 request extensions (可选，方便后续 handle 使用)
    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}
