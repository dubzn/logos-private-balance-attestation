use nssa_core::{
    account::Account, compute_digest_for_path, Commitment, MembershipProof, NullifierPublicKey,
};
use risc0_zkvm::{
    guest::env,
    sha::{Impl, Sha256 as _},
};
use serde::{Deserialize, Serialize};

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const NULLIFIER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/nullifier";
const PRESENTER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/presenter";

#[derive(Serialize, Deserialize)]
struct BindingAttestationInput {
    npk: NullifierPublicKey,
    account: Account,
    membership_proof: MembershipProof,
    threshold: u128,
    commitment_root: [u8; 32],
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    circuit_image_id: [u8; 32],
    presenter_secret: [u8; 32],
    presenter_id: [u8; 32],
    expected_context_nullifier: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct BindingAttestationJournal {
    version: u32,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
    context_nullifier: [u8; 32],
    presenter_id: [u8; 32],
    verifier_id: [u8; 32],
    circuit_image_id: [u8; 32],
    proof_index: usize,
    proof_depth: usize,
}

fn main() {
    let input: BindingAttestationInput = env::read();

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

    let presenter_id = derive_presenter_id(&input.presenter_secret);
    assert_eq!(
        presenter_id, input.presenter_id,
        "presenter secret does not match presenter id"
    );

    let context_id = derive_context_id(
        input.threshold,
        &input.chain_id,
        &input.verifier_id,
        &input.gate_id,
        &input.circuit_image_id,
    );
    let context_nullifier = derive_context_nullifier(&input.npk, &context_id, &input.presenter_id);
    assert_eq!(
        context_nullifier, input.expected_context_nullifier,
        "context nullifier does not match private witness and context"
    );

    env::commit(&BindingAttestationJournal {
        version: 1,
        threshold: input.threshold,
        commitment_root: input.commitment_root,
        context_id,
        context_nullifier,
        presenter_id: input.presenter_id,
        verifier_id: input.verifier_id,
        circuit_image_id: input.circuit_image_id,
        proof_index: input.membership_proof.0,
        proof_depth: input.membership_proof.1.len(),
    });
}

fn derive_presenter_id(presenter_secret: &[u8; 32]) -> [u8; 32] {
    hash_segments(&[PRESENTER_DOMAIN, presenter_secret])
}

fn derive_context_id(
    threshold: u128,
    chain_id: &[u8; 32],
    verifier_id: &[u8; 32],
    gate_id: &[u8; 32],
    circuit_image_id: &[u8; 32],
) -> [u8; 32] {
    hash_segments(&[
        CONTEXT_DOMAIN,
        chain_id,
        verifier_id,
        gate_id,
        &threshold.to_le_bytes(),
        circuit_image_id,
    ])
}

fn derive_context_nullifier(
    npk: &NullifierPublicKey,
    context_id: &[u8; 32],
    presenter_id: &[u8; 32],
) -> [u8; 32] {
    hash_segments(&[
        NULLIFIER_DOMAIN,
        &npk.to_byte_array(),
        context_id,
        presenter_id,
    ])
}

fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    let mut bytes = Vec::new();
    for segment in segments {
        bytes.extend_from_slice(&(segment.len() as u64).to_le_bytes());
        bytes.extend_from_slice(segment);
    }
    Impl::hash_bytes(&bytes).as_bytes().try_into().unwrap()
}
