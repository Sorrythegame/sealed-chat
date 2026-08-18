//! Key derivation: password -> key-encryption-key (KEK) via argon2id.

use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::{CryptoError, KEY_LEN};

/// Random 16-byte salt length.
pub const SALT_LEN: usize = 16;

/// Derive a 256-bit KEK from a password using argon2id.
///
/// The returned salt must be stored alongside any data encrypted with the KEK,
/// so the same KEK can be re-derived at login.
pub fn derive_kek(password: &str, salt: &[u8]) -> Result<([u8; KEY_LEN], [u8; SALT_LEN]), CryptoError> {
    let salt_arr: [u8; SALT_LEN] = salt
        .try_into()
        .map_err(|_| CryptoError::InvalidEncoding("salt must be 16 bytes".into()))?;

    let params = Params::new(64 * 1024, 3, 1, Some(KEY_LEN)).map_err(|_| CryptoError::Argon2)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut out = [0u8; KEY_LEN];
    argon
        .hash_password_into(password.as_bytes(), &salt_arr, &mut out)
        .map_err(|_| CryptoError::Argon2)?;
    Ok((out, salt_arr))
}

/// Generate a fresh random salt for [`derive_kek`].
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let salt = random_salt();
        let (a, _) = derive_kek("correct horse", &salt).unwrap();
        let (b, _) = derive_kek("correct horse", &salt).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_password_differs() {
        let salt = random_salt();
        let (a, _) = derive_kek("password1", &salt).unwrap();
        let (b, _) = derive_kek("password2", &salt).unwrap();
        assert_ne!(a, b);
    }
}
