//! Identity keys: X25519 for key agreement, Ed25519 for signing.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::Signer;
use rand::rngs::OsRng;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::CryptoError;

/// An identity key pair generated once per device at registration.
pub struct IdentityKeyPair {
    /// X25519 private key (used for ECDH session derivation).
    pub identity_secret: StaticSecret,
    /// Ed25519 signing key (used to sign session setup / key bundles).
    pub signing_key: ed25519_dalek::SigningKey,
}

impl Zeroize for IdentityKeyPair {
    fn zeroize(&mut self) {
        self.identity_secret.zeroize();
        self.signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
    }
}

impl Drop for IdentityKeyPair {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Public half of an identity, safe to upload to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicIdentity {
    pub identity_pub: XPublicKey,
    pub signing_pub: ed25519_dalek::VerifyingKey,
}

impl IdentityKeyPair {
    pub fn generate() -> Self {
        let identity_secret = StaticSecret::random_from_rng(OsRng);
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        Self {
            identity_secret,
            signing_key,
        }
    }

    pub fn public(&self) -> PublicIdentity {
        PublicIdentity {
            identity_pub: XPublicKey::from(&self.identity_secret),
            signing_pub: self.signing_key.verifying_key(),
        }
    }

    /// Sign an arbitrary message with the Ed25519 signing key.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_vec()
    }

    /// Serialize the private half to 64 bytes (X25519 32B || Ed25519 32B).
    pub fn to_private_bytes(&self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(self.identity_secret.as_bytes());
        buf[32..].copy_from_slice(self.signing_key.to_bytes().as_slice());
        buf
    }

    /// Reconstruct a key pair from [`IdentityKeyPair::to_private_bytes`].
    pub fn from_private_bytes(bytes: &[u8; 64]) -> Result<Self, CryptoError> {
        let mut id = [0u8; 32];
        let mut sig = [0u8; 32];
        id.copy_from_slice(&bytes[..32]);
        sig.copy_from_slice(&bytes[32..]);
        Ok(Self {
            identity_secret: StaticSecret::from(id),
            signing_key: ed25519_dalek::SigningKey::from_bytes(&sig),
        })
    }
}

impl PublicIdentity {
    /// Encode both public keys to base64, joined by a `.` — the wire format.
    pub fn to_wire(&self) -> String {
        format!(
            "{}.{}",
            B64.encode(self.identity_pub.as_bytes()),
            B64.encode(self.signing_pub.as_bytes())
        )
    }

    /// Parse the wire format produced by [`PublicIdentity::to_wire`].
    pub fn from_wire(s: &str) -> Result<Self, CryptoError> {
        let mut parts = s.split('.');
        let (id_b64, sig_b64) = (parts.next(), parts.next());
        let (Some(id_b64), Some(sig_b64)) = (id_b64, sig_b64) else {
            return Err(CryptoError::InvalidEncoding("expected two .-separated keys".into()));
        };

        let id_bytes = B64.decode(id_b64).map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;
        let sig_bytes = B64.decode(sig_b64).map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;

        let id: [u8; 32] = id_bytes
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidEncoding("identity key must be 32 bytes".into()))?;
        let sig: [u8; 32] = sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidEncoding("signing key must be 32 bytes".into()))?;

        Ok(PublicIdentity {
            identity_pub: XPublicKey::from(id),
            signing_pub: ed25519_dalek::VerifyingKey::from_bytes(&sig)
                .map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?,
        })
    }

    /// Verify an Ed25519 signature over `message`.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        use ed25519_dalek::Verifier;
        let sig = ed25519_dalek::Signature::from_slice(signature)
            .map_err(|e| CryptoError::InvalidEncoding(e.to_string()))?;
        self.signing_pub
            .verify(message, &sig)
            .map_err(|_| CryptoError::InvalidEncoding("bad signature".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_roundtrip() {
        let kp = IdentityKeyPair::generate();
        let pubid = kp.public();
        let wire = pubid.to_wire();
        let parsed = PublicIdentity::from_wire(&wire).unwrap();
        assert_eq!(pubid, parsed);
    }

    #[test]
    fn sign_verify() {
        let kp = IdentityKeyPair::generate();
        let pubid = kp.public();
        let msg = b"session setup";
        let sig = kp.sign(msg);
        assert!(pubid.verify(msg, &sig).is_ok());
        assert!(pubid.verify(b"other", &sig).is_err());
    }
}
