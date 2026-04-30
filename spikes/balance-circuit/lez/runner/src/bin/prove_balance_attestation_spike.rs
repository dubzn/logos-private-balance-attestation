use nssa::AccountId;
use nssa_core::{
    account::{Account, Data, Nonce},
    compute_digest_for_path, Commitment, MembershipProof, NullifierPublicKey,
};
use risc0_zkvm::{compute_image_id, default_prover, ExecutorEnv};
use serde::{Deserialize, Serialize};
use wallet::WalletCore;

#[derive(Serialize, Deserialize)]
struct BalanceAttestationInput {
    npk: NullifierPublicKey,
    account: Account,
    membership_proof: MembershipProof,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize)]
struct BalanceAttestationJournal {
    version: u32,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
    commitment: [u8; 32],
    proof_index: usize,
    proof_depth: usize,
}

#[derive(Debug, Serialize)]
struct ProveReport {
    mode: String,
    proved: bool,
    verified: bool,
    expected_failure_observed: bool,
    failure_contains: Option<String>,
    journal: Option<JournalReport>,
}

#[derive(Debug, Serialize)]
struct JournalReport {
    version: u32,
    threshold: String,
    commitment_hex: String,
    commitment_root_hex: String,
    context_id_hex: String,
    proof_index: usize,
    proof_depth: usize,
}

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(program_path) = args.first() else {
        usage_and_exit();
    };
    let mode = args.get(1).map(String::as_str).unwrap_or("fixture-valid");
    let elf = std::fs::read(program_path).expect("guest ELF should be readable");

    let result = match mode {
        "fixture-valid" => run_expected_success(&elf, mode, fixture_input(25, false)),
        "fixture-below-threshold" => run_expected_failure(
            &elf,
            mode,
            fixture_input(43, false),
            "private balance is below threshold",
        ),
        "fixture-bad-root" => run_expected_failure(
            &elf,
            mode,
            fixture_input(25, true),
            "commitment membership proof does not resolve to expected root",
        ),
        "live" => {
            let Some(account_id) = args.get(2) else {
                eprintln!("live mode requires <private-account-id-without-Private>");
                std::process::exit(2);
            };
            let threshold = args
                .get(3)
                .map_or(Ok(1_u128), |value| value.parse::<u128>())
                .expect("threshold should be a u128");
            run_expected_success(&elf, mode, live_input(account_id, threshold).await)
        }
        "live-below-threshold" => {
            let Some(account_id) = args.get(2) else {
                eprintln!(
                    "live-below-threshold mode requires <private-account-id-without-Private>"
                );
                std::process::exit(2);
            };
            let threshold = args
                .get(3)
                .map_or(Ok(1_u128), |value| value.parse::<u128>())
                .expect("threshold should be a u128");
            run_expected_failure(
                &elf,
                mode,
                live_input(account_id, threshold).await,
                "private balance is below threshold",
            )
        }
        _ => usage_and_exit(),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&result).expect("report should serialize")
    );

    if !(result.proved && result.verified || result.expected_failure_observed) {
        std::process::exit(1);
    }
}

fn usage_and_exit() -> ! {
    eprintln!(
        "usage: prove_balance_attestation_spike <guest-elf> \
         [fixture-valid|fixture-below-threshold|fixture-bad-root|live|live-below-threshold \
         <private-account> <threshold>]"
    );
    std::process::exit(2);
}

fn run_expected_success(elf: &[u8], mode: &str, input: BalanceAttestationInput) -> ProveReport {
    let prove_info = prove(elf, input).expect("expected proving to succeed");
    let image_id = compute_image_id(elf).expect("image id should be computed from ELF");
    prove_info
        .receipt
        .verify(image_id)
        .expect("receipt should verify");
    let journal: BalanceAttestationJournal = prove_info
        .receipt
        .journal
        .decode()
        .expect("journal should decode");

    ProveReport {
        mode: mode.to_owned(),
        proved: true,
        verified: true,
        expected_failure_observed: false,
        failure_contains: None,
        journal: Some(JournalReport::from(journal)),
    }
}

fn run_expected_failure(
    elf: &[u8],
    mode: &str,
    input: BalanceAttestationInput,
    expected_error: &str,
) -> ProveReport {
    match prove(elf, input) {
        Ok(_) => ProveReport {
            mode: mode.to_owned(),
            proved: true,
            verified: false,
            expected_failure_observed: false,
            failure_contains: None,
            journal: None,
        },
        Err(error) => {
            let error = error.to_string();
            ProveReport {
                mode: mode.to_owned(),
                proved: false,
                verified: false,
                expected_failure_observed: error.contains(expected_error),
                failure_contains: Some(error),
                journal: None,
            }
        }
    }
}

fn prove(elf: &[u8], input: BalanceAttestationInput) -> Result<risc0_zkvm::ProveInfo, String> {
    let env = ExecutorEnv::builder()
        .write(&input)
        .expect("input should serialize")
        .build()
        .expect("executor env should build");

    default_prover()
        .prove(env, elf)
        .map_err(|error| error.to_string())
}

fn fixture_input(threshold: u128, bad_root: bool) -> BalanceAttestationInput {
    let npk = NullifierPublicKey([7; 32]);
    let account = Account {
        program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
        balance: 42,
        data: Data::try_from(b"balance-attestation-spike-fixture".to_vec())
            .expect("fixture data should fit"),
        nonce: Nonce(123_456),
    };
    let commitment = Commitment::new(&npk, &account);
    let membership_proof = (
        5_usize,
        vec![[0x11; 32], [0x22; 32], [0x33; 32], [0x44; 32]],
    );
    let mut commitment_root = compute_digest_for_path(&commitment, &membership_proof);
    if bad_root {
        commitment_root[0] ^= 0xff;
    }

    BalanceAttestationInput {
        npk,
        account,
        membership_proof,
        threshold,
        commitment_root,
        context_id: [0x55; 32],
    }
}

async fn live_input(account_id_raw: &str, threshold: u128) -> BalanceAttestationInput {
    let account_id: AccountId = account_id_raw
        .parse()
        .expect("private account id should be valid base58 without Private/ prefix");
    let wallet_core = WalletCore::from_env().expect("wallet should initialize from env");
    let (key_chain, account) = wallet_core
        .storage()
        .user_data
        .get_private_account(account_id)
        .expect("private account should exist in wallet storage");
    let commitment = Commitment::new(&key_chain.nullifier_public_key, account);
    let membership_proof = wallet_core
        .check_private_account_initialized(account_id)
        .await
        .expect("sequencer call should not fail")
        .expect("sequencer should return membership proof for private commitment");
    let commitment_root = compute_digest_for_path(&commitment, &membership_proof);

    BalanceAttestationInput {
        npk: key_chain.nullifier_public_key.clone(),
        account: account.clone(),
        membership_proof,
        threshold,
        commitment_root,
        context_id: [0x66; 32],
    }
}

impl From<BalanceAttestationJournal> for JournalReport {
    fn from(value: BalanceAttestationJournal) -> Self {
        Self {
            version: value.version,
            threshold: value.threshold.to_string(),
            commitment_hex: hex::encode(value.commitment),
            commitment_root_hex: hex::encode(value.commitment_root),
            context_id_hex: hex::encode(value.context_id),
            proof_index: value.proof_index,
            proof_depth: value.proof_depth,
        }
    }
}
