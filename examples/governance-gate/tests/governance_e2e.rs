//! End-to-end governance-gate test (LP-0005 reference integration #1).
//!
//! Two voters with sufficient balance each generate one envelope; the program
//! accepts both and rejects a replay attempt with the same envelope.
//! A third voter with insufficient balance is rejected at proving time.
//!
//! Run with:
//!   RISC0_DEV_MODE=1 cargo test -p governance-gate-example -- --include-ignored

use attestation_core::{Digest32, HexBytes, LezMembershipProof, PresenterSecret};
use attestation_prover::{
    balance_attestation_image_id, build_balance_attestation_witness, prove_attestation,
    AttestationPublicParams, BalanceAttestationWitness, PresenterWitness, PrivateAccountWitness,
};
use attestation_verifier::VerifyError;
use governance_gate_example::{Choice, GateConfig, GovernanceProgram, SubmitError};

fn digest(seed: u8) -> Digest32 {
    Digest32([seed; 32])
}

fn gate() -> GateConfig {
    GateConfig {
        chain_id: digest(0xC1),
        circuit_image_id: Digest32(balance_attestation_image_id()),
        verifier_id: digest(0xC2),
        gate_id: digest(0xC3),
        threshold: 100,
    }
}

fn params_for(gate: &GateConfig) -> AttestationPublicParams {
    AttestationPublicParams {
        threshold: gate.threshold,
        chain_id: gate.chain_id,
        verifier_id: gate.verifier_id,
        gate_id: gate.gate_id,
        circuit_image_id: gate.circuit_image_id,
    }
}

fn witness_with(
    npk_seed: u8,
    presenter_seed: u8,
    balance: u128,
    gate: &GateConfig,
) -> BalanceAttestationWitness {
    build_balance_attestation_witness(
        PrivateAccountWitness {
            npk: digest(npk_seed),
            program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
            balance,
            nonce: 1,
            data: HexBytes::new(b"governance fixture".to_vec()),
        },
        LezMembershipProof {
            index: 0,
            siblings: vec![digest(0x11), digest(0x22), digest(0x33)],
        },
        PresenterWitness {
            presenter_secret: PresenterSecret::new([presenter_seed; 32]).unwrap(),
        },
        params_for(gate),
    )
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn two_distinct_voters_each_get_one_vote() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let mut program = GovernanceProgram::new(gate.clone());
    let params = params_for(&gate);

    // Voter A votes Yes.
    let witness_a = witness_with(0xAA, 0x77, 200, &gate);
    let envelope_a = prove_attestation(&witness_a, &params).expect("voter A proves");
    let nullifier_a = program
        .submit_vote(&envelope_a, Choice::Yes)
        .expect("voter A vote accepted");

    // Voter B votes No.
    let witness_b = witness_with(0xBB, 0x88, 150, &gate);
    let envelope_b = prove_attestation(&witness_b, &params).expect("voter B proves");
    let nullifier_b = program
        .submit_vote(&envelope_b, Choice::No)
        .expect("voter B vote accepted");

    assert_ne!(nullifier_a, nullifier_b);
    assert_eq!(program.tally(Choice::Yes), 1);
    assert_eq!(program.tally(Choice::No), 1);
    assert_eq!(program.total_votes(), 2);
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn replaying_the_same_envelope_is_rejected_as_nullifier_collision() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let mut program = GovernanceProgram::new(gate.clone());
    let params = params_for(&gate);

    let witness = witness_with(0xAA, 0x77, 200, &gate);
    let envelope = prove_attestation(&witness, &params).expect("voter proves");

    program
        .submit_vote(&envelope, Choice::Yes)
        .expect("first vote accepted");

    let err = program
        .submit_vote(&envelope, Choice::Yes)
        .expect_err("replay must be rejected");
    assert!(matches!(err, SubmitError::NullifierReplay { .. }));
    assert_eq!(program.tally(Choice::Yes), 1, "tally must not double-count");
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn envelope_for_different_gate_is_rejected_with_context_mismatch() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let real_gate = gate();
    // Voter generates an envelope bound to a DIFFERENT gate_id than what the
    // program will check against.
    let other_gate = GateConfig {
        gate_id: digest(0xFF),
        ..real_gate.clone()
    };
    let voter_params = params_for(&other_gate);
    let witness = witness_with(0xAA, 0x77, 200, &other_gate);
    let envelope = prove_attestation(&witness, &voter_params).expect("voter proves");

    let mut program = GovernanceProgram::new(real_gate);
    let err = program
        .submit_vote(&envelope, Choice::Yes)
        .expect_err("wrong gate must be rejected");
    match err {
        SubmitError::Verify(VerifyError::ContextMismatch { .. }) => {}
        other => panic!("expected ContextMismatch, got: {other:?}"),
    }
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn underfunded_voter_cannot_even_produce_an_envelope() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let params = params_for(&gate);
    // Balance below threshold (100). The circuit asserts and proving fails.
    let witness = witness_with(0xAA, 0x77, 50, &gate);
    let err = prove_attestation(&witness, &params)
        .expect_err("proving must fail when balance < threshold");
    assert!(
        format!("{err}").contains("private balance is below threshold"),
        "unexpected error: {err}"
    );
}
