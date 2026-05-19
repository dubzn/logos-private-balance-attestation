//! End-to-end fee-tier gate test (LP-0005 reference integration #3).
//!
//! A user proves enough private balance to unlock a fee tier without exposing
//! their exact balance. The program records only presenter id, tier, and
//! nullifier state.
//!
//! Run with:
//!   RISC0_DEV_MODE=1 cargo test -p fee-tier-gate-example -- --include-ignored

use attestation_core::{Digest32, HexBytes, LezMembershipProof, PresenterSecret};
use attestation_prover::{
    balance_attestation_image_id, build_balance_attestation_witness, prove_attestation,
    AttestationPublicParams, BalanceAttestationWitness, PresenterWitness, PrivateAccountWitness,
};
use attestation_verifier::VerifyError;
use fee_tier_gate_example::{ClaimError, FeeTier, FeeTierGate, FeeTierProgram};

fn digest(seed: u8) -> Digest32 {
    Digest32([seed; 32])
}

fn gate() -> FeeTierGate {
    FeeTierGate {
        chain_id: digest(0xD1),
        circuit_image_id: Digest32(balance_attestation_image_id()),
        verifier_id: digest(0xD2),
        gate_id: digest(0xD3),
        presentation_challenge: digest(0xD4),
    }
}

fn params(gate: &FeeTierGate, tier: FeeTier) -> AttestationPublicParams {
    AttestationPublicParams {
        threshold: tier.threshold(),
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
    gate: &FeeTierGate,
    tier: FeeTier,
) -> BalanceAttestationWitness {
    build_balance_attestation_witness(
        PrivateAccountWitness {
            account_id: digest(npk_seed.wrapping_sub(1)),
            npk: digest(npk_seed),
            program_owner: [3, 1, 4, 1, 5, 9, 2, 6],
            balance,
            nonce: 1,
            data: HexBytes::new(b"fee tier fixture".to_vec()),
        },
        LezMembershipProof {
            index: 0,
            siblings: vec![digest(0x51), digest(0x52), digest(0x53)],
        },
        PresenterWitness {
            presenter_secret: PresenterSecret::new([presenter_seed; 32]).unwrap(),
        },
        params(gate, tier),
    )
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn gold_holder_claims_lower_fee_without_revealing_balance() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let mut program = FeeTierProgram::new(gate.clone());
    let tier = FeeTier::Gold;
    let envelope = prove_attestation(
        &witness(0xAA, 0x77, 900, &gate, tier),
        &params(&gate, tier),
        gate.presentation_challenge,
    )
    .expect("gold holder proves");

    let pass = program
        .claim_tier(&envelope, tier)
        .expect("gold claim accepted");
    assert_eq!(pass.tier, FeeTier::Gold);
    assert_eq!(pass.fee_bps, FeeTier::Gold.fee_bps());
    assert_eq!(
        program.fee_bps_for(pass.presenter_id),
        FeeTier::Gold.fee_bps()
    );
    assert_eq!(program.claimed_count(), 1);
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn silver_proof_cannot_claim_gold_context() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let silver = FeeTier::Silver;
    let envelope = prove_attestation(
        &witness(0xAA, 0x77, 200, &gate, silver),
        &params(&gate, silver),
        gate.presentation_challenge,
    )
    .expect("silver holder proves");

    let mut program = FeeTierProgram::new(gate);
    let err = program
        .claim_tier(&envelope, FeeTier::Gold)
        .expect_err("silver context cannot claim gold");
    match err {
        ClaimError::Verify(VerifyError::ContextMismatch { .. }) => {}
        other => panic!("expected ContextMismatch, got {other:?}"),
    }
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn replaying_same_tier_claim_is_rejected() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let tier = FeeTier::Silver;
    let envelope = prove_attestation(
        &witness(0xAA, 0x77, 250, &gate, tier),
        &params(&gate, tier),
        gate.presentation_challenge,
    )
    .expect("silver holder proves");

    let mut program = FeeTierProgram::new(gate);
    program
        .claim_tier(&envelope, tier)
        .expect("first claim accepted");

    let err = program
        .claim_tier(&envelope, tier)
        .expect_err("same proof replay rejected");
    assert!(matches!(err, ClaimError::NullifierReplay { .. }));
}

#[test]
#[ignore = "requires RISC0_DEV_MODE=1"]
fn presenter_can_upgrade_from_silver_to_gold() {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let gate = gate();
    let mut program = FeeTierProgram::new(gate.clone());

    let silver = FeeTier::Silver;
    let silver_envelope = prove_attestation(
        &witness(0xAA, 0x77, 600, &gate, silver),
        &params(&gate, silver),
        gate.presentation_challenge,
    )
    .expect("silver proof succeeds");
    let silver_pass = program
        .claim_tier(&silver_envelope, silver)
        .expect("silver claim accepted");
    assert_eq!(
        program.best_tier_for(silver_pass.presenter_id),
        FeeTier::Silver
    );

    let gold = FeeTier::Gold;
    let gold_envelope = prove_attestation(
        &witness(0xAA, 0x77, 600, &gate, gold),
        &params(&gate, gold),
        gate.presentation_challenge,
    )
    .expect("gold proof succeeds");
    let gold_pass = program
        .claim_tier(&gold_envelope, gold)
        .expect("upgrade accepted");

    assert_eq!(silver_pass.presenter_id, gold_pass.presenter_id);
    assert_eq!(program.best_tier_for(gold_pass.presenter_id), FeeTier::Gold);
}

#[test]
fn standard_tier_does_not_require_proof() {
    let gate = gate();
    let mut program = FeeTierProgram::new(gate);
    assert_eq!(
        program.fee_bps_for(digest(0x99)),
        FeeTier::Standard.fee_bps()
    );

    let err = program
        .claim_tier(
            &attestation_core::BalanceAttestationEnvelope {
                version: attestation_core::ENVELOPE_VERSION,
                proof_system: attestation_core::ProofSystem::Risc0,
                image_id: digest(0),
                journal: attestation_core::BalanceAttestationJournal::new(
                    0,
                    digest(0),
                    digest(0),
                    digest(0),
                    digest(0),
                    digest(0),
                    digest(0),
                    0,
                    0,
                ),
                receipt: attestation_core::HexBytes::new(vec![]),
                presenter_pubkey: attestation_core::HexBytes::new(vec![]),
                presentation_challenge: digest(0),
                presenter_signature: attestation_core::HexBytes::new(vec![]),
            },
            FeeTier::Standard,
        )
        .expect_err("standard tier should not ask for proof");

    assert!(matches!(err, ClaimError::StandardDoesNotNeedProof));
}
