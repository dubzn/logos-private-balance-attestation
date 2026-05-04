//! End-to-end chat-gate test (LP-0005 reference integration #2 — off-chain path).
//!
//! Demonstrates the full off-chain flow:
//!   prove → serialize envelope → ship over "wire" → host verifies → admit.
//!
//! Run with:
//!   RISC0_DEV_MODE=1 cargo test -p chat-gate-example -- --include-ignored

use attestation_core::{Digest32, HexBytes, LezMembershipProof, PresenterSecret};
use attestation_prover::{
    balance_attestation_image_id, build_balance_attestation_witness, prove_attestation,
    AttestationPublicParams, BalanceAttestationWitness, PresenterWitness, PrivateAccountWitness,
};
use attestation_verifier::VerifyError;
use chat_gate_example::{pack_envelope_for_wire, AdmissionError, ChatRoomGate, ChatRoomHost};

fn digest(seed: u8) -> Digest32 {
    Digest32([seed; 32])
}

fn gate() -> ChatRoomGate {
    ChatRoomGate {
        chain_id: digest(0xC4),
        circuit_image_id: Digest32(balance_attestation_image_id()),
        verifier_id: digest(0xC5),
        gate_id: digest(0xC6),
        presentation_challenge: digest(0xC7),
        min_balance: 50,
    }
}

fn params(gate: &ChatRoomGate) -> AttestationPublicParams {
    AttestationPublicParams {
        threshold: gate.min_balance,
        chain_id: gate.chain_id,
        verifier_id: gate.verifier_id,
        gate_id: gate.gate_id,
        circuit_image_id: gate.circuit_image_id,
    }
}

fn witness(
    npk_seed: u8,
    presenter_seed: u8,
    balance: u128,
    gate: &ChatRoomGate,
) -> BalanceAttestationWitness {
    build_balance_attestation_witness(
        PrivateAccountWitness {
            npk: digest(npk_seed),
            program_owner: [9, 9, 9, 9, 9, 9, 9, 9],
            balance,
            nonce: 1,
            data: HexBytes::new(b"chat fixture".to_vec()),
        },
        LezMembershipProof {
            index: 0,
            siblings: vec![digest(0x55), digest(0x66)],
        },
        PresenterWitness {
            presenter_secret: PresenterSecret::new([presenter_seed; 32]).unwrap(),
        },
        params(gate),
    )
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn admits_member_from_wire_envelope() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let mut host = ChatRoomHost::new(gate.clone());

    // Prover side: build envelope and serialize to wire bytes.
    let envelope = prove_attestation(
        &witness(0xAA, 0x77, 100, &gate),
        &params(&gate),
        gate.presentation_challenge,
    )
    .expect("prove should succeed");
    let wire = pack_envelope_for_wire(&envelope);

    // Host side: receives bytes (e.g., over Logos Messaging) and admits.
    let token = host
        .admit_from_wire(&wire)
        .expect("host should admit valid envelope");
    assert!(host.is_admitted(&token));
    assert_eq!(host.member_count(), 1);
    assert_eq!(
        host.nullifier_for(&token),
        Some(envelope.journal.context_nullifier)
    );
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn rejects_replay_with_same_envelope_bytes() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let mut host = ChatRoomHost::new(gate.clone());

    let envelope = prove_attestation(
        &witness(0xAA, 0x77, 100, &gate),
        &params(&gate),
        gate.presentation_challenge,
    )
    .expect("prove should succeed");
    let wire = pack_envelope_for_wire(&envelope);

    host.admit_from_wire(&wire).expect("first admission OK");

    let err = host
        .admit_from_wire(&wire)
        .expect_err("replay must be rejected");
    assert!(matches!(err, AdmissionError::AlreadyAdmitted { .. }));
    assert_eq!(host.member_count(), 1);
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn rejects_envelope_for_a_different_room() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let host_gate = gate();
    let other_gate = ChatRoomGate {
        gate_id: digest(0xEE),
        ..host_gate.clone()
    };

    // Prover bound the envelope to other_gate, not host_gate.
    let envelope = prove_attestation(
        &witness(0xAA, 0x77, 100, &other_gate),
        &params(&other_gate),
        other_gate.presentation_challenge,
    )
    .expect("prove should succeed");
    let wire = pack_envelope_for_wire(&envelope);

    let mut host = ChatRoomHost::new(host_gate);
    let err = host
        .admit_from_wire(&wire)
        .expect_err("wrong room must be rejected");
    match err {
        AdmissionError::Verify(VerifyError::ContextMismatch { .. }) => {}
        other => panic!("expected ContextMismatch, got {other:?}"),
    }
    assert_eq!(host.member_count(), 0);
}

#[test]
fn rejects_malformed_wire_bytes() {
    let gate = gate();
    let mut host = ChatRoomHost::new(gate);
    let err = host
        .admit_from_wire(b"not json at all")
        .expect_err("must reject garbage bytes");
    assert!(matches!(err, AdmissionError::MalformedWire(_)));
}
