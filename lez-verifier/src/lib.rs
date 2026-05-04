//! Host-side wrapper for the LEZ on-chain balance-attestation gate
//! (Spike 0C / "Logos-native private execution gate" track per
//! `docs/ONCHAIN_PATH_DECISION.md`).
//!
//! `prove_lez_gate` takes an off-chain envelope (the inner balance-attestation
//! receipt + journal) and produces an outer RISC Zero receipt that the LEZ
//! on-chain program can dedup against its `(gate_context_id → set<nullifier>)`
//! state. The outer guest re-derives the gate's expected `context_id`, calls
//! `env::verify(BALANCE_ATTESTATION_ID, inner_journal)` to nest the inner
//! receipt as an assumption, and commits a small public summary.
//!
//! `LezGateProgram` is an in-memory rehearsal of the LEZ program semantics:
//! validate the outer receipt, check the committed `inner_image_id` is the
//! pinned `BALANCE_ATTESTATION_ID`, dedup the nullifier, then admit.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

use std::collections::HashSet;
use std::fmt;

use attestation_core::{
    derive_context_id, BalanceAttestationEnvelope, ContextBindingParams, Digest32,
};
use methods::BALANCE_ATTESTATION_ID;
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct LezGateError(pub String);

impl fmt::Display for LezGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LezGateError {}

/// Verifier-pinned gate parameters. The LEZ program owns this; envelopes whose
/// inner journal does not match these are rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LezGateConfig {
    pub chain_id: Digest32,
    pub verifier_id: Digest32,
    pub gate_id: Digest32,
    pub threshold: u128,
}

