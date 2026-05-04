//! Schnorr presenter binding (BIP-340 over secp256k1).
//!
//! The presenter holds a 32-byte secret. Its public counterpart is a 32-byte x-only
//! Schnorr public key. The journal commits to `presenter_id = SHA256(PRESENTER_DOMAIN
//! || pubkey_bytes)` — never to the secret itself, so the circuit only needs to hash
//! the pubkey (no in-circuit ECC). The signature over `journal.digest()` is what
//! enforces "I (the presenter) am attaching this proof": only the secret-holder can
//! produce a signature that verifies under the pubkey committed in the journal.

use crate::{derive_presenter_id, Digest32};
use k256::schnorr::{
    signature::{Signer, Verifier},
    Signature, SigningKey, VerifyingKey,
};
use std::fmt;

pub const PRESENTER_SIGNATURE_LEN: usize = 64;
pub const PRESENTER_PUBKEY_LEN: usize = 32;
pub const PRESENTER_SECRET_LEN: usize = 32;

#[derive(Clone, Eq, PartialEq)]
pub struct PresenterSecret([u8; PRESENTER_SECRET_LEN]);

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PresenterPubkey([u8; PRESENTER_PUBKEY_LEN]);

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PresenterSignature([u8; PRESENTER_SIGNATURE_LEN]);

#[derive(Debug)]
pub enum PresenterError {
    InvalidSecret,
    InvalidPubkey,
    InvalidSignature,
    SignFailed(String),
    VerifyFailed,
}

impl PresenterSecret {
    pub fn new(bytes: [u8; PRESENTER_SECRET_LEN]) -> Result<Self, PresenterError> {
        // Eagerly validate that the bytes form a usable Schnorr signing key.
        SigningKey::from_bytes(&bytes).map_err(|_| PresenterError::InvalidSecret)?;
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; PRESENTER_SECRET_LEN] {
        &self.0
    }

    pub fn pubkey(&self) -> PresenterPubkey {
        let signing_key = SigningKey::from_bytes(&self.0)
            .expect("PresenterSecret bytes were validated at construction");
        let verifying_key: VerifyingKey = *signing_key.verifying_key();
        let mut out = [0u8; PRESENTER_PUBKEY_LEN];
        out.copy_from_slice(&verifying_key.to_bytes());
        PresenterPubkey(out)
    }

    pub fn sign_journal_digest(&self, digest: &Digest32) -> PresenterSignature {
        let signing_key = SigningKey::from_bytes(&self.0)
            .expect("PresenterSecret bytes were validated at construction");
        let signature: Signature = signing_key.sign(digest.as_bytes());
        let mut out = [0u8; PRESENTER_SIGNATURE_LEN];
        out.copy_from_slice(&signature.to_bytes());
        PresenterSignature(out)
    }
}

