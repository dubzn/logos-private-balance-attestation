use crate::{ContextBindingParams, Digest32};
use sha2::{Digest, Sha256};

pub const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
pub const NULLIFIER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/nullifier";
pub const PRESENTER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/presenter";
pub const JOURNAL_DOMAIN: &[u8] = b"logos-balance-attestation/v1/journal";

pub fn derive_context_id(params: &ContextBindingParams) -> Digest32 {
    hash_segments(&[
        CONTEXT_DOMAIN,
        params.chain_id.as_bytes(),
        params.circuit_image_id.as_bytes(),
        params.verifier_id.as_bytes(),
        params.gate_id.as_bytes(),
        &params.threshold.to_le_bytes(),
    ])
}

pub fn derive_context_nullifier(
    npk: &Digest32,
    context_id: &Digest32,
    presenter_id: &Digest32,
) -> Digest32 {
    hash_segments(&[
        NULLIFIER_DOMAIN,
        npk.as_bytes(),
        context_id.as_bytes(),
        presenter_id.as_bytes(),
    ])
}

pub fn derive_presenter_id(presenter_secret: &Digest32) -> Digest32 {
    hash_segments(&[PRESENTER_DOMAIN, presenter_secret.as_bytes()])
}

pub fn hash_segments(segments: &[&[u8]]) -> Digest32 {
    let mut hasher = Sha256::new();
    for segment in segments {
        hasher.update((segment.len() as u64).to_le_bytes());
        hasher.update(segment);
    }

    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&hasher.finalize());
    Digest32(digest)
}

pub fn sha256_bytes(bytes: &[u8]) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&hasher.finalize());
    Digest32(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_segments_is_length_delimited() {
        let direct = hash_segments(&[b"ab", b"c"]);
        let ambiguous = hash_segments(&[b"a", b"bc"]);
        assert_ne!(direct, ambiguous);
    }
}
