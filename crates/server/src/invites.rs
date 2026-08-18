//! Administrator-issued, one-time registration invitations.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{Duration, Utc};
use rand::Rng;
use sealed_protocol::InviteCreateResponse;
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::auth;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const INVITE_ADMIN_USERNAME: &str = "wangxin";
const INVITE_VALID_DAYS: i64 = 30;
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const CODE_RANDOM_LEN: usize = 20;

pub async fn create_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<(StatusCode, Json<InviteCreateResponse>)> {
    let claims = auth::authenticate(&state, &headers)?;
    let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
        .bind(claims.sub)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;

    if username.as_deref() != Some(INVITE_ADMIN_USERNAME) {
        return Err(ApiError::Forbidden);
    }

    let expires_at = (Utc::now() + Duration::days(INVITE_VALID_DAYS)).to_rfc3339();
    for _ in 0..5 {
        let code = generate_code();
        let code_hash = hash_invite_code(&code)?;
        let result = sqlx::query(
            "INSERT INTO invite_codes
             (code_hash, code_prefix, created_by_user_id, max_uses, expires_at)
             VALUES (?, ?, ?, 1, ?)",
        )
        .bind(code_hash)
        .bind(&code[..9])
        .bind(claims.sub)
        .bind(&expires_at)
        .execute(&state.db)
        .await;

        match result {
            Ok(_) => {
                return Ok((
                    StatusCode::CREATED,
                    Json(InviteCreateResponse { code, expires_at }),
                ));
            }
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => continue,
            Err(error) => return Err(ApiError::Internal(error.into())),
        }
    }

    Err(ApiError::Internal(anyhow::anyhow!(
        "failed to generate a unique invite code"
    )))
}

pub async fn validate_available_code(
    pool: &SqlitePool,
    code: &str,
    username: &str,
) -> ApiResult<String> {
    let code_hash = hash_invite_code(code)?;
    let now = Utc::now().to_rfc3339();
    let available: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM invite_codes
            WHERE code_hash = ?
              AND disabled = 0
              AND used_count < max_uses
              AND expires_at > ?
              AND (restricted_username IS NULL OR restricted_username = ?)
        )",
    )
    .bind(&code_hash)
    .bind(now)
    .bind(username)
    .fetch_one(pool)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?;

    if !available {
        return Err(invalid_code());
    }
    Ok(code_hash)
}

pub async fn claim_code(
    transaction: &mut Transaction<'_, Sqlite>,
    code_hash: &str,
    username: &str,
) -> ApiResult<i64> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE invite_codes
         SET used_count = used_count + 1
         WHERE code_hash = ?
           AND disabled = 0
           AND used_count < max_uses
           AND expires_at > ?
           AND (restricted_username IS NULL OR restricted_username = ?)",
    )
    .bind(code_hash)
    .bind(now)
    .bind(username)
    .execute(&mut **transaction)
    .await
    .map_err(|error| ApiError::Internal(error.into()))?;

    if result.rows_affected() != 1 {
        return Err(invalid_code());
    }

    sqlx::query_scalar("SELECT id FROM invite_codes WHERE code_hash = ?")
        .bind(code_hash)
        .fetch_one(&mut **transaction)
        .await
        .map_err(|error| ApiError::Internal(error.into()))
}

