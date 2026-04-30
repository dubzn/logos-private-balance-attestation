use nssa::AccountId;
use nssa_core::{
    account::{Account, Data, Nonce},
    compute_digest_for_path, Commitment, MembershipProof, NullifierPublicKey,
};
use risc0_zkvm::{
    compute_image_id, default_prover,
    sha::{Impl, Sha256 as _},
    Digest, ExecutorEnv,
};
use serde::{Deserialize, Serialize};
use wallet::WalletCore;

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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
struct ProveReport {
    mode: String,
    proved: bool,
    verified: bool,
    expected_failure_observed: bool,
    failure_contains: Option<String>,
    journal: Option<JournalReport>,
    comparison: Option<ComparisonReport>,
}

#[derive(Debug, Serialize)]
struct JournalReport {
    version: u32,
    threshold: String,
    commitment_root_hex: String,
    context_id_hex: String,
    context_nullifier_hex: String,
    presenter_id_hex: String,
    verifier_id_hex: String,
    circuit_image_id_hex: String,
    proof_index: usize,
    proof_depth: usize,
}

#[derive(Debug, Serialize)]
struct ComparisonReport {
    context_ids_differ: bool,
    nullifiers_differ: bool,
    first_context_id_hex: String,
    second_context_id_hex: String,
    first_nullifier_hex: String,
    second_nullifier_hex: String,
}

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(program_path) = args.first() else {
        usage_and_exit();
    };
    let mode = args.get(1).map(String::as_str).unwrap_or("fixture-valid");
    let elf = std::fs::read(program_path).expect("guest ELF should be readable");
    let image_id = digest_to_bytes(&compute_image_id(&elf).expect("image id should compute"));

    let result = match mode {
        "fixture-valid" => run_expected_success(
            &elf,
            mode,
            fixture_input(image_id, 25, false, false, false, 0x30),
        ),
        "fixture-below-threshold" => run_expected_failure(
            &elf,
            mode,
            fixture_input(image_id, 43, false, false, false, 0x30),
            "private balance is below threshold",
        ),
        "fixture-bad-root" => run_expected_failure(
            &elf,
            mode,
            fixture_input(image_id, 25, true, false, false, 0x30),
            "commitment membership proof does not resolve to expected root",
        ),
        "fixture-bad-presenter" => run_expected_failure(
            &elf,
            mode,
            fixture_input(image_id, 25, false, true, false, 0x30),
            "presenter secret does not match presenter id",
        ),
        "fixture-bad-nullifier" => run_expected_failure(
            &elf,
            mode,
            fixture_input(image_id, 25, false, false, true, 0x30),
            "context nullifier does not match private witness and context",
        ),
        "fixture-context-variant" => run_context_variant(&elf, image_id),
        "live" => {
            let Some(account_id) = args.get(2) else {
                eprintln!("live mode requires <private-account-id-without-Private>");
                std::process::exit(2);
            };
            let threshold = args
                .get(3)
                .map_or(Ok(1_u128), |value| value.parse::<u128>())
                .expect("threshold should be a u128");
            run_expected_success(
                &elf,
                mode,
                live_input(account_id, threshold, image_id).await,
            )
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
                live_input(account_id, threshold, image_id).await,
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
        "usage: prove_binding_attestation_spike <guest-elf> \
         [fixture-valid|fixture-below-threshold|fixture-bad-root|fixture-bad-presenter|\
         fixture-bad-nullifier|fixture-context-variant|live|live-below-threshold \
         <private-account> <threshold>]"
    );
    std::process::exit(2);
}

fn run_expected_success(elf: &[u8], mode: &str, input: BindingAttestationInput) -> ProveReport {
    let prove_info = prove(elf, input).expect("expected proving to succeed");
    let image_id = compute_image_id(elf).expect("image id should be computed from ELF");
    prove_info
        .receipt
        .verify(image_id)
        .expect("receipt should verify");
    let journal: BindingAttestationJournal = prove_info
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
        comparison: None,
    }
}

fn run_expected_failure(
    elf: &[u8],
    mode: &str,
    input: BindingAttestationInput,
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
            comparison: None,
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
                comparison: None,
            }
        }
    }
}

fn run_context_variant(elf: &[u8], image_id: [u8; 32]) -> ProveReport {
    let first = prove_and_decode(elf, fixture_input(image_id, 25, false, false, false, 0x30));
    let second = prove_and_decode(elf, fixture_input(image_id, 25, false, false, false, 0x31));

    let context_ids_differ = first.context_id != second.context_id;
    let nullifiers_differ = first.context_nullifier != second.context_nullifier;
    let verified = context_ids_differ && nullifiers_differ;

    ProveReport {
        mode: "fixture-context-variant".to_owned(),
        proved: true,
        verified,
        expected_failure_observed: false,
        failure_contains: None,
        journal: Some(JournalReport::from(second.clone_for_report())),
        comparison: Some(ComparisonReport {
            context_ids_differ,
            nullifiers_differ,
            first_context_id_hex: hex::encode(first.context_id),
            second_context_id_hex: hex::encode(second.context_id),
            first_nullifier_hex: hex::encode(first.context_nullifier),
            second_nullifier_hex: hex::encode(second.context_nullifier),
        }),
    }
}

