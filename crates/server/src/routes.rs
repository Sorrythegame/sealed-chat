//! Authenticated relay endpoints: public-key distribution, conversations,
//! and ciphertext message storage/retrieval.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use sealed_protocol::{
    AvatarUpdateRequest, ConversationInfo, ConversationListResponse, CreateConversationRequest,
    DeviceInfo, MessageListResponse, MessageRecord, ProfileInfo, ProfileUpdateRequest,
    SendMessageRequest, UserDevicesResponse, UserInfo, UserListResponse, UserLookupResponse,
};
use serde::Deserialize;

use crate::auth;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

fn b64_decode(s: &str) -> ApiResult<Vec<u8>> {
    B64.decode(s)
        .map_err(|_| ApiError::BadRequest("invalid base64".into()))
}

fn b64_encode(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

/// GET /api/users — list all users except the caller (everyone is a friend).
pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<UserListResponse>> {
    let claims = auth::authenticate(&state, &headers)?;

    let users: Vec<UserInfo> = sqlx::query_as::<_, (i64, String, Option<String>, String)>(
        "SELECT id, username, avatar, bio FROM users WHERE id != ? ORDER BY id",
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .into_iter()
    .map(|(user_id, username, avatar, bio)| UserInfo {
        user_id,
        username,
        avatar,
        bio,
    })
    .collect();

    Ok(Json(UserListResponse { users }))
}

/// GET /api/users/by-name/{username} — resolve a username to a user id.
pub async fn get_user_by_name(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> ApiResult<Json<UserLookupResponse>> {
    auth::authenticate(&state, &headers)?;

    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, username FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let Some((user_id, username)) = row else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(UserLookupResponse { user_id, username }))
}

/// GET /api/users/{id}/devices — public keys for a user.
pub async fn get_user_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> ApiResult<Json<UserDevicesResponse>> {
    auth::authenticate(&state, &headers)?;

    let devices: Vec<DeviceInfo> = sqlx::query_as::<_, (i64, i64, String, String)>(
        "SELECT id, user_id, device_name, public_identity FROM devices WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .into_iter()
    .map(
        |(device_id, user_id, device_name, public_identity)| DeviceInfo {
            device_id,
            user_id,
            device_name,
            public_identity,
        },
    )
    .collect();

    Ok(Json(UserDevicesResponse { devices }))
}

/// POST /api/conversations — create a 1:1 conversation (or return existing).
pub async fn create_conversation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateConversationRequest>,
) -> ApiResult<Json<ConversationInfo>> {
    let claims = auth::authenticate(&state, &headers)?;
    let me = claims.sub;

    if req.peer_user_id == me {
        return Err(ApiError::BadRequest("cannot chat with yourself".into()));
    }

    // Peer must exist.
    let peer: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT username, avatar FROM users WHERE id = ?")
            .bind(req.peer_user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
    let Some((peer_username, peer_avatar)) = peer else {
        return Err(ApiError::NotFound);
    };

    // Reuse an existing 1:1 conversation if present.
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT my.conversation_id
         FROM conversation_members my
         JOIN conversation_members peer
           ON peer.conversation_id = my.conversation_id AND peer.user_id = ?
         WHERE my.user_id = ?",
    )
    .bind(req.peer_user_id)
    .bind(me)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if let Some(conversation_id) = existing {
        let info =
            load_conversation(&state, conversation_id, me, peer_username, req.peer_user_id).await?;
        return Ok(Json(info));
    }

    let created_at = chrono::Utc::now().to_rfc3339();
    let conv = sqlx::query("INSERT INTO conversations DEFAULT VALUES")
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conversation_id = conv.last_insert_rowid();

    sqlx::query("INSERT INTO conversation_members (conversation_id, user_id, ephemeral_pub) VALUES (?, ?, ?)")
        .bind(conversation_id)
        .bind(me)
        .bind(&req.ephemeral_pub)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query("INSERT INTO conversation_members (conversation_id, user_id, ephemeral_pub) VALUES (?, ?, NULL)")
        .bind(conversation_id)
        .bind(req.peer_user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(ConversationInfo {
        conversation_id,
        peer_user_id: req.peer_user_id,
        peer_username,
        ephemeral_pub: None,
        peer_avatar,
        created_at,
    }))
}

async fn load_conversation(
    state: &AppState,
    conversation_id: i64,
    me: i64,
    peer_username: String,
    peer_user_id: i64,
) -> ApiResult<ConversationInfo> {
    // The ephemeral pub that matters to `me` is the peer's (present only if the
    // peer was the initiator).
    let peer_eph: Option<String> = sqlx::query_scalar(
        "SELECT ephemeral_pub FROM conversation_members WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(peer_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .flatten();

    let created_at: String =
        sqlx::query_scalar("SELECT created_at FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let peer_avatar: Option<String> = sqlx::query_scalar("SELECT avatar FROM users WHERE id = ?")
        .bind(peer_user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .flatten();

    let _ = me;
    Ok(ConversationInfo {
        conversation_id,
        peer_user_id,
        peer_username,
        ephemeral_pub: peer_eph,
        peer_avatar,
        created_at,
    })
}

/// GET /api/conversations — list the caller's conversations.
pub async fn list_conversations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<ConversationListResponse>> {
    let claims = auth::authenticate(&state, &headers)?;
    let me = claims.sub;

    let rows = sqlx::query_as::<_, (i64, i64, String, Option<String>, Option<String>, String)>(
        "SELECT c.id, peer.user_id, u.username, peer.ephemeral_pub, u.avatar, c.created_at
         FROM conversation_members my
         JOIN conversation_members peer
           ON peer.conversation_id = my.conversation_id AND peer.user_id != my.user_id
         JOIN conversations c ON c.id = my.conversation_id
         JOIN users u ON u.id = peer.user_id
         WHERE my.user_id = ?
         ORDER BY c.id DESC",
    )
    .bind(me)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let conversations = rows
        .into_iter()
        .map(
            |(
                conversation_id,
                peer_user_id,
                peer_username,
                ephemeral_pub,
                peer_avatar,
                created_at,
            )| {
                ConversationInfo {
                    conversation_id,
                    peer_user_id,
                    peer_username,
                    ephemeral_pub,
                    peer_avatar,
                    created_at,
                }
            },
        )
        .collect();

    Ok(Json(ConversationListResponse { conversations }))
}

/// POST /api/conversations/{id}/messages — store an encrypted message.
pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<i64>,
    Json(req): Json<SendMessageRequest>,
) -> ApiResult<Json<MessageRecord>> {
    let claims = auth::authenticate(&state, &headers)?;

    let is_member: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM conversation_members WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if is_member.is_none() {
        return Err(ApiError::NotFound);
    }

    let ciphertext = b64_decode(&req.ciphertext)?;
    let nonce = b64_decode(&req.nonce)?;

    let created_at = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO messages (conversation_id, sender_device_id, ciphertext, nonce, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(conversation_id)
    .bind(claims.device_id)
    .bind(&ciphertext)
    .bind(&nonce)
    .bind(&created_at)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let message_id = result.last_insert_rowid();

    let record = MessageRecord {
        message_id,
        conversation_id,
        sender_device_id: claims.device_id,
        ciphertext: req.ciphertext,
        nonce: req.nonce,
        created_at,
    };

    // Push the ciphertext to the peer if they are online.
    let peer: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM conversation_members WHERE conversation_id = ? AND user_id != ?",
    )
    .bind(conversation_id)
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .flatten();

    if let Some(peer) = peer {
        if let Ok(json) = serde_json::to_string(&record) {
            state.connections.send(peer, json);
        }
    }

    Ok(Json(record))
}

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub after: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /api/conversations/{id}/messages?after=&limit= — retrieve ciphertext messages.
pub async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conversation_id): Path<i64>,
    Query(query): Query<MessageQuery>,
) -> ApiResult<Json<MessageListResponse>> {
    let claims = auth::authenticate(&state, &headers)?;

    let is_member: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM conversation_members WHERE conversation_id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if is_member.is_none() {
        return Err(ApiError::NotFound);
    }

    let after = query.after.unwrap_or(0);
    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let rows = sqlx::query_as::<_, (i64, i64, i64, Vec<u8>, Vec<u8>, String)>(
        "SELECT id, conversation_id, sender_device_id, ciphertext, nonce, created_at
         FROM messages WHERE conversation_id = ? AND id > ? ORDER BY id ASC LIMIT ?",
    )
    .bind(conversation_id)
    .bind(after)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let messages = rows
        .into_iter()
        .map(
            |(message_id, conversation_id, sender_device_id, ciphertext, nonce, created_at)| {
                MessageRecord {
                    message_id,
                    conversation_id,
                    sender_device_id,
                    ciphertext: b64_encode(&ciphertext),
                    nonce: b64_encode(&nonce),
                    created_at,
                }
            },
        )
        .collect();

    Ok(Json(MessageListResponse { messages }))
}

fn random_attachment_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// POST /api/attachments — store an encrypted image blob, returning its id.
pub async fn upload_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<sealed_protocol::AttachmentUploadRequest>,
) -> ApiResult<Json<sealed_protocol::AttachmentUploadResponse>> {
    let claims = auth::authenticate(&state, &headers)?;

    let ciphertext = b64_decode(&req.ciphertext)?;
    let nonce = b64_decode(&req.nonce)?;
    let id = random_attachment_id();

    sqlx::query(
        "INSERT INTO attachments (id, uploader_device_id, ciphertext, nonce, mime_type, size) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(claims.device_id)
    .bind(&ciphertext)
    .bind(&nonce)
    .bind(&req.mime_type)
    .bind(req.size as i64)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(sealed_protocol::AttachmentUploadResponse {
        attachment_id: id,
    }))
}

/// POST /api/me/avatar — update the caller's avatar (public profile data).
pub async fn update_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AvatarUpdateRequest>,
) -> ApiResult<axum::http::StatusCode> {
    let claims = auth::authenticate(&state, &headers)?;
    validate_avatar(&req.avatar)?;
    sqlx::query("UPDATE users SET avatar = ? WHERE id = ?")
        .bind(&req.avatar)
        .bind(claims.sub)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// GET /api/me — return the caller's public profile.
pub async fn get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<ProfileInfo>> {
    let claims = auth::authenticate(&state, &headers)?;
    let profile: Option<(i64, String, Option<String>, String)> =
        sqlx::query_as("SELECT id, username, avatar, bio FROM users WHERE id = ?")
            .bind(claims.sub)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let Some((user_id, username, avatar, bio)) = profile else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(ProfileInfo {
        user_id,
        username,
        avatar,
        bio,
    }))
}

