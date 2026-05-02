use std::fmt;

use attestation_core::{
    compute_lez_membership_root, derive_context_id, derive_context_nullifier,
    derive_lez_private_account_commitment, derive_presenter_id, Digest32, HexBytes,
    LezMembershipProof, LezPrivateAccountCommitmentInput,
};
use serde::{Deserialize, Serialize};

use crate::MembershipProofInspection;

pub const WITNESS_REDACTION_POLICY: &str =
    "witness debug/summary output redacts npk, balance, nonce, data, presenter secret, commitment, and membership siblings";

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivateAccountWitness {
    pub npk: Digest32,
    pub program_owner: [u32; 8],
    #[serde(with = "u128_decimal")]
    pub balance: u128,
    #[serde(with = "u128_decimal")]
    pub nonce: u128,
    pub data: HexBytes,
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct PresenterWitness {
    pub presenter_secret: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttestationPublicParams {
    #[serde(with = "u128_decimal")]
    pub threshold: u128,
    pub chain_id: Digest32,
    pub verifier_id: Digest32,
    pub gate_id: Digest32,
    pub circuit_image_id: Digest32,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BalanceAttestationWitness {
    pub private_account: PrivateAccountWitness,
    pub membership_proof: LezMembershipProof,
    pub presenter: PresenterWitness,
    pub threshold: u128,
    pub commitment_root: Digest32,
    pub context_id: Digest32,
    pub context_nullifier: Digest32,
    pub presenter_id: Digest32,
    pub verifier_id: Digest32,
    pub circuit_image_id: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BalanceAttestationWitnessSummary {
    pub threshold: String,
    pub commitment_root_hex: String,
    pub context_id_hex: String,
    pub context_nullifier_hex: String,
    pub presenter_id_hex: String,
    pub verifier_id_hex: String,
    pub circuit_image_id_hex: String,
    pub proof_index: u64,
    pub proof_depth: u64,
    pub redaction_policy: &'static str,
}

pub fn build_balance_attestation_witness(
    private_account: PrivateAccountWitness,
    membership_proof: LezMembershipProof,
    presenter: PresenterWitness,
    params: AttestationPublicParams,
) -> BalanceAttestationWitness {
    let commitment = private_account.commitment();
    let commitment_root = compute_lez_membership_root(&commitment, &membership_proof);
    let context_id = derive_context_id(&params.into());
    let presenter_id = derive_presenter_id(&presenter.presenter_secret);
    let context_nullifier =
        derive_context_nullifier(&private_account.npk, &context_id, &presenter_id);

    BalanceAttestationWitness {
        private_account,
        membership_proof,
        presenter,
        threshold: params.threshold,
        commitment_root,
        context_id,
        context_nullifier,
        presenter_id,
        verifier_id: params.verifier_id,
        circuit_image_id: params.circuit_image_id,
    }
}

pub fn inspect_membership_proof(
    private_account: &PrivateAccountWitness,
    proof: &LezMembershipProof,
    expected_root: Option<Digest32>,
) -> MembershipProofInspection {
    let commitment = private_account.commitment();
    let root = compute_lez_membership_root(&commitment, proof);

    MembershipProofInspection {
        proof_index: proof.index,
        proof_depth: proof.siblings.len() as u64,
        commitment_root: root,
        core_root_matches_wallet_root: match expected_root {
            Some(expected) => expected == root,
            None => true,
        },
    }
}

impl PrivateAccountWitness {
    pub fn commitment_input(&self) -> LezPrivateAccountCommitmentInput {
        LezPrivateAccountCommitmentInput {
            npk: self.npk,
            program_owner: self.program_owner,
            balance: self.balance,
            nonce: self.nonce,
            data: self.data.clone(),
        }
    }

    pub fn commitment(&self) -> Digest32 {
        derive_lez_private_account_commitment(&self.commitment_input())
    }
}

impl BalanceAttestationWitness {
    pub fn summary(&self) -> BalanceAttestationWitnessSummary {
        BalanceAttestationWitnessSummary {
            threshold: self.threshold.to_string(),
            commitment_root_hex: self.commitment_root.to_hex(),
            context_id_hex: self.context_id.to_hex(),
            context_nullifier_hex: self.context_nullifier.to_hex(),
            presenter_id_hex: self.presenter_id.to_hex(),
            verifier_id_hex: self.verifier_id.to_hex(),
            circuit_image_id_hex: self.circuit_image_id.to_hex(),
            proof_index: self.membership_proof.index,
            proof_depth: self.membership_proof.siblings.len() as u64,
            redaction_policy: WITNESS_REDACTION_POLICY,
        }
    }
}

impl From<AttestationPublicParams> for attestation_core::ContextBindingParams {
    fn from(value: AttestationPublicParams) -> Self {
        Self {
            chain_id: value.chain_id,
            circuit_image_id: value.circuit_image_id,
            verifier_id: value.verifier_id,
            gate_id: value.gate_id,
            threshold: value.threshold,
        }
    }
}

impl fmt::Debug for PrivateAccountWitness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateAccountWitness")
            .field("npk", &"<redacted>")
            .field("program_owner", &"<redacted>")
            .field("balance", &"<redacted>")
            .field("nonce", &"<redacted>")
            .field("data", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for PresenterWitness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PresenterWitness")
            .field("presenter_secret", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for BalanceAttestationWitness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BalanceAttestationWitness")
            .field("private_account", &self.private_account)
            .field("membership_proof", &"<redacted>")
            .field("presenter", &self.presenter)
            .field("threshold", &self.threshold)
            .field("commitment_root", &self.commitment_root)
            .field("context_id", &self.context_id)
            .field("context_nullifier", &self.context_nullifier)
            .field("presenter_id", &self.presenter_id)
            .field("verifier_id", &self.verifier_id)
            .field("circuit_image_id", &self.circuit_image_id)
            .finish()
    }
}

mod u128_decimal {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse::<u128>().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: u8) -> Digest32 {
        Digest32([seed; 32])
    }

    fn private_account() -> PrivateAccountWitness {
        PrivateAccountWitness {
            npk: digest(0x07),
            program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
            balance: 42,
            nonce: 123_456,
            data: HexBytes::new(b"witness fixture".to_vec()),
        }
    }

    fn proof() -> LezMembershipProof {
        LezMembershipProof {
            index: 5,
            siblings: vec![digest(0x11), digest(0x22), digest(0x33), digest(0x44)],
        }
    }

    fn params() -> AttestationPublicParams {
        AttestationPublicParams {
            threshold: 25,
            chain_id: digest(0x10),
            verifier_id: digest(0x20),
            gate_id: digest(0x30),
            circuit_image_id: digest(0x40),
        }
    }

    #[test]
    fn builds_balance_attestation_witness_from_private_account_and_context() {
        let witness = build_balance_attestation_witness(
            private_account(),
            proof(),
            PresenterWitness {
                presenter_secret: digest(0x77),
            },
            params(),
        );

        assert_eq!(witness.threshold, 25);
        assert_eq!(
            witness.commitment_root.to_hex(),
            "4f0de4f7a77701eb493e2a24d9d6596ee153e8c6abcb7eba828ea5a3efb09854"
        );
        assert_eq!(
            witness.context_id.to_hex(),
            "7467919be9a46e7e31823a93178a6f32b22e1f77d24c8fd3782423bccc67f9aa"
        );
        assert_eq!(
            witness.presenter_id.to_hex(),
            "ecc0fef4cd3e706458caa9eb944f487c99fa74d6c2c6a02bdae786450b850a48"
        );
        assert_eq!(
            witness.context_nullifier.to_hex(),
            "4a46dca676b96af04d1733a5c02bdc1aa9bee24d32615fa851491ad94ac46f14"
        );
    }

    #[test]
    fn witness_summary_exposes_only_public_fields() {
        let witness = build_balance_attestation_witness(
            private_account(),
            proof(),
            PresenterWitness {
                presenter_secret: digest(0x77),
            },
            params(),
        );
        let summary = witness.summary();
        let json = serde_json::to_string(&summary).unwrap();

        assert_eq!(summary.threshold, "25");
        assert_eq!(summary.proof_index, 5);
        assert_eq!(summary.proof_depth, 4);
        assert!(json.contains("context_nullifier_hex"));
        assert!(!json.contains("witness fixture"));
        assert!(!json.contains("123456"));
        assert!(!json.contains("presenter_secret"));
    }

    #[test]
    fn debug_output_redacts_private_fields() {
        let witness = build_balance_attestation_witness(
            private_account(),
            proof(),
            PresenterWitness {
                presenter_secret: digest(0x77),
            },
            params(),
        );
        let debug = format!("{witness:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("witness fixture"));
        assert!(!debug.contains("123456"));
        assert!(!debug.contains("presenter_secret: Digest32"));
    }

    #[test]
    fn membership_inspection_compares_expected_root_when_present() {
        let private_account = private_account();
        let proof = proof();
        let expected_root = compute_lez_membership_root(&private_account.commitment(), &proof);

        let ok = inspect_membership_proof(&private_account, &proof, Some(expected_root));
        assert!(ok.core_root_matches_wallet_root);

        let bad = inspect_membership_proof(&private_account, &proof, Some(digest(0xff)));
        assert!(!bad.core_root_matches_wallet_root);
    }
}
