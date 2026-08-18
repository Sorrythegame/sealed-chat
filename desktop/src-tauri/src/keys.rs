//! Identity key generation and (de)serialization. OS-keychain persistence is
//! done in `commands.rs` via the keyring plugin's `app.keyring()` accessor.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use sealed_crypto::identity::IdentityKeyPair;

pub const SERVICE: &str = "technology-communication";
pub const PUBLIC_USER: &str = "identity-public";
pub const PRIVATE_USER: &str = "identity-private";
pub const LMK_USER: &str = "local-master-key";
pub const TOKEN_USER: &str = "auth-token";

/// Generate a fresh identity key pair, returning `(public_wire, private_wire)`.
pub fn generate_identity() -> (String, String) {
    let kp = IdentityKeyPair::generate();
    let public_wire = kp.public().to_wire();
    let private_wire = B64.encode(kp.to_private_bytes());
    (public_wire, private_wire)
}

/// Reconstruct an identity key pair from its base64 private wire string.
pub fn parse_keypair(private_wire: &str) -> Result<IdentityKeyPair, String> {
    let bytes = B64.decode(private_wire).map_err(|e| e.to_string())?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "invalid private key length".to_string())?;
    IdentityKeyPair::from_private_bytes(&arr).map_err(|e| e.to_string())
}
