//! End-to-end test for the LEZ on-chain gate (Spike 0C track).
//!
//! Builds an off-chain envelope (inner receipt) → wraps it via
//! `prove_lez_gate` (outer receipt) → submits the outer receipt to
//! `LezGateProgram::admit`. Demonstrates:
//!
//! 1. Successful admission for a valid envelope at the pinned gate.
//! 2. Replaying the same outer receipt is rejected by the program's
//!    nullifier dedup.
//! 3. An outer receipt produced under a DIFFERENT gate is rejected with
//!    GateContextMismatch.
//!
//! Run with:
//!   RISC0_DEV_MODE=1 cargo test -p lez-verifier --test onchain_e2e -- --include-ignored

use attestation_core::{Digest32, HexBytes, LezMembershipProof, PresenterSecret};
use attestation_prover::{
    build_balance_attestation_witness, prove_attestation, AttestationPublicParams,
    PresenterWitness, PrivateAccountWitness,
};
use lez_verifier::{
    balance_attestation_inner_image_id, prove_lez_gate, LezGateConfig, LezGateProgram,
    LezGateProgramError,
};

fn digest(seed: u8) -> Digest32 {
    Digest32([seed; 32])
}

fn gate() -> LezGateConfig {
    LezGateConfig {
        chain_id: digest(0xD1),
        verifier_id: digest(0xD2),
        gate_id: digest(0xD3),
        threshold: 50,
    }
}

fn build_envelope(
    presenter_seed: u8,
    npk_seed: u8,
    balance: u128,
    gate: &LezGateConfig,
) -> attestation_core::BalanceAttestationEnvelope {
    let params = AttestationPublicParams {
        threshold: gate.threshold,
        chain_id: gate.chain_id,
        verifier_id: gate.verifier_id,
        gate_id: gate.gate_id,
        circuit_image_id: balance_attestation_inner_image_id(),
    };
    let witness = build_balance_attestation_witness(
        PrivateAccountWitness {
            npk: digest(npk_seed),
            program_owner: [3; 8],
            balance,
            nonce: 1,
            data: HexBytes::new(b"lez-gate fixture".to_vec()),
        },
        LezMembershipProof {
            index: 0,
            siblings: vec![digest(0xAB), digest(0xCD)],
        },
        PresenterWitness {
            presenter_secret: PresenterSecret::new([presenter_seed; 32]).unwrap(),
        },
        params,
    );
    prove_attestation(&witness, &params, digest(0x44)).expect("inner prove succeeds")
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1; run with: RISC0_DEV_MODE=1 cargo test -p lez-verifier -- --include-ignored"]
fn admits_valid_envelope_via_outer_receipt() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let inner = build_envelope(0x77, 0xAA, 100, &gate);
    let outer = prove_lez_gate(&inner, &gate).expect("outer prove succeeds");

    let mut program = LezGateProgram::new(gate);
    let nullifier = program
        .admit(&outer, inner.journal.presenter_id)
        .expect("program should admit");

    assert_eq!(nullifier, inner.journal.context_nullifier);
    assert_eq!(program.admitted_count(), 1);
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn rejects_replay_with_same_outer_receipt() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let inner = build_envelope(0x77, 0xAA, 100, &gate);
    let outer = prove_lez_gate(&inner, &gate).expect("outer prove succeeds");

    let mut program = LezGateProgram::new(gate);
    program
        .admit(&outer, inner.journal.presenter_id)
        .expect("first admission OK");

    let err = program
        .admit(&outer, inner.journal.presenter_id)
        .expect_err("replay must be rejected");
    assert!(matches!(err, LezGateProgramError::NullifierReplay { .. }));
    assert_eq!(program.admitted_count(), 1);
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn rejects_outer_receipt_for_different_gate() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    // Voter built an envelope + outer receipt bound to other_gate.
    let other_gate = LezGateConfig {
        gate_id: digest(0xFF),
        ..gate()
    };
    let inner = build_envelope(0x77, 0xAA, 100, &other_gate);
    let outer = prove_lez_gate(&inner, &other_gate).expect("outer prove succeeds");

    // Program is pinned to the canonical gate.
    let mut program = LezGateProgram::new(gate());
    let err = program
        .admit(&outer, inner.journal.presenter_id)
        .expect_err("wrong gate must be rejected");
    assert!(matches!(
        err,
        LezGateProgramError::GateContextMismatch { .. }
    ));
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn rejects_forwarded_receipt_from_wrong_presenter() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let inner = build_envelope(0x77, 0xAA, 100, &gate);
    let outer = prove_lez_gate(&inner, &gate).expect("outer prove succeeds");

    let mut program = LezGateProgram::new(gate);
    let err = program
        .admit(&outer, digest(0x99))
        .expect_err("wrong presenter signer must be rejected");
    assert!(matches!(
        err,
        LezGateProgramError::PresenterSignerMismatch { .. }
    ));
    assert_eq!(program.admitted_count(), 0);
}