pub(crate) fn hash_invite_code(code: &str) -> ApiResult<String> {
    let normalized = normalize_code(code)?;
    let digest = Sha256::digest(normalized.as_bytes());
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn normalize_code(code: &str) -> ApiResult<String> {
    let mut normalized = String::with_capacity(4 + CODE_RANDOM_LEN);
    for character in code.trim().chars() {
        if character == '-' || character.is_ascii_whitespace() {
            continue;
        }
        if !character.is_ascii_alphanumeric() {
            return Err(invalid_code());
        }
        normalized.push(character.to_ascii_uppercase());
    }

    if normalized.len() != 4 + CODE_RANDOM_LEN || !normalized.starts_with("JSJL") {
        return Err(invalid_code());
    }
    Ok(normalized)
}

fn generate_code() -> String {
    let mut rng = rand::rngs::OsRng;
    let random: String = (0..CODE_RANDOM_LEN)
        .map(|_| {
            let index = rng.gen_range(0..CODE_ALPHABET.len());
            CODE_ALPHABET[index] as char
        })
        .collect();
    format!(
        "JSJL-{}-{}-{}-{}",
        &random[0..5],
        &random[5..10],
        &random[10..15],
        &random[15..20]
    )
}

fn invalid_code() -> ApiError {
    ApiError::BadRequest("invite code is invalid or expired".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::AUTHORIZATION;
    use axum::http::HeaderValue;
    use sealed_protocol::RegisterRequest;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::state::Connections;

    async fn test_state() -> AppState {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory database");
        crate::db::migrate(&pool).await.expect("create schema");

        for username in ["wangxin", "alice"] {
            let user =
                sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, 'hash')")
                    .bind(username)
                    .execute(&pool)
                    .await
                    .expect("insert user");
            sqlx::query(
                "INSERT INTO devices (user_id, device_name, public_identity)
                 VALUES (?, 'test', 'identity.signing')",
            )
            .bind(user.last_insert_rowid())
            .execute(&pool)
            .await
            .expect("insert device");
        }

        AppState {
            db: pool,
            jwt_secret: "invite-test-secret".to_string(),
            connections: Connections::default(),
        }
    }

    fn headers_for(state: &AppState, user_id: i64, device_id: i64) -> HeaderMap {
        let token = auth::issue_token(state, user_id, device_id).expect("issue token");
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).expect("valid token header"),
        );
        headers
    }

    async fn generate_for_wangxin(state: &AppState) -> InviteCreateResponse {
        let (_, Json(invite)) = create_invite(State(state.clone()), headers_for(state, 1, 1))
            .await
            .expect("wangxin creates invite");
        invite
    }

    fn registration(username: &str, code: String) -> RegisterRequest {
        RegisterRequest {
            username: username.to_string(),
            password: "correct horse battery staple".to_string(),
            invite_code: code,
            device_name: "test-device".to_string(),
            public_identity: "identity.signing".to_string(),
        }
    }

    #[tokio::test]
    async fn only_wangxin_can_generate_invites() {
        let state = test_state().await;

        let (_, Json(invite)) = create_invite(State(state.clone()), headers_for(&state, 1, 1))
            .await
            .expect("wangxin is invite administrator");
        assert!(invite.code.starts_with("JSJL-"));
        assert_eq!(invite.code.len(), 28);

        let denied = create_invite(State(state.clone()), headers_for(&state, 2, 2)).await;
        assert!(matches!(denied, Err(ApiError::Forbidden)));

        let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invite_codes")
            .fetch_one(&state.db)
            .await
            .expect("count invitations");
        assert_eq!(stored_count, 1);
    }

    #[tokio::test]
    async fn registration_consumes_an_invite_once() {
        let state = test_state().await;
        let invite = generate_for_wangxin(&state).await;

        let first = auth::register(
            State(state.clone()),
            Json(registration("bob", invite.code.clone())),
        )
        .await;
        assert!(first.is_ok());

        let second = auth::register(
            State(state.clone()),
            Json(registration("charlie", invite.code)),
        )
        .await;
        assert!(
            matches!(second, Err(ApiError::BadRequest(message)) if message == "invite code is invalid or expired")
        );

        let counts: (i64, i64) = sqlx::query_as(
            "SELECT used_count,
                    (SELECT COUNT(*) FROM invite_redemptions)
             FROM invite_codes",
        )
        .fetch_one(&state.db)
        .await
        .expect("inspect redemption");
        assert_eq!(counts, (1, 1));
    }

    #[tokio::test]
    async fn failed_registration_does_not_burn_the_invite() {
        let state = test_state().await;
        let invite = generate_for_wangxin(&state).await;

        let conflict = auth::register(
            State(state.clone()),
            Json(registration("wangxin", invite.code.clone())),
        )
        .await;
        assert!(matches!(conflict, Err(ApiError::Conflict(_))));

        let retry =
            auth::register(State(state.clone()), Json(registration("bob", invite.code))).await;
        assert!(retry.is_ok());
    }

    #[tokio::test]
    async fn concurrent_registration_allows_only_one_redemption() {
        let state = test_state().await;
        let invite = generate_for_wangxin(&state).await;
        let left_state = state.clone();
        let right_state = state.clone();
        let left_code = invite.code.clone();
        let right_code = invite.code;

        let (left, right) = tokio::join!(
            auth::register(State(left_state), Json(registration("bob", left_code))),
            auth::register(
                State(right_state),
                Json(registration("charlie", right_code))
            ),
        );

        assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
        let used_count: i64 = sqlx::query_scalar("SELECT used_count FROM invite_codes")
            .fetch_one(&state.db)
            .await
            .expect("load use count");
        assert_eq!(used_count, 1);
    }

    #[tokio::test]
    async fn expired_invite_is_rejected() {
        let state = test_state().await;
        let code = "JSJL-ABCDE-FGHIJ-KLMNO-PQRST";
        let code_hash = hash_invite_code(code).expect("hash invite");
        sqlx::query(
            "INSERT INTO invite_codes
             (code_hash, code_prefix, created_by_user_id, max_uses, expires_at)
             VALUES (?, 'JSJL-ABCD', 1, 1, ?)",
        )
        .bind(code_hash)
        .bind((Utc::now() - Duration::days(1)).to_rfc3339())
        .execute(&state.db)
        .await
        .expect("insert expired invite");

        let result =
            auth::register(State(state), Json(registration("bob", code.to_string()))).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn restricted_invite_only_registers_its_named_user() {
        let state = test_state().await;
        let code = "JSJL-ABCDE-FGHIJ-KLMNO-PQRST";
        let code_hash = hash_invite_code(code).expect("hash invite");
        sqlx::query(
            "INSERT INTO invite_codes
             (code_hash, code_prefix, created_by_user_id, restricted_username, max_uses, expires_at)
             VALUES (?, 'JSJL-ABCD', NULL, 'bob', 1, ?)",
        )
        .bind(code_hash)
        .bind((Utc::now() + Duration::days(1)).to_rfc3339())
        .execute(&state.db)
        .await
        .expect("insert restricted invite");

        let wrong_user = auth::register(
            State(state.clone()),
            Json(registration("charlie", code.to_string())),
        )
        .await;
        assert!(matches!(wrong_user, Err(ApiError::BadRequest(_))));

        let named_user =
            auth::register(State(state), Json(registration("bob", code.to_string()))).await;
        assert!(named_user.is_ok());
    }
}
