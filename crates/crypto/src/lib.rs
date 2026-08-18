//! End-to-end encryption primitives for sealed-chat.
//!
//! Pure crypto, no I/O, no platform dependencies. Shared by the desktop client
//! (via Tauri commands). The relay server never uses the decryption paths here —
//! it only persists ciphertext.

pub mod cipher;
pub mod identity;
pub mod kdf;
pub mod session;

pub use cipher::{decrypt, encrypt, random_nonce, random_symmetric_key, SymmetricKey};
pub use identity::{IdentityKeyPair, PublicIdentity};
pub use kdf::derive_kek;
pub use session::{derive_session_key, SessionSetup};

/// 32-byte keys and 12-byte nonces used throughout AES-256-GCM.
pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed (wrong key or corrupted data)")]
    Decrypt,
    #[error("invalid key length: {0}")]
    InvalidKeyLength(usize),
    #[error("invalid encoding: {0}")]
    InvalidEncoding(String),
    #[error("argon2 error")]
    Argon2,
}