/// PATCH /api/me — update one or more public profile fields.
pub async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ProfileUpdateRequest>,
) -> ApiResult<axum::http::StatusCode> {
    let claims = auth::authenticate(&state, &headers)?;
    if req.avatar.is_none() && req.bio.is_none() {
        return Err(ApiError::BadRequest("no profile fields supplied".into()));
    }

    let avatar = req.avatar;
    if let Some(value) = avatar.as_deref() {
        validate_avatar(value)?;
    }
    let bio = req.bio.map(normalize_bio).transpose()?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    if let Some(value) = avatar {
        sqlx::query("UPDATE users SET avatar = ? WHERE id = ?")
            .bind(value)
            .bind(claims.sub)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
    }
    if let Some(value) = bio {
        sqlx::query("UPDATE users SET bio = ? WHERE id = ?")
            .bind(value)
            .bind(claims.sub)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
    }
    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn validate_avatar(avatar: &str) -> ApiResult<()> {
    if avatar.len() > 512 * 1024 {
        return Err(ApiError::BadRequest("avatar too large".into()));
    }
    Ok(())
}

fn normalize_bio(bio: String) -> ApiResult<String> {
    let bio = bio.trim().to_string();
    if bio.chars().count() > 120 {
        return Err(ApiError::BadRequest("bio too long".into()));
    }
    Ok(bio)
}

/// GET /api/attachments/{id} — download an encrypted image blob.
pub async fn download_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<sealed_protocol::AttachmentDownloadResponse>> {
    auth::authenticate(&state, &headers)?;

    let row: Option<(Vec<u8>, Vec<u8>, String, i64)> =
        sqlx::query_as("SELECT ciphertext, nonce, mime_type, size FROM attachments WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    let Some((ciphertext, nonce, mime_type, size)) = row else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(sealed_protocol::AttachmentDownloadResponse {
        ciphertext: b64_encode(&ciphertext),
        nonce: b64_encode(&nonce),
        mime_type,
        size: size as u64,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::AUTHORIZATION;
    use axum::http::HeaderValue;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::state::Connections;

    async fn profile_test_state() -> AppState {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory database");
        sqlx::query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                avatar TEXT,
                bio TEXT NOT NULL DEFAULT ''
            )",
        )
        .execute(&pool)
        .await
        .expect("create users table");
        sqlx::query(
            "INSERT INTO users (username, password_hash, avatar, bio)
             VALUES ('alice', 'hash', NULL, '')",
        )
        .execute(&pool)
        .await
        .expect("insert test user");

        AppState {
            db: pool,
            jwt_secret: "profile-test-secret".to_string(),
            connections: Connections::default(),
        }
    }

    fn authenticated_headers(state: &AppState) -> HeaderMap {
        let token = auth::issue_token(state, 1, 1).expect("issue test token");
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).expect("valid header"),
        );
        headers
    }

    #[tokio::test]
    async fn profile_requires_authentication() {
        let state = profile_test_state().await;
        let result = get_profile(State(state), HeaderMap::new()).await;
        assert!(matches!(result, Err(ApiError::Unauthorized)));
    }

    #[tokio::test]
    async fn profile_update_is_trimmed_and_persisted() {
        let state = profile_test_state().await;
        let headers = authenticated_headers(&state);

        let status = update_profile(
            State(state.clone()),
            headers.clone(),
            Json(ProfileUpdateRequest {
                avatar: Some("data:image/png;base64,dGVzdA==".to_string()),
                bio: Some("  加密系统工程师  ".to_string()),
            }),
        )
        .await
        .expect("update profile");
        assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

        let Json(profile) = get_profile(State(state), headers)
            .await
            .expect("load updated profile");
        assert_eq!(profile.username, "alice");
        assert_eq!(profile.bio, "加密系统工程师");
        assert_eq!(
            profile.avatar.as_deref(),
            Some("data:image/png;base64,dGVzdA==")
        );
    }

    #[tokio::test]
    async fn profile_rejects_bio_over_120_characters() {
        let state = profile_test_state().await;
        let headers = authenticated_headers(&state);
        let result = update_profile(
            State(state),
            headers,
            Json(ProfileUpdateRequest {
                avatar: None,
                bio: Some("字".repeat(121)),
            }),
        )
        .await;

        assert!(matches!(result, Err(ApiError::BadRequest(message)) if message == "bio too long"));
    }
}
