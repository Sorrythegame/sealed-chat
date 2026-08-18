//! 1:1 session key agreement with forward secrecy.
//!
//! Initiator generates an ephemeral X25519 key and derives the session key from
//! two DH outputs; the responder derives the same key once it receives the
//! ephemeral public key. Ephemeral keys are dropped after setup.

use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::cipher::SymmetricKey;
use crate::{CryptoError, KEY_LEN};

/// Result of initiating a session: the ephemeral public key to send to the peer
/// and the derived session key.
pub struct SessionSetup {
    pub ephemeral_pub: XPublicKey,
    pub session_key: SymmetricKey,
}

/// Initiator side. `self_identity` and `peer_identity_pub` are the two parties'
/// long-term identity keys.
pub fn initiator_session(
    self_identity: &StaticSecret,
    peer_identity_pub: &XPublicKey,
) -> SessionSetup {
    let mut ephemeral = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let ephemeral_pub = XPublicKey::from(&ephemeral);

    let session_key = derive(
        &ephemeral.diffie_hellman(peer_identity_pub),
        &self_identity.diffie_hellman(peer_identity_pub),
    );

    ephemeral.zeroize();
    SessionSetup {
        ephemeral_pub,
        session_key,
    }
}

/// Responder side. `self_identity` is the responder's identity, `peer_identity_pub`
/// the initiator's identity, and `peer_ephemeral_pub` the ephemeral key sent by the
/// initiator.
pub fn responder_session(
    self_identity: &StaticSecret,
    peer_identity_pub: &XPublicKey,
    peer_ephemeral_pub: &XPublicKey,
) -> SymmetricKey {
    let s1 = self_identity.diffie_hellman(peer_ephemeral_pub);
    let s2 = self_identity.diffie_hellman(peer_identity_pub);
    derive(&s1, &s2)
}

/// Responder-side derivation taking the initiator's ephemeral key as raw bytes.
pub fn responder_session_bytes(
    self_identity: &StaticSecret,
    peer_identity_pub: &XPublicKey,
    peer_ephemeral_pub: &[u8; 32],
) -> SymmetricKey {
    responder_session(
        self_identity,
        peer_identity_pub,
        &XPublicKey::from(*peer_ephemeral_pub),
    )
}

/// Convenience wrapper around [`initiator_session`] that derives a fresh key
/// using an externally provided identity key pair.
pub fn derive_session_key(
    self_identity: &StaticSecret,
    peer_identity_pub: &XPublicKey,
) -> SessionSetup {
    initiator_session(self_identity, peer_identity_pub)
}

fn derive(s1: &x25519_dalek::SharedSecret, s2: &x25519_dalek::SharedSecret) -> SymmetricKey {
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(s1.as_bytes());
    ikm[32..].copy_from_slice(s2.as_bytes());

    let hk = Hkdf::<Sha256>::new(None, &ikm);
    let mut okm = [0u8; KEY_LEN];
    // HKDF can only fail if the output length is too large; 32 is fine.
    hk.expand(b"sealed-chat session key v1", &mut okm)
        .expect("hkdf expand to 32 bytes cannot fail");
    SymmetricKey(okm)
}

/// Errors from session derivation (currently only used for API completeness).
pub type SessionError = CryptoError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityKeyPair;

    #[test]
    fn both_sides_agree() {
        let a = IdentityKeyPair::generate();
        let b = IdentityKeyPair::generate();
        let a_pub = a.public();
        let b_pub = b.public();

        let setup = initiator_session(&a.identity_secret, &b_pub.identity_pub);
        let b_key = responder_session(
            &b.identity_secret,
            &a_pub.identity_pub,
            &setup.ephemeral_pub,
        );

        assert_eq!(setup.session_key.as_bytes(), b_key.as_bytes());
    }

    #[test]
    fn encrypted_message_flows_between_parties() {
        let a = IdentityKeyPair::generate();
        let b = IdentityKeyPair::generate();
        let a_pub = a.public();
        let b_pub = b.public();

        let setup = initiator_session(&a.identity_secret, &b_pub.identity_pub);
        let b_key = responder_session(
            &b.identity_secret,
            &a_pub.identity_pub,
            &setup.ephemeral_pub,
        );

        let (ct, nonce) = crate::cipher::encrypt(&setup.session_key, b"hi").unwrap();
        let pt = crate::cipher::decrypt(&b_key, &ct, &nonce).unwrap();
        assert_eq!(pt, b"hi");
    }
}
