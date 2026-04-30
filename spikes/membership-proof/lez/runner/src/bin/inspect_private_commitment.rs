use nssa::AccountId;
use nssa_core::compute_digest_for_path;
use serde::Serialize;
use wallet::WalletCore;

#[derive(Debug, Serialize)]
struct MembershipProofReport {
    account_id: String,
    private_state_found: bool,
    commitment_hex: String,
    membership_proof_found: bool,
    proof_index: Option<usize>,
    proof_depth: Option<usize>,
    commitment_root_hex: Option<String>,
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let Some(account_id_raw) = args.next() else {
        eprintln!("usage: inspect_private_commitment <private-account-id-without-Private> [--require-proof]");
        std::process::exit(2);
    };
    let require_proof = args.any(|arg| arg == "--require-proof");

    let account_id: AccountId = account_id_raw
        .parse()
        .expect("private account id should be valid base58 without Private/ prefix");

    let wallet_core = WalletCore::from_env().expect("wallet should initialize from env");
    let private_state_found = wallet_core.get_account_private(account_id).is_some();
    let commitment = wallet_core
        .get_private_account_commitment(account_id)
        .expect("private account should exist in local wallet storage");
    let proof = wallet_core
        .check_private_account_initialized(account_id)
        .await
        .expect("sequencer getProofForCommitment call should succeed");

    let (proof_index, proof_depth, commitment_root_hex) = proof.as_ref().map_or(
        (None, None, None),
        |membership_proof| {
            let root = compute_digest_for_path(&commitment, membership_proof);
            (
                Some(membership_proof.0),
                Some(membership_proof.1.len()),
                Some(hex::encode(root)),
            )
        },
    );

    let report = MembershipProofReport {
        account_id: format!("Private/{account_id}"),
        private_state_found,
        commitment_hex: hex::encode(commitment.to_byte_array()),
        membership_proof_found: proof.is_some(),
        proof_index,
        proof_depth,
        commitment_root_hex,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report should serialize")
    );

    if require_proof && proof.is_none() {
        eprintln!("membership proof was not found for the current private commitment");
        std::process::exit(1);
    }
}

