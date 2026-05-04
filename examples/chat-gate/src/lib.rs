//! Chat-gate reference integration for LP-0005.
//!
//! Demonstrates the **off-chain** path: a chat-group host that admits a user
//! after locally verifying a balance-attestation envelope received over a
//! messaging layer (here simulated as a `Vec<u8>` wire buffer; Logos Messaging
//! would carry the same bytes unchanged).
//!
//! The envelope is JSON-encoded — the same format `balance-attest prove`
//! writes to disk — so any byte-carrying transport (Logos Messaging, HTTP,
//! email, paste-in-text) can deliver it. The host needs zero interaction
//! with a sequencer or on-chain program to admit the user.

use std::collections::{HashMap, HashSet};

use attestation_core::{
    derive_context_id, BalanceAttestationEnvelope, ContextBindingParams, Digest32,
};
use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};

/// What the chat-room host pins as the gate definition. The `gate_id` should
/// be stable across the room's lifetime — sharing the same `gate_id` across
/// rooms would let a single proof admit a user to all of them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatRoomGate {
    pub chain_id: Digest32,
    pub circuit_image_id: Digest32,
    pub verifier_id: Digest32,
    pub gate_id: Digest32,
    pub min_balance: u128,
}

impl ChatRoomGate {
    fn expected(&self) -> ExpectedGate {
        let ctx_params = ContextBindingParams {
            chain_id: self.chain_id,
            circuit_image_id: self.circuit_image_id,
            verifier_id: self.verifier_id,
            gate_id: self.gate_id,
            threshold: self.min_balance,
        };
        ExpectedGate {
            context_id: derive_context_id(&ctx_params),
            min_threshold: self.min_balance,
        }
    }
}

/// Opaque session token returned to admitted members. The host uses this for
/// subsequent message authorization; it does NOT carry presenter identity.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SessionToken(pub String);

#[derive(Debug)]
pub enum AdmissionError {
    /// Wire bytes were not valid JSON / not a BalanceAttestationEnvelope.
    MalformedWire(String),
    /// verify_envelope rejected the proof.
    Verify(VerifyError),
    /// The presenter already joined this room; nullifier reuse is blocked.
    AlreadyAdmitted { nullifier: Digest32 },
}

impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedWire(d) => write!(f, "malformed envelope on the wire: {d}"),
            Self::Verify(e) => write!(f, "envelope rejected: {e}"),
            Self::AlreadyAdmitted { nullifier } => write!(
                f,
                "presenter already admitted (nullifier {})",
                nullifier.to_hex()
            ),
        }
    }
}

impl std::error::Error for AdmissionError {}

/// In-memory chat-room host. A real deployment would persist `members` and
/// `admitted_nullifiers` and might cache validated session tokens out-of-band.
pub struct ChatRoomHost {
    gate: ChatRoomGate,
    admitted_nullifiers: HashSet<Digest32>,
    members: HashMap<SessionToken, Digest32>,
    next_session_id: u64,
}

impl ChatRoomHost {
    pub fn new(gate: ChatRoomGate) -> Self {
        Self {
            gate,
            admitted_nullifiers: HashSet::new(),
            members: HashMap::new(),
            next_session_id: 0,
        }
    }

    pub fn gate(&self) -> &ChatRoomGate {
        &self.gate
    }

    /// Admit a user from a wire-encoded envelope. The host does the entire
    /// off-chain verification locally; no sequencer or on-chain step is involved.
    pub fn admit_from_wire(&mut self, wire_bytes: &[u8]) -> Result<SessionToken, AdmissionError> {
        let envelope: BalanceAttestationEnvelope = serde_json::from_slice(wire_bytes)
            .map_err(|e| AdmissionError::MalformedWire(e.to_string()))?;

        verify_envelope(&envelope, &self.gate.expected()).map_err(AdmissionError::Verify)?;

        let nullifier = envelope.journal.context_nullifier;
        if !self.admitted_nullifiers.insert(nullifier) {
            return Err(AdmissionError::AlreadyAdmitted { nullifier });
        }

        let token = SessionToken(format!("session-{}", self.next_session_id));
        self.next_session_id += 1;
        self.members.insert(token.clone(), nullifier);
        Ok(token)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn is_admitted(&self, token: &SessionToken) -> bool {
        self.members.contains_key(token)
    }

    pub fn nullifier_for(&self, token: &SessionToken) -> Option<Digest32> {
        self.members.get(token).copied()
    }
}

/// Helper for the prover side: serialize an envelope to wire bytes (JSON).
pub fn pack_envelope_for_wire(envelope: &BalanceAttestationEnvelope) -> Vec<u8> {
    serde_json::to_vec(envelope).expect("envelope is always serializable")
}
