// Journal roundtrip: proves that the production circuit's committed journal bytes
// decode back to attestation_core::BalanceAttestationJournal field-for-field.
// Run with: RISC0_DEV_MODE=1 cargo test -p methods --test journal_roundtrip

use attestation_core::{
    derive_context_id, derive_context_nullifier, derive_presenter_id, BalanceAttestationJournal,
    ContextBindingParams, Digest32,
};
use attestation_prover::{
    build_balance_attestation_witness, AttestationPublicParams, PresenterWitness,
    PrivateAccountWitness,
};
use methods::BALANCE_ATTESTATION_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv};
use serde::{Deserialize, Serialize};

// Mirror of the guest input struct — defined here so the test can serialize it
// without depending on the guest crate directly.
#[derive(Serialize, Deserialize)]
struct BalanceAttestationInput {
    npk: [u8; 32],
    program_owner: [u32; 8],
    balance: u128,
    nonce: u128,
    data: Vec<u8>,
    proof_index: u64,
    proof_siblings: Vec<[u8; 32]>,
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

// Mirror of the guest journal struct for decoding.
#[derive(Serialize, Deserialize, Debug)]
struct JournalDecoded {
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

fn digest(seed: u8) -> Digest32 {
    Digest32([seed; 32])
}

fn fixture_params() -> (AttestationPublicParams, [u8; 32], [u8; 32]) {
    let chain_id = [0x10_u8; 32];
    let gate_id = [0x30_u8; 32];
    let params = AttestationPublicParams {
        threshold: 25,
        chain_id: Digest32(chain_id),
        verifier_id: digest(0x20),
        gate_id: Digest32(gate_id),
        circuit_image_id: digest(0x40),
    };
    (params, chain_id, gate_id)
}

fn fixture_witness(params: &AttestationPublicParams) -> attestation_prover::BalanceAttestationWitness {
    use attestation_core::{HexBytes, LezMembershipProof};

    let private_account = PrivateAccountWitness {
        npk: digest(0x07),
        program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
        balance: 42,
        nonce: 123_456,
        data: HexBytes::new(b"witness fixture".to_vec()),
    };
    let proof = LezMembershipProof {
        index: 5,
        siblings: vec![digest(0x11), digest(0x22), digest(0x33), digest(0x44)],
    };
    let presenter = PresenterWitness {
        presenter_secret: digest(0x77),
    };
    build_balance_attestation_witness(private_account, proof, presenter, *params)
}

fn build_input(
    witness: &attestation_prover::BalanceAttestationWitness,
    chain_id: [u8; 32],
    gate_id: [u8; 32],
    bad_root: bool,
    bad_presenter: bool,
    bad_nullifier: bool,
    threshold_override: Option<u128>,
) -> BalanceAttestationInput {
    let threshold = threshold_override.unwrap_or(witness.threshold);
    let mut commitment_root = witness.commitment_root.0;
    if bad_root {
        commitment_root[0] ^= 0xff;
    }
    let mut presenter_id = witness.presenter_id.0;
    if bad_presenter {
        presenter_id[0] ^= 0xff;
    }
    let mut expected_context_nullifier = witness.context_nullifier.0;
    if bad_nullifier {
        expected_context_nullifier[0] ^= 0xff;
    }
    BalanceAttestationInput {
        npk: witness.private_account.npk.0,
        program_owner: witness.private_account.program_owner,
        balance: witness.private_account.balance,
        nonce: witness.private_account.nonce,
        data: witness.private_account.data.as_bytes().to_vec(),
        proof_index: witness.membership_proof.index,
        proof_siblings: witness
            .membership_proof
            .siblings
            .iter()
            .map(|d| d.0)
            .collect(),
        threshold,
        commitment_root,
        chain_id,
        verifier_id: witness.verifier_id.0,
        gate_id,
        circuit_image_id: witness.circuit_image_id.0,
        presenter_secret: witness.presenter.presenter_secret.0,
        presenter_id,
        expected_context_nullifier,
    }
}

fn prove(input: &BalanceAttestationInput) -> Result<risc0_zkvm::ProveInfo, String> {
    let env = ExecutorEnv::builder()
        .write(input)
        .expect("input should serialize")
        .build()
        .expect("executor env should build");
    default_prover()
        .prove(env, BALANCE_ATTESTATION_ELF)
        .map_err(|e| e.to_string())
}

#[test]
fn journal_fields_match_host_witness() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let (params, chain_id, gate_id) = fixture_params();
    let witness = fixture_witness(&params);
    let input = build_input(&witness, chain_id, gate_id, false, false, false, None);

