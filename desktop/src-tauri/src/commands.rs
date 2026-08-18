//! Tauri commands exposed to the frontend.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Serialize;
use sealed_crypto::cipher::SymmetricKey;
use sealed_crypto::identity::{IdentityKeyPair, PublicIdentity};
use tauri_plugin_keyring::KeyringExt;

use crate::keys;

#[derive(Serialize)]
pub struct IdentityPayload {
    pub public_identity: String,
}

#[derive(Serialize)]
pub struct SessionInitPayload {
    pub ephemeral_pub: String,
    pub session_key: String,
}

#[derive(Serialize)]
pub struct CipherPayload {
    pub ciphertext: String,
    pub nonce: String,
}

/// Generate a fresh identity key pair, persist it in the OS keychain, and
/// return the public half to the frontend.
#[tauri::command]
pub fn generate_identity(app: tauri::AppHandle) -> Result<IdentityPayload, String> {
    let (public_wire, private_wire) = keys::generate_identity();
    let keyring = app.keyring();
    keyring
        .set_password(keys::SERVICE, keys::PRIVATE_USER, &private_wire)
        .map_err(|e| e.to_string())?;
    keyring
        .set_password(keys::SERVICE, keys::PUBLIC_USER, &public_wire)
        .map_err(|e| e.to_string())?;
    Ok(IdentityPayload {
        public_identity: public_wire,
    })
}

/// Return the persisted public identity, if any.
#[tauri::command]
pub fn load_identity(app: tauri::AppHandle) -> Result<Option<String>, String> {
    app.keyring()
        .get_password(keys::SERVICE, keys::PUBLIC_USER)
        .map_err(|e| e.to_string())
}

