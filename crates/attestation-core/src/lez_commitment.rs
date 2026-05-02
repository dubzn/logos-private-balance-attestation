use crate::{sha256_bytes, Digest32, HexBytes};
use serde::{Deserialize, Serialize};

pub const LEZ_COMMITMENT_PREFIX: &[u8; 32] =
    b"/LEE/v0.3/Commitment/\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LezPrivateAccountCommitmentInput {
    pub npk: Digest32,
    pub program_owner: [u32; 8],
    #[serde(with = "crate::serde_helpers::u128_decimal")]
    pub balance: u128,
    #[serde(with = "crate::serde_helpers::u128_decimal")]
    pub nonce: u128,
    pub data: HexBytes,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LezMembershipProof {
    pub index: u64,
    pub siblings: Vec<Digest32>,
}

pub fn derive_lez_private_account_commitment(input: &LezPrivateAccountCommitmentInput) -> Digest32 {
    let mut account_bytes = Vec::with_capacity(32 + 16 + 16 + 32);
    for word in input.program_owner {
        account_bytes.extend_from_slice(&word.to_le_bytes());
    }
    account_bytes.extend_from_slice(&input.balance.to_le_bytes());
    account_bytes.extend_from_slice(&input.nonce.to_le_bytes());
    account_bytes.extend_from_slice(sha256_bytes(input.data.as_bytes()).as_bytes());

    let mut commitment_bytes = Vec::with_capacity(32 + 32 + account_bytes.len());
    commitment_bytes.extend_from_slice(LEZ_COMMITMENT_PREFIX);
    commitment_bytes.extend_from_slice(input.npk.as_bytes());
    commitment_bytes.extend_from_slice(&account_bytes);

    sha256_bytes(&commitment_bytes)
}

pub fn hash_lez_commitment_leaf(commitment: &Digest32) -> Digest32 {
    sha256_bytes(commitment.as_bytes())
}

pub fn compute_lez_membership_root(commitment: &Digest32, proof: &LezMembershipProof) -> Digest32 {
    let mut result = hash_lez_commitment_leaf(commitment);
    let mut level_index = proof.index;

    for sibling in &proof.siblings {
        let mut bytes = [0_u8; 64];
        let is_left_child = level_index & 1 == 0;
        if is_left_child {
            bytes[..32].copy_from_slice(result.as_bytes());
            bytes[32..].copy_from_slice(sibling.as_bytes());
        } else {
            bytes[..32].copy_from_slice(sibling.as_bytes());
            bytes[32..].copy_from_slice(result.as_bytes());
        }
        result = sha256_bytes(&bytes);
        level_index >>= 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_input() -> LezPrivateAccountCommitmentInput {
        LezPrivateAccountCommitmentInput {
            npk: Digest32([0; 32]),
            program_owner: [0; 8],
            balance: 0,
            nonce: 0,
            data: HexBytes::default(),
        }
    }

    #[test]
    fn dummy_commitment_matches_lez_constant() {
        let commitment = derive_lez_private_account_commitment(&dummy_input());
        assert_eq!(
            commitment.to_hex(),
            "37e4d7cf70ddef31ee4f47879b0fb82d684a33d3ee2aa0f30f7cfd3e03e55a1b"
        );
    }

    #[test]
    fn dummy_commitment_leaf_hash_matches_lez_constant() {
        let commitment = derive_lez_private_account_commitment(&dummy_input());
        let leaf_hash = hash_lez_commitment_leaf(&commitment);
        assert_eq!(
            leaf_hash.to_hex(),
            "faedc0719b65771eebb714541a20c4e59a4afef981f1762729fd8dabb8470829"
        );
    }

    #[test]
    fn commitment_changes_with_private_account_fields() {
        let base = derive_lez_private_account_commitment(&dummy_input());

        let mut changed = dummy_input();
        changed.npk = Digest32([1; 32]);
        assert_ne!(base, derive_lez_private_account_commitment(&changed));

        let mut changed = dummy_input();
        changed.program_owner[0] = 1;
        assert_ne!(base, derive_lez_private_account_commitment(&changed));

        let mut changed = dummy_input();
        changed.balance = 1;
        assert_ne!(base, derive_lez_private_account_commitment(&changed));

        let mut changed = dummy_input();
        changed.nonce = 1;
        assert_ne!(base, derive_lez_private_account_commitment(&changed));

        let mut changed = dummy_input();
        changed.data = HexBytes::new(b"state".to_vec());
        assert_ne!(base, derive_lez_private_account_commitment(&changed));
    }

    #[test]
    fn empty_membership_proof_root_is_leaf_hash() {
        let commitment = derive_lez_private_account_commitment(&dummy_input());
        let proof = LezMembershipProof {
            index: 0,
            siblings: Vec::new(),
        };
        assert_eq!(
            compute_lez_membership_root(&commitment, &proof),
            hash_lez_commitment_leaf(&commitment)
        );
    }

    #[test]
    fn membership_root_changes_with_index_ordering() {
        let commitment = derive_lez_private_account_commitment(&dummy_input());
        let sibling = Digest32([0x11; 32]);
        let left = compute_lez_membership_root(
            &commitment,
            &LezMembershipProof {
                index: 0,
                siblings: vec![sibling],
            },
        );
        let right = compute_lez_membership_root(
            &commitment,
            &LezMembershipProof {
                index: 1,
                siblings: vec![sibling],
            },
        );
        assert_ne!(left, right);
    }
}
