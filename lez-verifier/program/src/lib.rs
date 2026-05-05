//! Deployable LEZ program for the balance-attestation gate.
//!
//! Embeds the guest binary via `risc0_build::embed_methods()` at build time,
//! exposing:
//!
//! - `BALANCE_ATTESTATION_PROGRAM_ELF: &[u8]` — the RISC Zero ELF that gets
//!   shipped to a LEZ sequencer via `wallet deploy-program`.
//! - `BALANCE_ATTESTATION_PROGRAM_ID: [u32; 8]` — the LEZ `ProgramId`
//!   computed from the ELF's image id.
//!
//! See `lez-verifier/program/README.md` for the deployment story and the
//! trust-bridge between this program and the off-chain balance-attestation
//! receipt.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));
