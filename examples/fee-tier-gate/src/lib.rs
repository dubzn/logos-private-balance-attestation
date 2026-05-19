//! Fee-tier gate reference integration for LP-0005.
//!
//! Demonstrates a product/protocol fee schedule where users prove they meet a
//! private balance threshold to unlock a lower fee tier. The verifier learns
//! only the claimed tier, presenter id, and nullifier; it never learns the
//! private account id or exact balance.

use std::collections::{HashMap, HashSet};

use attestation_core::{
    derive_context_id, BalanceAttestationEnvelope, ContextBindingParams, Digest32,
};
use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum FeeTier {
    Standard,
    Silver,
    Gold,
}

impl FeeTier {
    pub fn threshold(self) -> u128 {
        match self {
            Self::Standard => 0,
            Self::Silver => 100,
            Self::Gold => 500,
        }
    }

    pub fn fee_bps(self) -> u16 {
        match self {
            Self::Standard => 100,
            Self::Silver => 60,
            Self::Gold => 25,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Silver => "silver",
            Self::Gold => "gold",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTierGate {
    pub chain_id: Digest32,
    pub circuit_image_id: Digest32,
    pub verifier_id: Digest32,
    pub gate_id: Digest32,
    /// Verifier-provided session challenge. Production callers should rotate
    /// this per claim session.
    pub presentation_challenge: Digest32,
}

impl FeeTierGate {
    pub fn expected(&self, tier: FeeTier) -> ExpectedGate {
        let ctx_params = ContextBindingParams {
            chain_id: self.chain_id,
            circuit_image_id: self.circuit_image_id,
            verifier_id: self.verifier_id,
            gate_id: self.gate_id,
            threshold: tier.threshold(),
        };
        ExpectedGate {
            context_id: derive_context_id(&ctx_params),
            threshold: tier.threshold(),
            presentation_challenge: self.presentation_challenge,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePass {
    pub presenter_id: Digest32,
    pub tier: FeeTier,
    pub fee_bps: u16,
    pub context_nullifier: Digest32,
}

#[derive(Debug)]
pub enum ClaimError {
    StandardDoesNotNeedProof,
    Verify(VerifyError),
    NullifierReplay {
        nullifier: Digest32,
    },
    LowerTierAlreadyClaimed {
        current: FeeTier,
        requested: FeeTier,
    },
}

impl std::fmt::Display for ClaimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StandardDoesNotNeedProof => {
                f.write_str("standard fee tier does not require an attestation")
            }
            Self::Verify(error) => write!(f, "envelope rejected: {error}"),
            Self::NullifierReplay { nullifier } => write!(
                f,
                "nullifier already used for a fee-tier claim: {}",
                nullifier.to_hex()
            ),
            Self::LowerTierAlreadyClaimed { current, requested } => write!(
                f,
                "presenter already has tier {}, cannot downgrade to {}",
                current.as_str(),
                requested.as_str()
            ),
        }
    }
}

impl std::error::Error for ClaimError {}

/// In-memory fee gate. A real integration would persist the best claimed tier
/// per presenter and the used nullifiers in app or on-chain state.
pub struct FeeTierProgram {
    gate: FeeTierGate,
    used_nullifiers: HashSet<Digest32>,
    best_tier_by_presenter: HashMap<Digest32, FeeTier>,
}

impl FeeTierProgram {
    pub fn new(gate: FeeTierGate) -> Self {
        Self {
            gate,
            used_nullifiers: HashSet::new(),
            best_tier_by_presenter: HashMap::new(),
        }
    }

    pub fn gate(&self) -> &FeeTierGate {
        &self.gate
    }

    pub fn claim_tier(
        &mut self,
        envelope: &BalanceAttestationEnvelope,
        requested: FeeTier,
    ) -> Result<FeePass, ClaimError> {
        if requested == FeeTier::Standard {
            return Err(ClaimError::StandardDoesNotNeedProof);
        }

        verify_envelope(envelope, &self.gate.expected(requested)).map_err(ClaimError::Verify)?;

        let nullifier = envelope.journal.context_nullifier;
        if !self.used_nullifiers.insert(nullifier) {
            return Err(ClaimError::NullifierReplay { nullifier });
        }

        let presenter_id = envelope.journal.presenter_id;
        if let Some(current) = self.best_tier_by_presenter.get(&presenter_id).copied() {
            if requested < current {
                return Err(ClaimError::LowerTierAlreadyClaimed { current, requested });
            }
        }

        self.best_tier_by_presenter.insert(presenter_id, requested);

        Ok(FeePass {
            presenter_id,
            tier: requested,
            fee_bps: requested.fee_bps(),
            context_nullifier: nullifier,
        })
    }

    pub fn best_tier_for(&self, presenter_id: Digest32) -> FeeTier {
        self.best_tier_by_presenter
            .get(&presenter_id)
            .copied()
            .unwrap_or(FeeTier::Standard)
    }

    pub fn fee_bps_for(&self, presenter_id: Digest32) -> u16 {
        self.best_tier_for(presenter_id).fee_bps()
    }

    pub fn claimed_count(&self) -> usize {
        self.best_tier_by_presenter.len()
    }
}
