// Production balance-attestation circuit.
// Proves: balance >= threshold, Merkle membership, presenter binding, context binding, context nullifier.
// Journal schema mirrors attestation_core::BalanceAttestationJournal exactly.

#![no_main]

use risc0_zkvm::{
    guest::env,
    sha::{Impl, Sha256 as _},
};
use serde::{Deserialize, Serialize};

risc0_zkvm::guest::entry!(main);

// Domain constants — must stay byte-identical to attestation_core::hash (crates/attestation-core/src/hash.rs).
const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const NULLIFIER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/nullifier";
const PRESENTER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/presenter";

// LEZ private account commitment prefix — must match LEZ_COMMITMENT_PREFIX in attestation_core::lez_commitment.
const LEZ_COMMITMENT_PREFIX: &[u8; 32] =
    b"/LEE/v0.3/Commitment/\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

#[derive(Serialize, Deserialize)]
struct BalanceAttestationInput {
    // Private account — used to reconstruct the LEZ commitment.
    account_id: [u8; 32],
    npk: [u8; 32],
    program_owner: [u32; 8],
    balance: u128,
    nonce: u128,
    data: Vec<u8>,
    // Merkle membership proof.
    proof_index: u64,
    proof_siblings: Vec<[u8; 32]>,
    // Public gate params.
    threshold: u128,
    commitment_root: [u8; 32],
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    circuit_image_id: [u8; 32],
    // Presenter binding: 32-byte BIP-340 x-only Schnorr pubkey. The circuit only
    // hashes it; the off-chain Schnorr signature in the envelope proves knowledge
    // of the matching secret (no in-circuit ECC).
    presenter_pubkey: [u8; 32],
    presenter_id: [u8; 32],
    // Anti-replay: prover pre-computes the expected nullifier; circuit verifies it.
    expected_context_nullifier: [u8; 32],
}

// Journal types match attestation_core::BalanceAttestationJournal field-for-field.
// version: u16, proof_index/depth: u64 (NOT u32/usize from spike-04).
#[derive(Serialize, Deserialize)]
struct BalanceAttestationJournal {
    version: u16,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
    context_nullifier: [u8; 32],
    presenter_id: [u8; 32],
    verifier_id: [u8; 32],
    circuit_image_id: [u8; 32],
    proof_index: u64,
    proof_depth: u64,
}

pub fn main() {
    let input: BalanceAttestationInput = env::read();

    // 1. Balance check.
    assert!(
        input.balance >= input.threshold,
        "private balance is below threshold"
    );

    // 2. Commitment construction and Merkle membership check.
    let commitment = derive_lez_commitment(&input);
    let computed_root = compute_membership_root(commitment, input.proof_index, &input.proof_siblings);
    assert_eq!(
        computed_root, input.commitment_root,
        "commitment membership proof does not resolve to expected root"
    );

    // 3. Presenter binding: verify pubkey hashes to the claimed public presenter_id.
    // Knowledge-of-secret is proved off-circuit by the BIP-340 signature in the envelope.
    let presenter_id = derive_presenter_id(&input.presenter_pubkey);
    assert_eq!(
        presenter_id, input.presenter_id,
        "presenter pubkey does not match presenter id"
    );

    // 4. Context binding: derive context_id from public gate params.
    // Field order mirrors attestation_core::derive_context_id (crates/attestation-core/src/hash.rs:9-18).
    let context_id = derive_context_id(
        &input.chain_id,
        &input.circuit_image_id,
        &input.verifier_id,
        &input.gate_id,
        input.threshold,
    );

    // 5. Context nullifier: derive and verify anti-replay tag.
    let context_nullifier = derive_context_nullifier(&input.npk, &context_id, &input.presenter_id);
    assert_eq!(
        context_nullifier, input.expected_context_nullifier,
        "context nullifier does not match private witness and context"
    );

    env::commit(&BalanceAttestationJournal {
        version: 1,
        threshold: input.threshold,
        commitment_root: input.commitment_root,
        context_id,
        context_nullifier,
        presenter_id: input.presenter_id,
        verifier_id: input.verifier_id,
        circuit_image_id: input.circuit_image_id,
        proof_index: input.proof_index,
        proof_depth: input.proof_siblings.len() as u64,
    });
}

// --- LEZ commitment helpers (mirror of attestation_core::lez_commitment) ---

fn derive_lez_commitment(input: &BalanceAttestationInput) -> [u8; 32] {
    // account_bytes = program_owner_le || balance_le || nonce_le || sha256(data)
    let mut account_bytes = Vec::with_capacity(32 + 16 + 16 + 32);
    for word in input.program_owner {
        account_bytes.extend_from_slice(&word.to_le_bytes());
    }
    account_bytes.extend_from_slice(&input.balance.to_le_bytes());
    account_bytes.extend_from_slice(&input.nonce.to_le_bytes());
    account_bytes.extend_from_slice(&sha256(&input.data));

    // commitment = sha256(prefix || account_id || account_bytes)
    let mut commitment_bytes = Vec::with_capacity(32 + 32 + account_bytes.len());
    commitment_bytes.extend_from_slice(LEZ_COMMITMENT_PREFIX);
    commitment_bytes.extend_from_slice(&input.account_id);
    commitment_bytes.extend_from_slice(&account_bytes);
    sha256(&commitment_bytes)
}

fn compute_membership_root(commitment: [u8; 32], index: u64, siblings: &[[u8; 32]]) -> [u8; 32] {
    // leaf = sha256(commitment)
    let mut current = sha256(&commitment);
    let mut idx = index;
    for sibling in siblings {
        let mut buf = [0u8; 64];
        if idx & 1 == 0 {
            buf[..32].copy_from_slice(&current);
            buf[32..].copy_from_slice(sibling);
        } else {
            buf[..32].copy_from_slice(sibling);
            buf[32..].copy_from_slice(&current);
        }
        current = sha256(&buf);
        idx >>= 1;
    }
    current
}

// --- Attestation derivation functions ---

fn derive_presenter_id(presenter_pubkey: &[u8; 32]) -> [u8; 32] {
    hash_segments(&[PRESENTER_DOMAIN, presenter_pubkey])
}

fn derive_context_id(
    chain_id: &[u8; 32],
    circuit_image_id: &[u8; 32],
    verifier_id: &[u8; 32],
    gate_id: &[u8; 32],
    threshold: u128,
) -> [u8; 32] {
    // Order: domain, chain_id, circuit_image_id, verifier_id, gate_id, threshold
    // mirrors attestation_core::derive_context_id (crates/attestation-core/src/hash.rs:9-18)
    hash_segments(&[
        CONTEXT_DOMAIN,
        chain_id,
        circuit_image_id,
        verifier_id,
        gate_id,
        &threshold.to_le_bytes(),
    ])
}

fn derive_context_nullifier(
    npk: &[u8; 32],
    context_id: &[u8; 32],
    presenter_id: &[u8; 32],
) -> [u8; 32] {
    hash_segments(&[NULLIFIER_DOMAIN, npk, context_id, presenter_id])
}

// Length-delimited SHA-256 over segments.
// sha2 is forbidden in the guest — use the RISC Zero SHA-256 accelerator.
fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    let mut bytes = Vec::new();
    for segment in segments {
        bytes.extend_from_slice(&(segment.len() as u64).to_le_bytes());
        bytes.extend_from_slice(segment);
    }
    sha256(&bytes)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Impl::hash_bytes(bytes).as_bytes().try_into().unwrap()
}
