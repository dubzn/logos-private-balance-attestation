//! Prover-side helpers that are safe to reuse before the full RISC Zero prover exists.
//!
//! This crate deliberately keeps the LEZ wallet dependency outside the workspace for
//! now. The local M2 script adapts `WalletCore` data into these plain Rust types, so
//! the reusable report/redaction logic can be tested without a sequencer or wallet.

mod inspect;

pub use inspect::{
    build_private_account_inspect_report, redact_private_account_id, MembershipProofInspection,
    PrivateAccountInspectReport, PrivateAccountInspectSource, PrivateAccountInspectStatus,
    REDACTION_POLICY,
};
