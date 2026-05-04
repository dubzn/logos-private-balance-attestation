//! Emit deterministic demo fixtures (`witness.json` + `gate.json`) for the
//! end-to-end demo script. Both files are written to the directory passed as
//! the only CLI arg (default: ./fixtures/demo).
//!
//! The witness contains derived fields (commitment_root, context_id,
//! presenter_pubkey, presenter_id, context_nullifier) computed via the SDK.
//! The gate is the matching verifier-side view: chain_id + verifier_id +
//! gate_id + presentation_challenge + threshold (the circuit_image_id is
//! always pinned to the compiled BALANCE_ATTESTATION_ID by the verifier).

use std::env;
use std::fs;
use std::path::PathBuf;

use attestation_sdk::{
    balance_attestation_image_id, build_balance_attestation_witness, AttestationPublicParams,
    Digest32, HexBytes, LezMembershipProof, PresenterSecret, PresenterWitness,
    PrivateAccountWitness,
};
use serde::Serialize;

#[derive(Serialize)]
struct ProveInput<'a> {
    witness: &'a attestation_sdk::BalanceAttestationWitness,
    chain_id: Digest32,
    gate_id: Digest32,
    presentation_challenge: Digest32,
}

#[derive(Serialize)]
struct GateFile {
    chain_id: Digest32,
    verifier_id: Digest32,
    gate_id: Digest32,
    presentation_challenge: Digest32,
    /// Decimal string — the CLI's `verify` parser expects a string-encoded u128.
    threshold: String,
}

fn digest(seed: u8) -> Digest32 {
    Digest32([seed; 32])
}

fn main() {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fixtures/demo"));
    fs::create_dir_all(&out_dir).expect("create out dir");

    let chain_id = digest(0x10);
    let verifier_id = digest(0x20);
    let gate_id = digest(0x30);
    let presentation_challenge = digest(0x44);
    let threshold: u128 = 100;

    let params = AttestationPublicParams {
        threshold,
        chain_id,
        verifier_id,
        gate_id,
        circuit_image_id: Digest32(balance_attestation_image_id()),
    };

    let witness = build_balance_attestation_witness(
        PrivateAccountWitness {
            npk: digest(0x07),
            program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
            balance: 250,
            nonce: 42,
            data: HexBytes::new(b"demo private account".to_vec()),
        },
        LezMembershipProof {
            index: 3,
            siblings: vec![digest(0xA1), digest(0xA2), digest(0xA3), digest(0xA4)],
        },
        PresenterWitness {
            presenter_secret: PresenterSecret::new([0x55; 32]).expect("valid demo secret"),
        },
        params,
    );

    let prove_input = ProveInput {
        witness: &witness,
        chain_id,
        gate_id,
        presentation_challenge,
    };
    let gate_file = GateFile {
        chain_id,
        verifier_id,
        gate_id,
        presentation_challenge,
        threshold: threshold.to_string(),
    };

    let witness_path = out_dir.join("witness.json");
    let gate_path = out_dir.join("gate.json");
    fs::write(
        &witness_path,
        serde_json::to_string_pretty(&prove_input).expect("witness ser"),
    )
    .expect("write witness");
    fs::write(
        &gate_path,
        serde_json::to_string_pretty(&gate_file).expect("gate ser"),
    )
    .expect("write gate");

    println!("wrote {}", witness_path.display());
    println!("wrote {}", gate_path.display());
}
