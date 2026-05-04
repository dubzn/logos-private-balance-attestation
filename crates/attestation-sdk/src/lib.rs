//! Logos private balance attestation SDK (LP-0005).
//!
//! This is an umbrella crate: it doesn't define new logic, it re-exports the
//! user-facing surface of the constituent crates so a downstream Logos module
//! only needs one workspace dependency:
//!
//! ```toml
//! [dependencies]
//! attestation-sdk = { path = "...", features = ["off-chain", "on-chain"] }
//! ```
//!
//! ## Off-chain flow
//!
//! ```text
//! 1. PrivateAccountWitness + LezMembershipProof + PresenterSecret + AttestationPublicParams
//! 2. build_balance_attestation_witness(...) -> BalanceAttestationWitness
//! 3. prove_attestation(&witness, &params)   -> BalanceAttestationEnvelope
//! 4. verify_envelope(&envelope, &expected)  -> Result<(), VerifyError>
//! ```
//!
//! ## On-chain flow (feature = "on-chain")
//!
//! ```text
//! 1..3. Same as off-chain — produces BalanceAttestationEnvelope.
//! 4. prove_lez_gate(&envelope, &lez_gate_config) -> LezGateProof
//! 5. The LEZ on-chain program (modeled by LezGateProgram) calls
//!    LezGateProgram::admit(&proof) which verifies the outer receipt and
//!    dedups journal.context_nullifier.
//! ```
//!
//! ## Application layer (NOT included)
//!
//! - `examples/governance-gate/` — voting integration (off-chain).
//! - `examples/chat-gate/` — chat-room admission via wire-encoded envelope.
//! - The CLI `balance-attest` — `prove` and `verify` subcommands.

// ── attestation-core: shared types, hashes, journal/envelope shapes ────────────
pub use attestation_core::{
    compute_lez_membership_root, derive_context_id, derive_context_nullifier,
    derive_lez_private_account_commitment, derive_presenter_id, hash_lez_commitment_leaf,
    hash_segments, sha256_bytes, AttestationError, AttestationErrorCode,
    BalanceAttestationEnvelope, BalanceAttestationJournal, ContextBindingParams, Digest32,
    HexBytes, HexParseError, LezMembershipProof, LezPrivateAccountCommitmentInput, PresenterError,
    PresenterPubkey, PresenterSecret, PresenterSignature, ProofSystem, CONTEXT_DOMAIN,
    ENVELOPE_VERSION, JOURNAL_DOMAIN, JOURNAL_VERSION, LEZ_COMMITMENT_PREFIX, NULLIFIER_DOMAIN,
    PRESENTER_DOMAIN, PRESENTER_PUBKEY_LEN, PRESENTER_SECRET_LEN, PRESENTER_SIGNATURE_LEN,
};

// ── attestation-prover: build witness + generate envelope ─────────────────────
pub use attestation_prover::{
    balance_attestation_image_id, build_balance_attestation_witness,
    build_private_account_inspect_report, inspect_membership_proof, prove_attestation,
    redact_private_account_id, AttestationPublicParams, BalanceAttestationWitness,
    BalanceAttestationWitnessSummary, MembershipProofInspection, PresenterWitness,
    PrivateAccountInspectReport, PrivateAccountInspectSource, PrivateAccountInspectStatus,
    PrivateAccountWitness, ProveError, REDACTION_POLICY, WITNESS_REDACTION_POLICY,
};

// ── attestation-verifier: off-chain verification ──────────────────────────────
pub use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};

// ── lez-verifier: on-chain (LEZ) gate (recursion) ─────────────────────────────
#[cfg(feature = "on-chain")]
pub mod onchain {
    pub use lez_verifier::{
        balance_attestation_inner_image_id, lez_balance_gate_image_id, prove_lez_gate,
        LezGateConfig, LezGateError, LezGateJournal, LezGateProgram, LezGateProgramError,
        LezGateProof, LEZ_GATE_JOURNAL_VERSION,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umbrella_exposes_off_chain_surface() {
        // Compile-time smoke: confirm the canonical types resolve.
        let _: Digest32 = Digest32::ZERO;
        let _: AttestationErrorCode = AttestationErrorCode::InvalidReceipt;
        let _: u16 = ENVELOPE_VERSION;
        let _: u16 = JOURNAL_VERSION;
    }

    #[cfg(feature = "on-chain")]
    #[test]
    fn umbrella_exposes_on_chain_surface() {
        // Compile-time smoke for on-chain re-exports.
        let _: u16 = onchain::LEZ_GATE_JOURNAL_VERSION;
    }
}
