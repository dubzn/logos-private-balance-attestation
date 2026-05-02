use attestation_core::Digest32;
use serde::{Deserialize, Serialize};

pub const REDACTION_POLICY: &str =
    "does not print npk, balance, nonce, data, commitment, membership siblings, or private keys";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrivateAccountInspectSource {
    LocalOnly,
    GetProofForCommitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipProofInspection {
    pub proof_index: u64,
    pub proof_depth: u64,
    pub commitment_root: Digest32,
    pub core_root_matches_wallet_root: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivateAccountInspectStatus {
    pub account_id_raw: String,
    pub private_state_found: bool,
    pub local_commitment_matches_wallet: bool,
    pub membership_proof: Option<MembershipProofInspection>,
    pub source: PrivateAccountInspectSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivateAccountInspectReport {
    pub account_id_redacted: String,
    pub private_state_found: bool,
    pub local_commitment_matches_wallet: bool,
    pub membership_proof_found: bool,
    pub proof_index: Option<u64>,
    pub proof_depth: Option<u64>,
    pub commitment_root_hex: Option<String>,
    pub core_root_matches_wallet_root: Option<bool>,
    pub proof_source: String,
    pub redaction_policy: &'static str,
}

pub fn build_private_account_inspect_report(
    status: PrivateAccountInspectStatus,
) -> PrivateAccountInspectReport {
    let (
        membership_proof_found,
        proof_index,
        proof_depth,
        commitment_root_hex,
        core_root_matches_wallet_root,
    ) = status
        .membership_proof
        .map_or((false, None, None, None, None), |proof| {
            (
                true,
                Some(proof.proof_index),
                Some(proof.proof_depth),
                Some(proof.commitment_root.to_hex()),
                Some(proof.core_root_matches_wallet_root),
            )
        });

    PrivateAccountInspectReport {
        account_id_redacted: redact_private_account_id(&status.account_id_raw),
        private_state_found: status.private_state_found,
        local_commitment_matches_wallet: status.local_commitment_matches_wallet,
        membership_proof_found,
        proof_index,
        proof_depth,
        commitment_root_hex,
        core_root_matches_wallet_root,
        proof_source: status.source.description().to_owned(),
        redaction_policy: REDACTION_POLICY,
    }
}

pub fn redact_private_account_id(account_id: &str) -> String {
    if account_id.len() <= 12 {
        return "Private/<redacted>".to_owned();
    }

    let start = &account_id[..6];
    let end = &account_id[account_id.len() - 6..];
    format!("Private/{start}...{end}")
}

impl PrivateAccountInspectSource {
    pub fn description(&self) -> &'static str {
        match self {
            Self::LocalOnly => "local wallet storage only; getProofForCommitment not requested",
            Self::GetProofForCommitment => {
                "WalletCore::check_private_account_initialized -> getProofForCommitment"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_status() -> PrivateAccountInspectStatus {
        PrivateAccountInspectStatus {
            account_id_raw: "H7ZFXwtW692aufxMPrC2CBpVD3tKXRaATLsJtbFhokw".to_owned(),
            private_state_found: true,
            local_commitment_matches_wallet: true,
            membership_proof: None,
            source: PrivateAccountInspectSource::LocalOnly,
        }
    }

    #[test]
    fn redacts_long_private_account_id() {
        assert_eq!(
            redact_private_account_id("H7ZFXwtW692aufxMPrC2CBpVD3tKXRaATLsJtbFhokw"),
            "Private/H7ZFXw...bFhokw"
        );
    }

    #[test]
    fn redacts_short_private_account_id_completely() {
        assert_eq!(redact_private_account_id("short"), "Private/<redacted>");
    }

    #[test]
    fn builds_local_only_report_without_witness_fields() {
        let report = build_private_account_inspect_report(base_status());
        assert_eq!(report.account_id_redacted, "Private/H7ZFXw...bFhokw");
        assert!(report.private_state_found);
        assert!(report.local_commitment_matches_wallet);
        assert!(!report.membership_proof_found);
        assert_eq!(report.proof_index, None);
        assert_eq!(
            report.proof_source,
            PrivateAccountInspectSource::LocalOnly.description()
        );
        assert_eq!(report.redaction_policy, REDACTION_POLICY);
    }

    #[test]
    fn builds_membership_proof_report() {
        let mut status = base_status();
        status.source = PrivateAccountInspectSource::GetProofForCommitment;
        status.membership_proof = Some(MembershipProofInspection {
            proof_index: 6,
            proof_depth: 4,
            commitment_root: Digest32([0x9b; 32]),
            core_root_matches_wallet_root: true,
        });

        let report = build_private_account_inspect_report(status);
        assert!(report.membership_proof_found);
        assert_eq!(report.proof_index, Some(6));
        assert_eq!(report.proof_depth, Some(4));
        assert_eq!(
            report.commitment_root_hex.as_deref(),
            Some("9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b9b")
        );
        assert_eq!(report.core_root_matches_wallet_root, Some(true));
        assert_eq!(
            report.proof_source,
            PrivateAccountInspectSource::GetProofForCommitment.description()
        );
    }
}
