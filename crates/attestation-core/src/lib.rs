//! Shared types and deterministic helpers for Logos private balance attestations.
//!
//! This crate intentionally has no wallet, sequencer, RISC Zero, or LEZ runtime
//! dependency. Higher layers can use it from the prover, verifier, CLI,
//! Basecamp backend, and LEZ runners without pulling in runtime-specific code.

mod bytes;
mod error;
mod hash;
mod lez_commitment;
mod presenter;
mod serde_helpers;
mod types;

pub use bytes::{Digest32, HexBytes, HexParseError};
pub use error::{AttestationError, AttestationErrorCode};
pub use hash::{
    derive_context_id, derive_context_nullifier, derive_presentation_digest, derive_presenter_id,
    hash_segments, sha256_bytes, CONTEXT_DOMAIN, JOURNAL_DOMAIN, NULLIFIER_DOMAIN,
    PRESENTATION_DOMAIN, PRESENTER_DOMAIN,
};
pub use lez_commitment::{
    compute_lez_membership_root, derive_lez_private_account_commitment, hash_lez_commitment_leaf,
    LezMembershipProof, LezPrivateAccountCommitmentInput, LEZ_COMMITMENT_PREFIX,
};
pub use presenter::{
    PresenterError, PresenterPubkey, PresenterSecret, PresenterSignature, PRESENTER_PUBKEY_LEN,
    PRESENTER_SECRET_LEN, PRESENTER_SIGNATURE_LEN,
};
pub use types::{
    BalanceAttestationEnvelope, BalanceAttestationJournal, ContextBindingParams, ProofSystem,
    ENVELOPE_VERSION, JOURNAL_VERSION,
};
