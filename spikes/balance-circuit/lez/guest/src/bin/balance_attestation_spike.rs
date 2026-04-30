use nssa_core::{
    account::Account, compute_digest_for_path, Commitment, MembershipProof, NullifierPublicKey,
};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct BalanceAttestationInput {
    npk: NullifierPublicKey,
    account: Account,
    membership_proof: MembershipProof,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct BalanceAttestationJournal {
    version: u32,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
    commitment: [u8; 32],
    proof_index: usize,
    proof_depth: usize,
}

fn main() {
    let input: BalanceAttestationInput = env::read();

    assert!(
        input.account.balance >= input.threshold,
        "private balance is below threshold"
    );

    let commitment = Commitment::new(&input.npk, &input.account);
    let computed_root = compute_digest_for_path(&commitment, &input.membership_proof);
    assert_eq!(
        computed_root, input.commitment_root,
        "commitment membership proof does not resolve to expected root"
    );

    env::commit(&BalanceAttestationJournal {
        version: 1,
        threshold: input.threshold,
        commitment_root: input.commitment_root,
        context_id: input.context_id,
        commitment: commitment.to_byte_array(),
        proof_index: input.membership_proof.0,
        proof_depth: input.membership_proof.1.len(),
    });
}
