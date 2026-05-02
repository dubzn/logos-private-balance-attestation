//! Shared types and deterministic helpers for Logos private balance attestations.
//!
//! This crate intentionally has no wallet, sequencer, RISC Zero, or LEZ runtime
//! dependency. Higher layers can use it from the prover, verifier, CLI,
//! Basecamp backend, and LEZ runners without pulling in runtime-specific code.

mod bytes;
mod error;
mod hash;
mod serde_helpers;
mod types;

pub use bytes::{Digest32, HexBytes, HexParseError};
pub use error::{AttestationError, AttestationErrorCode};
pub use hash::{
    derive_context_id, derive_context_nullifier, derive_presenter_id, hash_segments,
    CONTEXT_DOMAIN, JOURNAL_DOMAIN, NULLIFIER_DOMAIN, PRESENTER_DOMAIN,
};
pub use types::{
    BalanceAttestationEnvelope, BalanceAttestationJournal, ContextBindingParams, ProofSystem,
    ENVELOPE_VERSION, JOURNAL_VERSION,
};
