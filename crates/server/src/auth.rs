//! Registration, login, and JWT auth.

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sealed_protocol::{AuthResponse, LoginRequest, RegisterRequest};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const TOKEN_TTL_SECS: usize = 60 * 60 * 24 * 30; // 30 days

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // user_id
    pub device_id: i64,
    pub exp: usize,
}

pub fn issue_token(state: &AppState, user_id: i64, device_id: i64) -> ApiResult<String> {
    let exp = chrono::Utc::now().timestamp() as usize + TOKEN_TTL_SECS;
    let claims = Claims {
        sub: user_id,
        device_id,
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(e.into()))
}

/// Extract the authenticated user/device from the Authorization header.
pub fn authenticate(state: &AppState, headers: &HeaderMap) -> ApiResult<Claims> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| ApiError::Unauthorized)
}

fn hash_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password hashing failed: {e}")))
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<AuthResponse>)> {
    let username = req.username.trim().to_string();
    if username.is_empty() || req.password.is_empty() {
        return Err(ApiError::BadRequest(
            "username and password are required".into(),
        ));
    }
    if username.chars().count() > 32 {
        return Err(ApiError::BadRequest("username too long".into()));
    }

    // Reject invalid invitations before performing the intentionally expensive
    // password hash, then check and consume the code again atomically below.
    let invite_hash =
        crate::invites::validate_available_code(&state.db, &req.invite_code, &username).await?;
    let password_hash = hash_password(&req.password)?;
    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;
    let invite_id = crate::invites::claim_code(&mut transaction, &invite_hash, &username).await?;

    let result = sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&username)
        .bind(&password_hash)
        .execute(&mut *transaction)
        .await;

    let user_id = match result {
        Ok(r) => r.last_insert_rowid(),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(ApiError::Conflict("username already taken".into()))
        }
        Err(e) => return Err(ApiError::Internal(e.into())),
    };

    let device =
        sqlx::query("INSERT INTO devices (user_id, device_name, public_identity) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(&req.device_name)
            .bind(&req.public_identity)
            .execute(&mut *transaction)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let device_id = device.last_insert_rowid();
    sqlx::query("INSERT INTO invite_redemptions (invite_code_id, user_id) VALUES (?, ?)")
        .bind(invite_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;
    transaction
        .commit()
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

    let token = issue_token(&state, user_id, device_id)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id,
            device_id,
            username,
            token,
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let Some((user_id, password_hash)) = row else {
        return Err(ApiError::Unauthorized);
    };

    if !verify_password(&req.password, &password_hash) {
        return Err(ApiError::Unauthorized);
    }

    // Use the user's first device for the token (single-device MVP).
    let device_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM devices WHERE user_id = ? ORDER BY id LIMIT 1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let device_id = device_id.ok_or(ApiError::Unauthorized)?;
    let token = issue_token(&state, user_id, device_id)?;

    Ok(Json(AuthResponse {
        user_id,
        device_id,
        username: req.username,
        token,
    }))
}