impl PresenterPubkey {
    pub fn new(bytes: [u8; PRESENTER_PUBKEY_LEN]) -> Result<Self, PresenterError> {
        VerifyingKey::from_bytes(&bytes).map_err(|_| PresenterError::InvalidPubkey)?;
        Ok(Self(bytes))
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, PresenterError> {
        if bytes.len() != PRESENTER_PUBKEY_LEN {
            return Err(PresenterError::InvalidPubkey);
        }
        let mut out = [0u8; PRESENTER_PUBKEY_LEN];
        out.copy_from_slice(bytes);
        Self::new(out)
    }

    pub fn as_bytes(&self) -> &[u8; PRESENTER_PUBKEY_LEN] {
        &self.0
    }

    pub fn presenter_id(&self) -> Digest32 {
        derive_presenter_id(self)
    }

    pub fn verify_journal_digest(
        &self,
        digest: &Digest32,
        signature: &PresenterSignature,
    ) -> Result<(), PresenterError> {
        let verifying_key =
            VerifyingKey::from_bytes(&self.0).map_err(|_| PresenterError::InvalidPubkey)?;
        let parsed = Signature::try_from(signature.as_bytes().as_slice())
            .map_err(|_| PresenterError::InvalidSignature)?;
        verifying_key
            .verify(digest.as_bytes(), &parsed)
            .map_err(|_| PresenterError::VerifyFailed)
    }
}

impl PresenterSignature {
    pub fn new(bytes: [u8; PRESENTER_SIGNATURE_LEN]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, PresenterError> {
        if bytes.len() != PRESENTER_SIGNATURE_LEN {
            return Err(PresenterError::InvalidSignature);
        }
        let mut out = [0u8; PRESENTER_SIGNATURE_LEN];
        out.copy_from_slice(bytes);
        Ok(Self(out))
    }

    pub fn as_bytes(&self) -> &[u8; PRESENTER_SIGNATURE_LEN] {
        &self.0
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl fmt::Debug for PresenterSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PresenterSecret")
            .field("bytes", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for PresenterPubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PresenterPubkey({})", hex::encode(self.0))
    }
}

impl fmt::Debug for PresenterSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PresenterSignature({})", hex::encode(self.0))
    }
}

impl fmt::Display for PresenterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSecret => f.write_str("invalid Schnorr secret bytes"),
            Self::InvalidPubkey => f.write_str("invalid Schnorr x-only pubkey bytes"),
            Self::InvalidSignature => f.write_str("invalid Schnorr signature encoding"),
            Self::SignFailed(detail) => write!(f, "Schnorr signing failed: {detail}"),
            Self::VerifyFailed => f.write_str("Schnorr signature verification failed"),
        }
    }
}

impl std::error::Error for PresenterError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_secret_bytes() -> [u8; 32] {
        // A valid secp256k1 scalar < n. Pick a small constant for stability.
        let mut bytes = [0u8; 32];
        bytes[31] = 7;
        bytes
    }

    #[test]
    fn rejects_invalid_secret_zero() {
        assert!(PresenterSecret::new([0u8; 32]).is_err());
    }

    #[test]
    fn pubkey_derivation_is_deterministic() {
        let secret = PresenterSecret::new(valid_secret_bytes()).unwrap();
        let pk1 = secret.pubkey();
        let pk2 = secret.pubkey();
        assert_eq!(pk1, pk2);
    }

    #[test]
    fn presenter_id_is_hash_of_pubkey_bytes() {
        let secret = PresenterSecret::new(valid_secret_bytes()).unwrap();
        let pubkey = secret.pubkey();
        let id_via_pubkey = pubkey.presenter_id();
        let id_via_helper = derive_presenter_id(&pubkey);
        assert_eq!(id_via_pubkey, id_via_helper);
    }

    #[test]
    fn sign_then_verify_succeeds_for_same_digest() {
        let secret = PresenterSecret::new(valid_secret_bytes()).unwrap();
        let pubkey = secret.pubkey();
        let digest = Digest32([0xab; 32]);
        let sig = secret.sign_journal_digest(&digest);
        pubkey.verify_journal_digest(&digest, &sig).unwrap();
    }

    #[test]
    fn verify_fails_on_tampered_digest() {
        let secret = PresenterSecret::new(valid_secret_bytes()).unwrap();
        let pubkey = secret.pubkey();
        let digest = Digest32([0xab; 32]);
        let sig = secret.sign_journal_digest(&digest);

        let tampered = Digest32([0xac; 32]);
        assert!(pubkey.verify_journal_digest(&tampered, &sig).is_err());
    }

    #[test]
    fn verify_fails_with_wrong_pubkey() {
        let secret_a = PresenterSecret::new(valid_secret_bytes()).unwrap();
        let mut other_bytes = [0u8; 32];
        other_bytes[31] = 9;
        let secret_b = PresenterSecret::new(other_bytes).unwrap();

        let digest = Digest32([0xab; 32]);
        let sig = secret_a.sign_journal_digest(&digest);

        let pubkey_b = secret_b.pubkey();
        assert!(pubkey_b.verify_journal_digest(&digest, &sig).is_err());
    }
}
