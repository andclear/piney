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
    // 1. 获取 Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. 解析 Bearer token
    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let token = &auth_header[7..];

    // 3. 获取 Config 中的 secret
    let conf = match config.get() {
        Some(c) => c,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 4. 验证 Token
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(conf.jwt_secret.as_bytes()),
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