impl LezGateConfig {
    pub fn expected_context_id(&self) -> Digest32 {
        derive_context_id(&ContextBindingParams {
            chain_id: self.chain_id,
            circuit_image_id: balance_attestation_inner_image_id(),
            verifier_id: self.verifier_id,
            gate_id: self.gate_id,
            threshold: self.threshold,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LezGateJournal {
    pub version: u16,
    pub inner_image_id: Digest32,
    pub gate_context_id: Digest32,
    pub accepted_context_nullifier: Digest32,
    pub accepted_presenter_id: Digest32,
    pub accepted_threshold: u128,
}

pub const LEZ_GATE_JOURNAL_VERSION: u16 = 1;

/// Outer receipt produced by `prove_lez_gate`. The `receipt_bytes` are
/// serde_json-encoded for transport, matching the inner-receipt convention.
pub struct LezGateProof {
    pub journal: LezGateJournal,
    pub receipt_bytes: Vec<u8>,
}

/// Returns the compiled image id of the lez-balance-gate guest. The LEZ
/// program pins this — any outer receipt whose proof key differs is rejected.
pub fn lez_balance_gate_image_id() -> Digest32 {
    Digest32(
        risc0_zkvm::Digest::from(LEZ_BALANCE_GATE_ID)
            .as_bytes()
            .try_into()
            .expect("Digest is 32 bytes"),
    )
}

/// Returns the compiled inner image id (BALANCE_ATTESTATION_ID). Exposed
/// here so the LEZ program (and tests) can pin against the same constant
/// the lez-verifier was built with.
pub fn balance_attestation_inner_image_id() -> Digest32 {
    Digest32(
        risc0_zkvm::Digest::from(BALANCE_ATTESTATION_ID)
            .as_bytes()
            .try_into()
            .expect("Digest is 32 bytes"),
    )
}

// Mirror of the outer guest input — matches `LezGateInput` in
// lez-verifier/guest/src/bin/lez_balance_gate.rs.
#[derive(Serialize, Deserialize)]
struct GuestInput {
    inner_image_id: [u8; 32],
    inner_journal_bytes: Vec<u8>,
    expected_chain_id: [u8; 32],
    expected_verifier_id: [u8; 32],
    expected_gate_id: [u8; 32],
    expected_threshold: u128,
}

// Decode mirror for the outer journal — kept as plain arrays so the LEZ
// program's downstream comparisons are byte-for-byte.
#[derive(Deserialize)]
struct OuterJournalDecoded {
    version: u16,
    inner_image_id: [u8; 32],
    gate_context_id: [u8; 32],
    accepted_context_nullifier: [u8; 32],
    accepted_presenter_id: [u8; 32],
    accepted_threshold: u128,
}

/// Wrap an off-chain balance-attestation envelope into an LEZ-gate receipt.
///
/// The inner receipt (deserialized from `envelope.receipt`) is added as an
/// assumption so the outer guest's `env::verify` resolves. The outer journal
/// commits the gate's `(inner_image_id, gate_context_id, nullifier,
/// presenter_id, threshold)` tuple, which is what the LEZ on-chain program
/// dedups on.
pub fn prove_lez_gate(
    envelope: &BalanceAttestationEnvelope,
    gate: &LezGateConfig,
) -> Result<LezGateProof, LezGateError> {
    // Decode the inner receipt.
    let inner_receipt: Receipt = serde_json::from_slice(envelope.receipt.as_bytes())
        .map_err(|e| LezGateError(format!("inner receipt deserialize failed: {e}")))?;

    // Re-encode the inner journal in the canonical RISC Zero byte format.
    // We use the bytes the inner receipt itself committed so env::verify
    // sees the exact assumption the inner produced.
    let inner_journal_bytes = inner_receipt.journal.bytes.clone();

    let input = GuestInput {
        inner_image_id: balance_attestation_inner_image_id().0,
        inner_journal_bytes,
        expected_chain_id: gate.chain_id.0,
        expected_verifier_id: gate.verifier_id.0,
        expected_gate_id: gate.gate_id.0,
        expected_threshold: gate.threshold,
    };

    let env = ExecutorEnv::builder()
        .add_assumption(inner_receipt)
        .write(&input)
        .map_err(|e| LezGateError(format!("guest input write failed: {e}")))?
        .build()
        .map_err(|e| LezGateError(format!("executor env build failed: {e}")))?;

    let prove_info = default_prover()
        .prove(env, LEZ_BALANCE_GATE_ELF)
        .map_err(|e| LezGateError(e.to_string()))?;

    prove_info
        .receipt
        .verify(risc0_zkvm::Digest::from(LEZ_BALANCE_GATE_ID))
        .map_err(|e| LezGateError(format!("outer receipt self-verify failed: {e}")))?;

    let decoded: OuterJournalDecoded = prove_info
        .receipt
        .journal
        .decode()
        .map_err(|e| LezGateError(format!("outer journal decode failed: {e}")))?;

    if decoded.version != LEZ_GATE_JOURNAL_VERSION {
        return Err(LezGateError(format!(
            "unexpected lez gate journal version {}, expected {LEZ_GATE_JOURNAL_VERSION}",
            decoded.version
        )));
    }

    let journal = LezGateJournal {
        version: decoded.version,
        inner_image_id: Digest32(decoded.inner_image_id),
        gate_context_id: Digest32(decoded.gate_context_id),
        accepted_context_nullifier: Digest32(decoded.accepted_context_nullifier),
        accepted_presenter_id: Digest32(decoded.accepted_presenter_id),
        accepted_threshold: decoded.accepted_threshold,
    };

    let receipt_bytes = serde_json::to_vec(&prove_info.receipt)
        .map_err(|e| LezGateError(format!("outer receipt serialize failed: {e}")))?;

    Ok(LezGateProof {
        journal,
        receipt_bytes,
    })
}

#[derive(Debug)]
pub enum LezGateProgramError {
    MalformedReceipt(String),
    InvalidReceipt(String),
    JournalDecodeFailed(String),
    UnexpectedJournalVersion {
        actual: u16,
    },
    InnerImageIdMismatch {
        actual: Digest32,
    },
    GateContextMismatch {
        expected: Digest32,
        actual: Digest32,
    },
    ThresholdNotMet {
        required: u128,
        journal: u128,
    },
    NullifierReplay {
        nullifier: Digest32,
    },
}

impl fmt::Display for LezGateProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedReceipt(d) => write!(f, "outer receipt deserialize failed: {d}"),
            Self::InvalidReceipt(d) => write!(f, "outer receipt verify failed: {d}"),
            Self::JournalDecodeFailed(d) => write!(f, "outer journal decode failed: {d}"),
            Self::UnexpectedJournalVersion { actual } => {
                write!(f, "unexpected lez gate journal version {actual}")
            }
            Self::InnerImageIdMismatch { actual } => write!(
                f,
                "inner_image_id mismatch: outer journal claims {}",
                actual.to_hex()
            ),
            Self::GateContextMismatch { expected, actual } => write!(
                f,
                "gate_context_id mismatch: expected {}, got {}",
                expected.to_hex(),
                actual.to_hex()
            ),
            Self::ThresholdNotMet { required, journal } => write!(
                f,
                "outer journal threshold {journal} below required {required}"
            ),
            Self::NullifierReplay { nullifier } => {
                write!(f, "nullifier already admitted: {}", nullifier.to_hex())
            }
        }
    }
}

impl std::error::Error for LezGateProgramError {}

/// In-memory rehearsal of the LEZ on-chain program. A real deployment runs
/// inside LEZ's private-execution gate and persists `admitted_nullifiers` to
/// program storage.
pub struct LezGateProgram {
    gate: LezGateConfig,
    admitted_nullifiers: HashSet<Digest32>,
}

impl LezGateProgram {
    pub fn new(gate: LezGateConfig) -> Self {
        Self {
            gate,
            admitted_nullifiers: HashSet::new(),
        }
    }

