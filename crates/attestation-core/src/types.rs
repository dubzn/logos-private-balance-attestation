use crate::{
    derive_context_id, hash_segments, serde_helpers, AttestationError, AttestationErrorCode,
    Digest32, HexBytes, JOURNAL_DOMAIN,
};
use serde::{Deserialize, Serialize};

pub const JOURNAL_VERSION: u16 = 1;
pub const ENVELOPE_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofSystem {
    #[serde(rename = "risc0")]
    Risc0,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextBindingParams {
    pub chain_id: Digest32,
    pub circuit_image_id: Digest32,
    pub verifier_id: Digest32,
    pub gate_id: Digest32,
    #[serde(with = "serde_helpers::u128_decimal")]
    pub threshold: u128,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BalanceAttestationJournal {
    pub version: u16,
    #[serde(with = "serde_helpers::u128_decimal")]
    pub threshold: u128,
    pub commitment_root: Digest32,
    pub context_id: Digest32,
    pub context_nullifier: Digest32,
    pub presenter_id: Digest32,
    pub verifier_id: Digest32,
    pub circuit_image_id: Digest32,
    pub proof_index: u64,
    pub proof_depth: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BalanceAttestationEnvelope {
    pub version: u16,
    pub proof_system: ProofSystem,
    pub image_id: Digest32,
    pub journal: BalanceAttestationJournal,
    pub receipt: HexBytes,
    pub presenter_signature: Option<HexBytes>,
}

impl ContextBindingParams {
    pub fn context_id(&self) -> Digest32 {
        derive_context_id(self)
    }
}

impl BalanceAttestationJournal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        threshold: u128,
        commitment_root: Digest32,
        context_id: Digest32,
        context_nullifier: Digest32,
        presenter_id: Digest32,
        verifier_id: Digest32,
        circuit_image_id: Digest32,
        proof_index: u64,
        proof_depth: u64,
    ) -> Self {
        Self {
            version: JOURNAL_VERSION,
            threshold,
            commitment_root,
            context_id,
            context_nullifier,
            presenter_id,
            verifier_id,
            circuit_image_id,
            proof_index,
            proof_depth,
        }
    }

    pub fn digest(&self) -> Digest32 {
        hash_segments(&[
            JOURNAL_DOMAIN,
            &self.version.to_le_bytes(),
            &self.threshold.to_le_bytes(),
            self.commitment_root.as_bytes(),
            self.context_id.as_bytes(),
            self.context_nullifier.as_bytes(),
            self.presenter_id.as_bytes(),
            self.verifier_id.as_bytes(),
            self.circuit_image_id.as_bytes(),
            &self.proof_index.to_le_bytes(),
            &self.proof_depth.to_le_bytes(),
        ])
    }
}

impl BalanceAttestationEnvelope {
    pub fn new_risc0(journal: BalanceAttestationJournal, receipt: Vec<u8>) -> Self {
        Self {
            version: ENVELOPE_VERSION,
            proof_system: ProofSystem::Risc0,
            image_id: journal.circuit_image_id,
            journal,
            receipt: receipt.into(),
            presenter_signature: None,
        }
    }

    pub fn with_presenter_signature(mut self, signature: Vec<u8>) -> Self {
        self.presenter_signature = Some(signature.into());
        self
    }

    pub fn validate_shape(&self) -> Result<(), AttestationError> {
        if self.version != ENVELOPE_VERSION {
            return Err(AttestationError::with_detail(
                AttestationErrorCode::InvalidEnvelopeVersion,
                format!("expected version {ENVELOPE_VERSION}, got {}", self.version),
            ));
        }

        if self.proof_system != ProofSystem::Risc0 {
            return Err(AttestationError::new(
                AttestationErrorCode::InvalidProofSystem,
            ));
        }

        if self.journal.version != JOURNAL_VERSION {
            return Err(AttestationError::with_detail(
                AttestationErrorCode::MalformedJournal,
                format!(
                    "expected journal version {JOURNAL_VERSION}, got {}",
                    self.journal.version
                ),
            ));
        }

        if self.image_id != self.journal.circuit_image_id {
            return Err(AttestationError::new(AttestationErrorCode::InvalidImageId));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{derive_context_nullifier, derive_presenter_id};

    fn digest(seed: u8) -> Digest32 {
        Digest32([seed; 32])
    }

    fn context_params() -> ContextBindingParams {
        ContextBindingParams {
            chain_id: digest(0x10),
            circuit_image_id: digest(0x20),
            verifier_id: digest(0x30),
            gate_id: digest(0x40),
            threshold: 100,
        }
    }

    fn journal() -> BalanceAttestationJournal {
        let params = context_params();
        let context_id = params.context_id();
        let presenter_id = derive_presenter_id(&digest(0x55));
        let nullifier = derive_context_nullifier(&digest(0x77), &context_id, &presenter_id);
        BalanceAttestationJournal::new(
            params.threshold,
            digest(0xaa),
            context_id,
            nullifier,
            presenter_id,
            params.verifier_id,
            params.circuit_image_id,
            5,
            16,
        )
    }

    #[test]
    fn derives_context_id_deterministically() {
        let first = context_params().context_id();
        let second = context_params().context_id();
        assert_eq!(first, second);
        assert_eq!(
            first.to_hex(),
            "0bbc46302aaa63e1bf7bdea0db33f21bef9a1a404de05ffabb74fe474519af41"
        );
    }

    #[test]
    fn context_id_changes_with_each_context_field() {
        let base = context_params().context_id();

        let mut changed = context_params();
        changed.chain_id = digest(0x11);
        assert_ne!(base, changed.context_id());

        let mut changed = context_params();
        changed.circuit_image_id = digest(0x21);
        assert_ne!(base, changed.context_id());

        let mut changed = context_params();
        changed.verifier_id = digest(0x31);
        assert_ne!(base, changed.context_id());

        let mut changed = context_params();
        changed.gate_id = digest(0x41);
        assert_ne!(base, changed.context_id());

        let mut changed = context_params();
        changed.threshold = 101;
        assert_ne!(base, changed.context_id());
    }

    #[test]
    fn nullifier_changes_by_context_or_presenter() {
        let context = context_params().context_id();
        let presenter = derive_presenter_id(&digest(0x55));
        let base = derive_context_nullifier(&digest(0x77), &context, &presenter);

        let other_context = {
            let mut params = context_params();
            params.gate_id = digest(0x41);
            params.context_id()
        };
        assert_ne!(
            base,
            derive_context_nullifier(&digest(0x77), &other_context, &presenter)
        );

        let other_presenter = derive_presenter_id(&digest(0x56));
        assert_ne!(
            base,
            derive_context_nullifier(&digest(0x77), &context, &other_presenter)
        );

        assert_ne!(
            base,
            derive_context_nullifier(&digest(0x78), &context, &presenter)
        );
    }

    #[test]
    fn journal_serialization_is_stable_and_hex_encoded() {
        let json = serde_json::to_string_pretty(&journal()).unwrap();
        assert_eq!(
            json,
            r#"{
  "version": 1,
  "threshold": "100",
  "commitment_root": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "context_id": "0bbc46302aaa63e1bf7bdea0db33f21bef9a1a404de05ffabb74fe474519af41",
  "context_nullifier": "a5acd9d46bc7d9077001c483f45ebf5be601b77b22d8add3b1aa2ef1f72d5cbf",
  "presenter_id": "e70e635f25d1d89a600bf581e15f72db6b5594bea46a6877e828df6be0f9d4ea",
  "verifier_id": "3030303030303030303030303030303030303030303030303030303030303030",
  "circuit_image_id": "2020202020202020202020202020202020202020202020202020202020202020",
  "proof_index": 5,
  "proof_depth": 16
}"#
        );
        assert_eq!(
            serde_json::from_str::<BalanceAttestationJournal>(&json).unwrap(),
            journal()
        );
    }

    #[test]
    fn journal_digest_is_stable() {
        assert_eq!(
            journal().digest().to_hex(),
            "f30b38540124de1425af566ffa4eaad8e7f8347efd7de8789d5b5ced06fb8a6f"
        );
    }

    #[test]
    fn envelope_shape_accepts_consistent_risc0_envelope() {
        let envelope =
            BalanceAttestationEnvelope::new_risc0(journal(), vec![0xde, 0xad, 0xbe, 0xef])
                .with_presenter_signature(vec![0xca, 0xfe]);
        envelope.validate_shape().unwrap();

        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["proof_system"], "risc0");
        assert_eq!(json["receipt"], "deadbeef");
        assert_eq!(json["presenter_signature"], "cafe");
    }

    #[test]
    fn envelope_shape_rejects_image_mismatch() {
        let mut envelope = BalanceAttestationEnvelope::new_risc0(journal(), vec![]);
        envelope.image_id = digest(0xff);
        let error = envelope.validate_shape().unwrap_err();
        assert_eq!(error.code(), AttestationErrorCode::InvalidImageId);
    }
}
