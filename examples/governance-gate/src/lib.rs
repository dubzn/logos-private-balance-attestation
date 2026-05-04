//! Governance-gate reference integration for LP-0005.
//!
//! Demonstrates the off-chain verification flow: a governance contract that
//! admits one vote per (presenter, gate) pair, gated on `balance >= threshold`
//! proven via a balance-attestation envelope. The same logic shape (verify the
//! envelope, then check nullifier uniqueness) is what the on-chain LEZ
//! verifier program will execute — this example is an in-memory rehearsal.

use std::collections::{HashMap, HashSet};

use attestation_core::{
    derive_context_id, BalanceAttestationEnvelope, ContextBindingParams, Digest32,
};
use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};

/// What a vote envelope is bound to. The verifier — here the "governance program"
/// — pins these values; envelopes whose journal does not match are rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateConfig {
    pub chain_id: Digest32,
    pub circuit_image_id: Digest32,
    pub verifier_id: Digest32,
    pub gate_id: Digest32,
    pub threshold: u128,
}

impl GateConfig {
    pub fn expected(&self) -> ExpectedGate {
        let ctx_params = ContextBindingParams {
            chain_id: self.chain_id,
            circuit_image_id: self.circuit_image_id,
            verifier_id: self.verifier_id,
            gate_id: self.gate_id,
            threshold: self.threshold,
        };
        ExpectedGate {
            context_id: derive_context_id(&ctx_params),
            threshold: self.threshold,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Choice {
    Yes,
    No,
    Abstain,
}

#[derive(Debug)]
pub enum SubmitError {
    Verify(VerifyError),
    /// The journal's context_nullifier has already been recorded for this gate.
    NullifierReplay {
        nullifier: Digest32,
    },
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verify(e) => write!(f, "envelope rejected: {e}"),
            Self::NullifierReplay { nullifier } => write!(
                f,
                "nullifier already used in this gate: {}",
                nullifier.to_hex()
            ),
        }
    }
}

impl std::error::Error for SubmitError {}

/// In-memory "program" state. A real on-chain implementation would persist
/// `tally` and `used_nullifiers` in account storage and reject the same
/// nullifier across replays.
pub struct GovernanceProgram {
    gate: GateConfig,
    used_nullifiers: HashSet<Digest32>,
    tally: HashMap<Choice, u64>,
}

impl GovernanceProgram {
    pub fn new(gate: GateConfig) -> Self {
        Self {
            gate,
            used_nullifiers: HashSet::new(),
            tally: HashMap::new(),
        }
    }

    pub fn gate(&self) -> &GateConfig {
        &self.gate
    }

    /// Cast a vote backed by a balance-attestation envelope. Returns the
    /// nullifier that was recorded so callers can audit the chain of votes.
    pub fn submit_vote(
        &mut self,
        envelope: &BalanceAttestationEnvelope,
        choice: Choice,
    ) -> Result<Digest32, SubmitError> {
        verify_envelope(envelope, &self.gate.expected()).map_err(SubmitError::Verify)?;

        let nullifier = envelope.journal.context_nullifier;
        if !self.used_nullifiers.insert(nullifier) {
            return Err(SubmitError::NullifierReplay { nullifier });
        }
        *self.tally.entry(choice).or_default() += 1;
        Ok(nullifier)
    }

    pub fn tally(&self, choice: Choice) -> u64 {
        self.tally.get(&choice).copied().unwrap_or(0)
    }

    pub fn total_votes(&self) -> u64 {
        self.tally.values().sum()
    }
}