    pub fn gate(&self) -> &LezGateConfig {
        &self.gate
    }

    /// Validate an outer LEZ-gate receipt and admit the presenter on success.
    /// Returns the recorded nullifier so the LEZ program can index its state.
    pub fn admit(&mut self, proof: &LezGateProof) -> Result<Digest32, LezGateProgramError> {
        // 1. Outer receipt verifies against the pinned LEZ_BALANCE_GATE_ID.
        let outer_receipt: Receipt = serde_json::from_slice(&proof.receipt_bytes)
            .map_err(|e| LezGateProgramError::MalformedReceipt(e.to_string()))?;
        outer_receipt
            .verify(risc0_zkvm::Digest::from(LEZ_BALANCE_GATE_ID))
            .map_err(|e| LezGateProgramError::InvalidReceipt(e.to_string()))?;

        // 2. Decode and check the journal matches what the proof carries.
        let decoded: OuterJournalDecoded = outer_receipt
            .journal
            .decode()
            .map_err(|e| LezGateProgramError::JournalDecodeFailed(e.to_string()))?;
        if decoded.version != LEZ_GATE_JOURNAL_VERSION {
            return Err(LezGateProgramError::UnexpectedJournalVersion {
                actual: decoded.version,
            });
        }

        // 3. Inner image id must be the pinned BALANCE_ATTESTATION_ID — defense
        //    against an outer receipt produced over a different inner circuit.
        let pinned_inner = balance_attestation_inner_image_id();
        if decoded.inner_image_id != pinned_inner.0 {
            return Err(LezGateProgramError::InnerImageIdMismatch {
                actual: Digest32(decoded.inner_image_id),
            });
        }

        // 4. Gate context: must match the program's pinned gate.
        let expected_ctx = self.gate.expected_context_id();
        if decoded.gate_context_id != expected_ctx.0 {
            return Err(LezGateProgramError::GateContextMismatch {
                expected: expected_ctx,
                actual: Digest32(decoded.gate_context_id),
            });
        }

        // 5. Threshold floor check.
        if decoded.accepted_threshold < self.gate.threshold {
            return Err(LezGateProgramError::ThresholdNotMet {
                required: self.gate.threshold,
                journal: decoded.accepted_threshold,
            });
        }

        // 6. Nullifier replay: dedup against admitted_nullifiers.
        let nullifier = Digest32(decoded.accepted_context_nullifier);
        if !self.admitted_nullifiers.insert(nullifier) {
            return Err(LezGateProgramError::NullifierReplay { nullifier });
        }

        Ok(nullifier)
    }

    pub fn admitted_count(&self) -> usize {
        self.admitted_nullifiers.len()
    }
}
