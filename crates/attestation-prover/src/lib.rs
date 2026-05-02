//! Prover-side helpers that are safe to reuse before the full RISC Zero prover exists.
//!
//! This crate deliberately keeps the LEZ wallet dependency outside the workspace for
//! now. The local M2 script adapts `WalletCore` data into these plain Rust types, so
//! the reusable report/redaction logic can be tested without a sequencer or wallet.

mod inspect;
mod witness;

pub use inspect::{
    build_private_account_inspect_report, redact_private_account_id, MembershipProofInspection,
    PrivateAccountInspectReport, PrivateAccountInspectSource, PrivateAccountInspectStatus,
    REDACTION_POLICY,
};
pub use witness::{
    build_balance_attestation_witness, inspect_membership_proof, AttestationPublicParams,
    BalanceAttestationWitness, BalanceAttestationWitnessSummary, PresenterWitness,
    PrivateAccountWitness, WITNESS_REDACTION_POLICY,
};
