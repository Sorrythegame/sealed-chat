//! Symmetric encryption (AES-256-GCM) with random nonces.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use zeroize::Zeroize;

use crate::{CryptoError, KEY_LEN, NONCE_LEN};

/// A 32-byte symmetric key. Implements zeroize-on-drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct SymmetricKey(pub [u8; KEY_LEN]);

impl SymmetricKey {
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl AsRef<[u8]> for SymmetricKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Generate a fresh random 256-bit key.
pub fn random_symmetric_key() -> SymmetricKey {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    SymmetricKey(key)
}

/// Generate a fresh random 96-bit nonce.
pub fn random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypt `plaintext` with AES-256-GCM, returning `(ciphertext, nonce)`.
pub fn encrypt(key: &SymmetricKey, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; NONCE_LEN]), CryptoError> {
    let nonce = random_nonce();
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| CryptoError::Encrypt)?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| CryptoError::Encrypt)?;
    Ok((ct, nonce))
}

/// Decrypt `ciphertext` with AES-256-GCM using the provided nonce.
pub fn decrypt(key: &SymmetricKey, ciphertext: &[u8], nonce: &[u8; NONCE_LEN]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| CryptoError::Decrypt)?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| CryptoError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = random_symmetric_key();
        let plaintext = b"hello sealed-chat";
        let (ct, nonce) = encrypt(&key, plaintext).unwrap();
        let pt = decrypt(&key, &ct, &nonce).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let key = random_symmetric_key();
        let other = random_symmetric_key();
        let (ct, nonce) = encrypt(&key, b"secret").unwrap();
        assert!(decrypt(&other, &ct, &nonce).is_err());
    }
}