/// Remove the persisted identity from the keychain.
#[tauri::command]
pub fn clear_identity(app: tauri::AppHandle) -> Result<(), String> {
    let keyring = app.keyring();
    for user in [keys::PRIVATE_USER, keys::PUBLIC_USER] {
        keyring
            .delete_password(keys::SERVICE, user)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Initiator: derive a session key with the peer, returning the ephemeral
/// public key (to publish) and the session key (base64).
#[tauri::command]
pub fn initiate_session(
    app: tauri::AppHandle,
    peer_public_identity: String,
) -> Result<SessionInitPayload, String> {
    let kp = load_keypair(&app)?;
    let peer = PublicIdentity::from_wire(&peer_public_identity).map_err(|e| e.to_string())?;
    let setup = sealed_crypto::session::initiator_session(&kp.identity_secret, &peer.identity_pub);
    Ok(SessionInitPayload {
        ephemeral_pub: B64.encode(setup.ephemeral_pub.as_bytes()),
        session_key: B64.encode(setup.session_key.as_bytes()),
    })
}

/// Responder: derive the same session key from the initiator's ephemeral key.
#[tauri::command]
pub fn complete_session(
    app: tauri::AppHandle,
    peer_public_identity: String,
    peer_ephemeral_pub: String,
) -> Result<String, String> {
    let kp = load_keypair(&app)?;
    let peer = PublicIdentity::from_wire(&peer_public_identity).map_err(|e| e.to_string())?;
    let eph: [u8; 32] = B64.decode(&peer_ephemeral_pub)
        .map_err(|e| e.to_string())?
        .as_slice()
        .try_into()
        .map_err(|_| "invalid ephemeral key".to_string())?;
    let key = sealed_crypto::session::responder_session_bytes(
        &kp.identity_secret,
        &peer.identity_pub,
        &eph,
    );
    Ok(B64.encode(key.as_bytes()))
}

/// Encrypt a JSON message body with the given session key.
#[tauri::command]
pub fn encrypt_message(session_key: String, plaintext: String) -> Result<CipherPayload, String> {
    let key = parse_key(&session_key)?;
    let (ct, nonce) = sealed_crypto::encrypt(&key, plaintext.as_bytes()).map_err(|e| e.to_string())?;
    Ok(CipherPayload {
        ciphertext: B64.encode(ct),
        nonce: B64.encode(nonce),
    })
}

/// Decrypt a message with the given session key, returning the JSON body.
#[tauri::command]
pub fn decrypt_message(
    session_key: String,
    ciphertext: String,
    nonce: String,
) -> Result<String, String> {
    let key = parse_key(&session_key)?;
    let ct = B64.decode(ciphertext).map_err(|e| e.to_string())?;
    let nonce: [u8; 12] = B64.decode(nonce)
        .map_err(|e| e.to_string())?
        .as_slice()
        .try_into()
        .map_err(|_| "invalid nonce".to_string())?;
    let pt = sealed_crypto::decrypt(&key, &ct, &nonce).map_err(|e| e.to_string())?;
    String::from_utf8(pt).map_err(|e| e.to_string())
}

/// Get or create the Local Master Key (base64) used to encrypt at-rest local data.
#[tauri::command]
pub fn get_or_create_lmk(app: tauri::AppHandle) -> Result<String, String> {
    let keyring = app.keyring();
    if let Some(existing) = keyring
        .get_password(keys::SERVICE, keys::LMK_USER)
        .map_err(|e| e.to_string())?
    {
        return Ok(existing);
    }
    let lmk = sealed_crypto::cipher::random_symmetric_key();
    let b64 = B64.encode(lmk.as_bytes());
    keyring
        .set_password(keys::SERVICE, keys::LMK_USER, &b64)
        .map_err(|e| e.to_string())?;
    Ok(b64)
}

/// Persist the auth token in the OS keychain (remember-me).
#[tauri::command]
pub fn save_token(app: tauri::AppHandle, token: String) -> Result<(), String> {
    app.keyring()
        .set_password(keys::SERVICE, keys::TOKEN_USER, &token)
        .map_err(|e| e.to_string())
}

/// Load the persisted auth token, if any.
#[tauri::command]
pub fn get_token(app: tauri::AppHandle) -> Result<Option<String>, String> {
    app.keyring()
        .get_password(keys::SERVICE, keys::TOKEN_USER)
        .map_err(|e| e.to_string())
}

/// Clear the persisted auth token.
#[tauri::command]
pub fn clear_token(app: tauri::AppHandle) -> Result<(), String> {
    let keyring = app.keyring();
    match keyring.delete_password(keys::SERVICE, keys::TOKEN_USER) {
        Ok(()) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn load_keypair(app: &tauri::AppHandle) -> Result<IdentityKeyPair, String> {
    let private_wire = app
        .keyring()
        .get_password(keys::SERVICE, keys::PRIVATE_USER)
        .map_err(|e| e.to_string())?
        .ok_or("no identity found")?;
    keys::parse_keypair(&private_wire)
}

fn parse_key(b64: &str) -> Result<SymmetricKey, String> {
    let bytes = B64.decode(b64).map_err(|e| e.to_string())?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "invalid key length".to_string())?;
    Ok(SymmetricKey::from_bytes(arr))
}

#[derive(Serialize)]
pub struct AttachmentCipherPayload {
    pub ciphertext: String,
    pub key: String,
    pub nonce: String,
}

/// Encrypt an attachment (image) with a fresh random key, returning the
/// ciphertext plus the key and nonce needed to decrypt it.
#[tauri::command]
pub fn encrypt_attachment(data: Vec<u8>) -> Result<AttachmentCipherPayload, String> {
    let key = sealed_crypto::cipher::random_symmetric_key();
    let (ct, nonce) = sealed_crypto::encrypt(&key, &data).map_err(|e| e.to_string())?;
    Ok(AttachmentCipherPayload {
        ciphertext: B64.encode(ct),
        key: B64.encode(key.as_bytes()),
        nonce: B64.encode(nonce),
    })
}

/// Decrypt an attachment ciphertext using its key and nonce.
#[tauri::command]
pub fn decrypt_attachment(ciphertext: String, key: String, nonce: String) -> Result<Vec<u8>, String> {
    let key = parse_key(&key)?;
    let ct = B64.decode(ciphertext).map_err(|e| e.to_string())?;
    let nonce: [u8; 12] = B64.decode(nonce)
        .map_err(|e| e.to_string())?
        .as_slice()
        .try_into()
        .map_err(|_| "invalid nonce".to_string())?;
    sealed_crypto::decrypt(&key, &ct, &nonce).map_err(|e| e.to_string())
}

/// Capture the primary monitor and return a PNG byte buffer.
#[tauri::command]
pub fn screenshot() -> Result<Vec<u8>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let monitor = monitors.first().ok_or("no monitor found")?;
    let image = monitor.capture_image().map_err(|e| e.to_string())?;
    let mut png = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png),
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())?;
    Ok(png)
}
