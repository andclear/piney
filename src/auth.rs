use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::config::ConfigState;

// DTOs
#[derive(Deserialize)]
pub struct SetupRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    initialized: bool,
    authenticated: bool, // This will be handled by middleware essentially, but for status check we might just indicate if config exists
    username: Option<String>,
}

// Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    sub: String,
    exp: usize,
}

// Handlers
async fn get_status(State(config): State<ConfigState>) -> impl IntoResponse {
    let conf = config.get();
    let (initialized, username) = match conf {
        Some(c) => (true, Some(c.username)),
        None => (false, None),
    };

    Json(StatusResponse {
        initialized,
        authenticated: false, // Client should check this via middleware/token
        username,
    })
}

async fn setup(
    State(config): State<ConfigState>,
    Json(payload): Json<SetupRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if config.is_initialized() {
        return Err((StatusCode::BAD_REQUEST, "Already initialized".to_string()));
    }

    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    // Generate JWT secret
    let jwt_secret: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Save
    config
        .save(payload.username.clone(), password_hash, jwt_secret.clone())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Auto login (generate token)
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(90))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: payload.username,
        exp: expiration as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse { token }))
}

async fn login(
    State(config): State<ConfigState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let conf = match config.get() {
        Some(c) => c,
        None => return Err((StatusCode::UNAUTHORIZED, "Not initialized".to_string())),
    };

    // Verify username
    if conf.username != payload.username {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Verify password
    let parsed_hash = PasswordHash::new(&conf.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Generate Token
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(90))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: payload.username,
        exp: expiration as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(conf.jwt_secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse { token }))
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub current_password: String,
    pub new_username: Option<String>,
    pub new_password: Option<String>,
}

async fn update_profile(
    State(config): State<ConfigState>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut conf = match config.get() {
        Some(c) => c,
        None => return Err((StatusCode::UNAUTHORIZED, "Not initialized".to_string())),
    };

    // 1. Verify current password
    let parsed_hash = PasswordHash::new(&conf.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if Argon2::default()
        .verify_password(payload.current_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid current password".to_string(),
        ));
    }

    // 2. Update Username
    let mut needs_save = false;
    if let Some(new_name) = payload.new_username {
        if !new_name.is_empty() && new_name != conf.username {
            conf.username = new_name;
            // Rotate JWT secret to force re-login on all devices when username changes
            conf.jwt_secret = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(32)
                .map(char::from)
                .collect();
            needs_save = true;
        }
    }

    // 3. Update Password
    if let Some(new_pass) = payload.new_password {
        if !new_pass.is_empty() {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            let new_hash = argon2
                .hash_password(new_pass.as_bytes(), &salt)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .to_string();

            conf.password_hash = new_hash;
            // Rotate JWT secret to force re-login on all devices
            conf.jwt_secret = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(32)
                .map(char::from)
                .collect();
            needs_save = true;
        }
    }

    // 4. Save
    if needs_save {
        config
            .save(conf.username, conf.password_hash, conf.jwt_secret)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(Json("Profile updated"))
    } else {
        Ok(Json("No changes made"))
    }
}

pub fn router(config: ConfigState) -> Router {
    Router::new()
        .route("/status", get(get_status))
        .route("/setup", post(setup))
        .route("/login", post(login))
        .route("/profile", post(update_profile))
        .with_state(config)
}