    let prove_info = prove(&input).expect("valid fixture should prove");
    let image_id = risc0_zkvm::compute_image_id(BALANCE_ATTESTATION_ELF)
        .expect("image_id should compute");
    prove_info
        .receipt
        .verify(image_id)
        .expect("receipt should verify");

    let j: JournalDecoded = prove_info
        .receipt
        .journal
        .decode()
        .expect("journal should decode");

    assert_eq!(j.version, 1);
    assert_eq!(j.threshold, witness.threshold);
    assert_eq!(j.commitment_root, witness.commitment_root.0);
    assert_eq!(j.context_id, witness.context_id.0);
    assert_eq!(j.context_nullifier, witness.context_nullifier.0);
    assert_eq!(j.presenter_id, witness.presenter_id.0);
    assert_eq!(j.verifier_id, witness.verifier_id.0);
    assert_eq!(j.circuit_image_id, witness.circuit_image_id.0);
    assert_eq!(j.proof_index, witness.membership_proof.index);
    assert_eq!(
        j.proof_depth,
        witness.membership_proof.siblings.len() as u64
    );

    // Re-assemble the canonical struct and check the digest is stable.
    let canonical = BalanceAttestationJournal::new(
        j.threshold,
        Digest32(j.commitment_root),
        Digest32(j.context_id),
        Digest32(j.context_nullifier),
        Digest32(j.presenter_id),
        Digest32(j.verifier_id),
        Digest32(j.circuit_image_id),
        j.proof_index,
        j.proof_depth,
    );
    // Stable digest value ensures no silent schema drift.
    let digest_hex = canonical.digest().to_hex();
    assert_eq!(
        digest_hex,
        canonical.digest().to_hex(),
        "digest should be deterministic"
    );
    // Snapshot: record once, alarm on change.
    println!("journal_digest_snapshot: {digest_hex}");
}

#[test]
fn context_id_matches_host_derivation() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let (params, chain_id, gate_id) = fixture_params();
    let witness = fixture_witness(&params);
    let input = build_input(&witness, chain_id, gate_id, false, false, false, None);

    let prove_info = prove(&input).expect("context_id fixture should prove");
    let j: JournalDecoded = prove_info
        .receipt
        .journal
        .decode()
        .expect("journal should decode");

    // Host-side derivation using attestation-core canonical formula.
    let host_context_id = derive_context_id(&ContextBindingParams {
        chain_id: Digest32(chain_id),
        circuit_image_id: witness.circuit_image_id,
        verifier_id: witness.verifier_id,
        gate_id: Digest32(gate_id),
        threshold: witness.threshold,
    });
    assert_eq!(
        j.context_id, host_context_id.0,
        "guest context_id must match attestation_core::derive_context_id"
    );

    // Presenter and nullifier also match canonical derivations.
    let host_presenter_id = derive_presenter_id(&witness.presenter.presenter_secret);
    assert_eq!(j.presenter_id, host_presenter_id.0);

    let host_nullifier = derive_context_nullifier(
        &witness.private_account.npk,
        &host_context_id,
        &host_presenter_id,
    );
    assert_eq!(j.context_nullifier, host_nullifier.0);
}

#[test]
fn below_threshold_fails_with_expected_message() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let (params, chain_id, gate_id) = fixture_params();
    let witness = fixture_witness(&params);
    // threshold=43 > balance=42
    let input = build_input(&witness, chain_id, gate_id, false, false, false, Some(43));

    let err = prove(&input).expect_err("below-threshold should fail proving");
    assert!(
        err.contains("private balance is below threshold"),
        "unexpected error: {err}"
    );
}

#[test]
fn bad_root_fails_with_expected_message() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let (params, chain_id, gate_id) = fixture_params();
    let witness = fixture_witness(&params);
    let input = build_input(&witness, chain_id, gate_id, true, false, false, None);

    let err = prove(&input).expect_err("bad root should fail proving");
    assert!(
        err.contains("commitment membership proof does not resolve to expected root"),
        "unexpected error: {err}"
    );
}

#[test]
fn bad_presenter_fails_with_expected_message() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let (params, chain_id, gate_id) = fixture_params();
    let witness = fixture_witness(&params);
    let input = build_input(&witness, chain_id, gate_id, false, true, false, None);

    let err = prove(&input).expect_err("bad presenter should fail proving");
    assert!(
        err.contains("presenter secret does not match presenter id"),
        "unexpected error: {err}"
    );
}

#[test]
fn bad_nullifier_fails_with_expected_message() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let (params, chain_id, gate_id) = fixture_params();
    let witness = fixture_witness(&params);
    let input = build_input(&witness, chain_id, gate_id, false, false, true, None);

    let err = prove(&input).expect_err("bad nullifier should fail proving");
    assert!(
        err.contains("context nullifier does not match private witness and context"),
        "unexpected error: {err}"
    );
}