fn prove_and_decode(elf: &[u8], input: BindingAttestationInput) -> BindingAttestationJournal {
    let prove_info = prove(elf, input).expect("expected context variant proving to succeed");
    let image_id = compute_image_id(elf).expect("image id should be computed from ELF");
    prove_info
        .receipt
        .verify(image_id)
        .expect("context variant receipt should verify");
    prove_info
        .receipt
        .journal
        .decode()
        .expect("context variant journal should decode")
}

fn prove(elf: &[u8], input: BindingAttestationInput) -> Result<risc0_zkvm::ProveInfo, String> {
    let env = ExecutorEnv::builder()
        .write(&input)
        .expect("input should serialize")
        .build()
        .expect("executor env should build");

    default_prover()
        .prove(env, elf)
        .map_err(|error| error.to_string())
}

fn fixture_input(
    image_id: [u8; 32],
    threshold: u128,
    bad_root: bool,
    bad_presenter: bool,
    bad_nullifier: bool,
    gate_seed: u8,
) -> BindingAttestationInput {
    // Synthetic fixture values: these are stable test vectors, not real wallet state.
    let npk = NullifierPublicKey([7; 32]);
    let account = Account {
        program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
        balance: 42,
        data: Data::try_from(b"binding-attestation-spike-fixture".to_vec())
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

    let chain_id = [0x10; 32];
    let verifier_id = [0x20; 32];
    let gate_id = [gate_seed; 32];
    let presenter_secret = [0x42; 32];
    let mut presenter_id = derive_presenter_id(&presenter_secret);
    if bad_presenter {
        presenter_id[0] ^= 0xff;
    }
    let context_id = derive_context_id(threshold, &chain_id, &verifier_id, &gate_id, &image_id);
    let mut expected_context_nullifier = derive_context_nullifier(&npk, &context_id, &presenter_id);
    if bad_nullifier {
        expected_context_nullifier[0] ^= 0xff;
    }

    BindingAttestationInput {
        npk,
        account,
        membership_proof,
        threshold,
        commitment_root,
        chain_id,
        verifier_id,
        gate_id,
        circuit_image_id: image_id,
        presenter_secret,
        presenter_id,
        expected_context_nullifier,
    }
}

async fn live_input(
    account_id_raw: &str,
    threshold: u128,
    image_id: [u8; 32],
) -> BindingAttestationInput {
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

    let chain_id = [0x99; 32];
    let verifier_id = [0xaa; 32];
    let gate_id = [0xbb; 32];
    let presenter_secret = [0x77; 32];
    let presenter_id = derive_presenter_id(&presenter_secret);
    let context_id = derive_context_id(threshold, &chain_id, &verifier_id, &gate_id, &image_id);
    let expected_context_nullifier =
        derive_context_nullifier(&key_chain.nullifier_public_key, &context_id, &presenter_id);

    BindingAttestationInput {
        npk: key_chain.nullifier_public_key.clone(),
        account: account.clone(),
        membership_proof,
        threshold,
        commitment_root,
        chain_id,
        verifier_id,
        gate_id,
        circuit_image_id: image_id,
        presenter_secret,
        presenter_id,
        expected_context_nullifier,
    }
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

fn digest_to_bytes(digest: &Digest) -> [u8; 32] {
    digest.as_bytes().try_into().unwrap()
}

impl BindingAttestationJournal {
    fn clone_for_report(&self) -> Self {
        Self {
            version: self.version,
            threshold: self.threshold,
            commitment_root: self.commitment_root,
            context_id: self.context_id,
            context_nullifier: self.context_nullifier,
            presenter_id: self.presenter_id,
            verifier_id: self.verifier_id,
            circuit_image_id: self.circuit_image_id,
            proof_index: self.proof_index,
            proof_depth: self.proof_depth,
        }
    }
}

impl From<BindingAttestationJournal> for JournalReport {
    fn from(value: BindingAttestationJournal) -> Self {
        Self {
            version: value.version,
            threshold: value.threshold.to_string(),
            commitment_root_hex: hex::encode(value.commitment_root),
            context_id_hex: hex::encode(value.context_id),
            context_nullifier_hex: hex::encode(value.context_nullifier),
            presenter_id_hex: hex::encode(value.presenter_id),
            verifier_id_hex: hex::encode(value.verifier_id),
            circuit_image_id_hex: hex::encode(value.circuit_image_id),
            proof_index: value.proof_index,
            proof_depth: value.proof_depth,
        }
    }
}
