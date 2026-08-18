//! HTTP API request/response DTOs. Binary fields (keys, ciphertext, nonces)
//! are base64-encoded strings over the wire.

use serde::{Deserialize, Serialize};

// ---- Auth ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub invite_code: String,
    pub device_name: String,
    /// Public identity in wire format: "identity_b64.signing_b64".
    pub public_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCreateResponse {
    /// Plaintext is returned once and is never persisted by the server.
    pub code: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    /// Optional for protocol compatibility. Current desktop clients send both
    /// fields so a pre-provisioned account can bind its first device at login.
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub public_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user_id: i64,
    pub device_id: i64,
    pub username: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLookupResponse {
    pub user_id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: i64,
    pub username: String,
    pub avatar: Option<String>,
    pub bio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListResponse {
    pub users: Vec<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub user_id: i64,
    pub username: String,
    pub avatar: Option<String>,
    pub bio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdateRequest {
    pub avatar: Option<String>,
    pub bio: Option<String>,
}

// ---- Devices / public keys ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: i64,
    pub user_id: i64,
    pub device_name: String,
    pub public_identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDevicesResponse {
    pub devices: Vec<DeviceInfo>,
}

// ---- Conversations (1:1) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub peer_user_id: i64,
    /// Ephemeral public key (base64) for session-key derivation.
    pub ephemeral_pub: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInfo {
    pub conversation_id: i64,
    pub peer_user_id: i64,
    pub peer_username: String,
    /// Ephemeral public key published by the initiator (base64), empty for the
    /// initiator themselves.
    pub ephemeral_pub: Option<String>,
    /// Peer's avatar (public profile data), a base64 data URL.
    pub peer_avatar: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarUpdateRequest {
    /// Base64 data URL of the new avatar (public profile data).
    pub avatar: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationInfo>,
}

// ---- Messages ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// AES-GCM ciphertext (base64).
    pub ciphertext: String,
    /// 12-byte nonce (base64).
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub message_id: i64,
    pub conversation_id: i64,
    pub sender_device_id: i64,
    pub ciphertext: String,
    pub nonce: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListResponse {
    pub messages: Vec<MessageRecord>,
}

// ---- Attachments ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentUploadRequest {
    /// AES-GCM ciphertext of the image (base64).
    pub ciphertext: String,
    /// Encryption nonce (base64).
    pub nonce: String,
    pub mime_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentUploadResponse {
    pub attachment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentDownloadResponse {
    pub ciphertext: String,
    pub nonce: String,
    pub mime_type: String,
    pub size: u64,
}
