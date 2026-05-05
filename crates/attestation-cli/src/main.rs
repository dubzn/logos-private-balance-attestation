use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

use attestation_core::{
    derive_context_id, derive_presenter_id, BalanceAttestationEnvelope, ContextBindingParams,
    Digest32, PresenterPubkey,
};
use attestation_prover::{
    balance_attestation_image_id, prove_attestation, AttestationPublicParams,
    BalanceAttestationWitness,
};
use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};
use serde::Deserialize;

const INSPECT_SCRIPT: &str = "scripts/m2-inspect-private-account.sh";
const SPIKE_08_RUNNER_MANIFEST: &str = "spikes/spike-08-program-chaining/lez/runner/Cargo.toml";
const SPIKE_08_RUNNER_BIN: &str =
    "spikes/spike-08-program-chaining/lez/runner/target/release/spike_08_run";
const LEZ_PROGRAM_MANIFEST: &str = "lez-verifier/program/Cargo.toml";
const LEZ_PROGRAM_BIN: &str = "lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin";

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), CliError> {
    let command = parse_args(args)?;
    match command {
        CommandArgs::Help => {
            print_help();
            Ok(())
        }
        CommandArgs::InspectPrivate(options) => run_inspect_private(options),
        CommandArgs::Prove(options) => run_prove(options),
        CommandArgs::Verify(options) => run_verify(options),
        CommandArgs::GateRegisterPresenter(options) => run_gate_register_presenter(options),
        CommandArgs::GateInit(options) => run_gate_init(options),
        CommandArgs::GateAdmit(options) => run_gate_admit(options),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum CommandArgs {
    Help,
    InspectPrivate(InspectPrivateOptions),
    Prove(ProveOptions),
    Verify(VerifyOptions),
    GateRegisterPresenter(GateRegisterPresenterOptions),
    GateInit(GateInitOptions),
    GateAdmit(GateAdmitOptions),
}

// ── prove ─────────────────────────────────────────────────────────────────────

/// Input file format for `balance-attest prove`.
/// chain_id and gate_id are separated out because BalanceAttestationWitness
/// consumes them into context_id during build and does not store them.
#[derive(Deserialize)]
struct ProveInput {
    witness: BalanceAttestationWitness,
    chain_id: Digest32,
    gate_id: Digest32,
    presentation_challenge: Digest32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ProveOptions {
    witness: PathBuf,
    out: Option<PathBuf>,
}

fn parse_prove(args: Vec<String>) -> Result<ProveOptions, CliError> {
    let mut witness = None;
    let mut out = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(CliError::Usage(prove_help())),
            "--witness" => {
                witness = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--witness needs a value".to_owned())
                })?));
            }
            "--out" => {
                out = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--out needs a value".to_owned())
                })?));
            }
            _ => return Err(CliError::Usage(format!("unknown prove argument: {arg}"))),
        }
    }

    let witness =
        witness.ok_or_else(|| CliError::Usage("prove needs --witness <path>".to_owned()))?;

    Ok(ProveOptions { witness, out })
}

fn run_prove(options: ProveOptions) -> Result<(), CliError> {
    let json = fs::read_to_string(&options.witness).map_err(|source| CliError::FileRead {
        path: options.witness.clone(),
        source,
    })?;

    let input: ProveInput =
        serde_json::from_str(&json).map_err(|source| CliError::WitnessParse {
            path: options.witness.clone(),
            source,
        })?;

    let params = AttestationPublicParams {
        threshold: input.witness.threshold,
        chain_id: input.chain_id,
        verifier_id: input.witness.verifier_id,
        gate_id: input.gate_id,
        circuit_image_id: input.witness.circuit_image_id,
    };

    let envelope = prove_attestation(&input.witness, &params, input.presentation_challenge)
        .map_err(|e| CliError::Prove(e.to_string()))?;

    let output = serde_json::to_string_pretty(&envelope).expect("envelope should serialize");

    match &options.out {
        Some(path) => {
            fs::write(path, &output).map_err(|source| CliError::FileRead {
                path: path.clone(),
                source,
            })?;
            eprintln!("envelope written to {}", path.display());
        }
        None => println!("{output}"),
    }

    Ok(())
}

// ── verify ────────────────────────────────────────────────────────────────────

/// Gate file format for `balance-attest verify`.
/// circuit_image_id is omitted on purpose — the verifier always uses the
/// compiled BALANCE_ATTESTATION_ID, so the user can never accidentally
/// verify against the wrong image.
#[derive(Deserialize)]
struct GateFile {
    chain_id: Digest32,
    verifier_id: Digest32,
    gate_id: Digest32,
    presentation_challenge: Digest32,
    #[serde(with = "u128_decimal")]
    threshold: u128,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct VerifyOptions {
    envelope: PathBuf,
    gate: PathBuf,
}

fn parse_verify(args: Vec<String>) -> Result<VerifyOptions, CliError> {
    let mut envelope = None;
    let mut gate = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(CliError::Usage(verify_help())),
            "--envelope" => {
                envelope = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--envelope needs a value".to_owned())
                })?));
            }
            "--gate" => {
                gate = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--gate needs a value".to_owned())
                })?));
            }
            _ => return Err(CliError::Usage(format!("unknown verify argument: {arg}"))),
        }
    }

    let envelope =
        envelope.ok_or_else(|| CliError::Usage("verify needs --envelope <path>".to_owned()))?;
    let gate = gate.ok_or_else(|| CliError::Usage("verify needs --gate <path>".to_owned()))?;

    Ok(VerifyOptions { envelope, gate })
}

fn run_verify(options: VerifyOptions) -> Result<(), CliError> {
    let (envelope, _gate, expected) = load_verified_inputs(&options.envelope, &options.gate)?;

    match verify_envelope(&envelope, &expected) {
        Ok(()) => {
            println!(
                "{{\"status\":\"ok\",\"presenter_id\":\"{}\",\"context_id\":\"{}\",\"context_nullifier\":\"{}\",\"presentation_challenge\":\"{}\",\"threshold\":\"{}\"}}",
                envelope.journal.presenter_id.to_hex(),
                envelope.journal.context_id.to_hex(),
                envelope.journal.context_nullifier.to_hex(),
                envelope.presentation_challenge.to_hex(),
                envelope.journal.threshold,
            );
            Ok(())
        }
        Err(error) => Err(CliError::Verify(error)),
    }
}

// ── gate setup/admit ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Eq, PartialEq)]
struct GateRegisterPresenterOptions {
    presenter_account: String,
    admin_account: String,
    presenter_pubkey_hex: String,
    program_bin: Option<PathBuf>,
    runner_bin: Option<PathBuf>,
    wallet_home: Option<PathBuf>,
    execute: bool,
    skip_build: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct GateInitOptions {
    gate: PathBuf,
    gate_account: String,
    admin_account: String,
    program_bin: Option<PathBuf>,
    runner_bin: Option<PathBuf>,
    wallet_home: Option<PathBuf>,
    execute: bool,
    skip_build: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct GateAdmitOptions {
    envelope: PathBuf,
    gate: PathBuf,
    gate_account: String,
    presenter_account: String,
    program_bin: Option<PathBuf>,
    runner_bin: Option<PathBuf>,
    wallet_home: Option<PathBuf>,
    execute: bool,
    skip_build: bool,
}

fn parse_gate_register_presenter(
    args: Vec<String>,
) -> Result<GateRegisterPresenterOptions, CliError> {
    let mut presenter_account = None;
    let mut admin_account = None;
    let mut presenter_pubkey_hex = None;
    let mut program_bin = None;
    let mut runner_bin = None;
    let mut wallet_home = None;
    let mut execute = false;
    let mut skip_build = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(CliError::Usage(gate_register_presenter_help())),
            "--presenter-account" => {
                presenter_account = Some(args.next().ok_or_else(|| {
                    CliError::Usage("--presenter-account needs a value".to_owned())
                })?);
            }
            "--admin-account" => {
                admin_account =
                    Some(args.next().ok_or_else(|| {
                        CliError::Usage("--admin-account needs a value".to_owned())
                    })?);
            }
            "--presenter-pubkey-hex" => {
                presenter_pubkey_hex = Some(args.next().ok_or_else(|| {
                    CliError::Usage("--presenter-pubkey-hex needs a value".to_owned())
                })?);
            }
            "--program-bin" => {
                program_bin = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--program-bin needs a value".to_owned())
                })?));
            }
            "--runner-bin" => {
                runner_bin = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--runner-bin needs a value".to_owned())
                })?));
            }
            "--wallet-home" => {
                wallet_home = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--wallet-home needs a value".to_owned())
                })?));
            }
            "--execute" => execute = true,
            "--skip-build" => skip_build = true,
            _ => {
                return Err(CliError::Usage(format!(
                    "unknown gate-register-presenter argument: {arg}"
                )))
            }
        }
    }

    let presenter_account = presenter_account.ok_or_else(|| {
        CliError::Usage("gate-register-presenter needs --presenter-account Public/<id>".to_owned())
    })?;
    let admin_account = admin_account.ok_or_else(|| {
        CliError::Usage("gate-register-presenter needs --admin-account Public/<id>".to_owned())
    })?;
    let presenter_pubkey_hex = presenter_pubkey_hex.ok_or_else(|| {
        CliError::Usage("gate-register-presenter needs --presenter-pubkey-hex <hex64>".to_owned())
    })?;

    ensure_public_account("--presenter-account", &presenter_account)?;
    ensure_public_account("--admin-account", &admin_account)?;

    Ok(GateRegisterPresenterOptions {
        presenter_account,
        admin_account,
        presenter_pubkey_hex,
        program_bin,
        runner_bin,
        wallet_home,
        execute,
        skip_build,
    })
}

fn parse_gate_init(args: Vec<String>) -> Result<GateInitOptions, CliError> {
    let mut gate = None;
    let mut gate_account = None;
    let mut admin_account = None;
    let mut program_bin = None;
    let mut runner_bin = None;
    let mut wallet_home = None;
    let mut execute = false;
    let mut skip_build = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(CliError::Usage(gate_init_help())),
            "--gate" => {
                gate = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--gate needs a value".to_owned())
                })?));
            }
            "--gate-account" => {
                gate_account =
                    Some(args.next().ok_or_else(|| {
                        CliError::Usage("--gate-account needs a value".to_owned())
                    })?);
            }
            "--admin-account" => {
                admin_account =
                    Some(args.next().ok_or_else(|| {
                        CliError::Usage("--admin-account needs a value".to_owned())
                    })?);
            }
            "--program-bin" => {
                program_bin = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--program-bin needs a value".to_owned())
                })?));
            }
            "--runner-bin" => {
                runner_bin = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--runner-bin needs a value".to_owned())
                })?));
            }
            "--wallet-home" => {
                wallet_home = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--wallet-home needs a value".to_owned())
                })?));
            }
            "--execute" => execute = true,
            "--skip-build" => skip_build = true,
            _ => {
                return Err(CliError::Usage(format!(
                    "unknown gate-init argument: {arg}"
                )))
            }
        }
    }

    let gate = gate.ok_or_else(|| CliError::Usage("gate-init needs --gate <path>".to_owned()))?;
    let gate_account = gate_account
        .ok_or_else(|| CliError::Usage("gate-init needs --gate-account Public/<id>".to_owned()))?;
    let admin_account = admin_account
        .ok_or_else(|| CliError::Usage("gate-init needs --admin-account Public/<id>".to_owned()))?;

    ensure_public_account("--gate-account", &gate_account)?;
    ensure_public_account("--admin-account", &admin_account)?;

    Ok(GateInitOptions {
        gate,
        gate_account,
        admin_account,
        program_bin,
        runner_bin,
        wallet_home,
        execute,
        skip_build,
    })
}

fn parse_gate_admit(args: Vec<String>) -> Result<GateAdmitOptions, CliError> {
    let mut envelope = None;
    let mut gate = None;
    let mut gate_account = None;
    let mut presenter_account = None;
    let mut program_bin = None;
    let mut runner_bin = None;
    let mut wallet_home = None;
    let mut execute = false;
    let mut skip_build = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(CliError::Usage(gate_admit_help())),
            "--envelope" => {
                envelope = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--envelope needs a value".to_owned())
                })?));
            }
            "--gate" => {
                gate = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--gate needs a value".to_owned())
                })?));
            }
            "--gate-account" => {
                gate_account =
                    Some(args.next().ok_or_else(|| {
                        CliError::Usage("--gate-account needs a value".to_owned())
                    })?);
            }
            "--presenter-account" => {
                presenter_account = Some(args.next().ok_or_else(|| {
                    CliError::Usage("--presenter-account needs a value".to_owned())
                })?);
            }
            "--program-bin" => {
                program_bin = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--program-bin needs a value".to_owned())
                })?));
            }
            "--runner-bin" => {
                runner_bin = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--runner-bin needs a value".to_owned())
                })?));
            }
            "--wallet-home" => {
                wallet_home = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--wallet-home needs a value".to_owned())
                })?));
            }
            "--execute" => execute = true,
            "--skip-build" => skip_build = true,
            _ => {
                return Err(CliError::Usage(format!(
                    "unknown gate-admit argument: {arg}"
                )))
            }
        }
    }

    let envelope =
        envelope.ok_or_else(|| CliError::Usage("gate-admit needs --envelope <path>".to_owned()))?;
    let gate = gate.ok_or_else(|| CliError::Usage("gate-admit needs --gate <path>".to_owned()))?;
    let gate_account = gate_account
        .ok_or_else(|| CliError::Usage("gate-admit needs --gate-account Public/<id>".to_owned()))?;
    let presenter_account = presenter_account.ok_or_else(|| {
        CliError::Usage("gate-admit needs --presenter-account Public/<id>".to_owned())
    })?;

    ensure_public_account("--gate-account", &gate_account)?;
    ensure_public_account("--presenter-account", &presenter_account)?;

    Ok(GateAdmitOptions {
        envelope,
        gate,
        gate_account,
        presenter_account,
        program_bin,
        runner_bin,
        wallet_home,
        execute,
        skip_build,
    })
}

fn run_gate_register_presenter(options: GateRegisterPresenterOptions) -> Result<(), CliError> {
    let presenter_pubkey = parse_presenter_pubkey_hex(&options.presenter_pubkey_hex)?;
    let presenter_pubkey_hex = hex::encode(presenter_pubkey.as_bytes());
    let presenter_id = derive_presenter_id(&presenter_pubkey);

    if !options.execute {
        println!(
            "{{\"status\":\"prepared\",\"command\":\"gate-register-presenter\",\"execute\":false,\"presenter_account\":\"{}\",\"admin_account\":\"{}\",\"presenter_pubkey\":\"{}\",\"presenter_id\":\"{}\"}}",
            options.presenter_account,
            options.admin_account,
            presenter_pubkey_hex,
            presenter_id.to_hex(),
        );
        return Ok(());
    }

    let (repo_root, program_bin, runner_bin) =
        prepare_gate_execution(options.program_bin, options.runner_bin, options.skip_build)?;

    let output = run_gate_runner(GateRunnerCommand {
        repo_root: &repo_root,
        runner_bin: &runner_bin,
        program_bin: &program_bin,
        gate_account: &options.presenter_account,
        admin_account: Some(&options.admin_account),
        presenter_account: &options.presenter_account,
        presenter_pubkey_hex: &presenter_pubkey_hex,
        chain_id_hex: &Digest32::ZERO.to_hex(),
        verifier_id_hex: &Digest32::ZERO.to_hex(),
        gate_id_hex: &Digest32::ZERO.to_hex(),
        threshold: 0,
        inner_image_id_hex: &Digest32::ZERO.to_hex(),
        nullifier_hex: &Digest32::ZERO.to_hex(),
        presenter_id_hex: &presenter_id.to_hex(),
        mode: "register-presenter",
        wallet_home: options.wallet_home.as_deref(),
    })?;

    eprintln!("gate-register-presenter transaction submitted by live runner");
    print_runner_output(&output);
    Ok(())
}

fn run_gate_init(options: GateInitOptions) -> Result<(), CliError> {
    let gate = load_gate_file(&options.gate)?;
    let context_id = expected_context_id(&gate);
    let inner_image_id = Digest32(balance_attestation_image_id());

    if !options.execute {
        println!(
            "{{\"status\":\"prepared\",\"command\":\"gate-init\",\"execute\":false,\"gate_account\":\"{}\",\"admin_account\":\"{}\",\"context_id\":\"{}\",\"inner_image_id\":\"{}\",\"threshold\":\"{}\"}}",
            options.gate_account,
            options.admin_account,
            context_id.to_hex(),
            inner_image_id.to_hex(),
            gate.threshold,
        );
        return Ok(());
    }

    let (repo_root, program_bin, runner_bin) =
        prepare_gate_execution(options.program_bin, options.runner_bin, options.skip_build)?;

    let output = run_gate_runner(GateRunnerCommand {
        repo_root: &repo_root,
        runner_bin: &runner_bin,
        program_bin: &program_bin,
        gate_account: &options.gate_account,
        admin_account: Some(&options.admin_account),
        presenter_account: &options.gate_account,
        presenter_pubkey_hex: &Digest32::ZERO.to_hex(),
        chain_id_hex: &gate.chain_id.to_hex(),
        verifier_id_hex: &gate.verifier_id.to_hex(),
        gate_id_hex: &gate.gate_id.to_hex(),
        threshold: gate.threshold,
        inner_image_id_hex: &inner_image_id.to_hex(),
        nullifier_hex: &Digest32::ZERO.to_hex(),
        presenter_id_hex: &Digest32::ZERO.to_hex(),
        mode: "init-gate",
        wallet_home: options.wallet_home.as_deref(),
    })?;

    eprintln!("gate-init transaction submitted by live runner");
    print_runner_output(&output);
    Ok(())
}

fn run_gate_admit(options: GateAdmitOptions) -> Result<(), CliError> {
    let (envelope, gate, expected) = load_verified_inputs(&options.envelope, &options.gate)?;
    verify_envelope(&envelope, &expected).map_err(CliError::Verify)?;

    if !options.execute {
        println!(
            "{{\"status\":\"precheck_ok\",\"execute\":false,\"presenter_id\":\"{}\",\"context_id\":\"{}\",\"context_nullifier\":\"{}\",\"gate_account\":\"{}\",\"presenter_account\":\"{}\"}}",
            envelope.journal.presenter_id.to_hex(),
            envelope.journal.context_id.to_hex(),
            envelope.journal.context_nullifier.to_hex(),
            options.gate_account,
            options.presenter_account,
        );
        return Ok(());
    }

    let (repo_root, program_bin, runner_bin) =
        prepare_gate_execution(options.program_bin, options.runner_bin, options.skip_build)?;
    let output = run_gate_runner(GateRunnerCommand {
        repo_root: &repo_root,
        runner_bin: &runner_bin,
        program_bin: &program_bin,
        gate_account: &options.gate_account,
        admin_account: None,
        presenter_account: &options.presenter_account,
        presenter_pubkey_hex: &envelope.presenter_pubkey.to_hex(),
        chain_id_hex: &gate.chain_id.to_hex(),
        verifier_id_hex: &gate.verifier_id.to_hex(),
        gate_id_hex: &gate.gate_id.to_hex(),
        threshold: gate.threshold,
        inner_image_id_hex: &Digest32(balance_attestation_image_id()).to_hex(),
        nullifier_hex: &envelope.journal.context_nullifier.to_hex(),
        presenter_id_hex: &envelope.journal.presenter_id.to_hex(),
        mode: "admit",
        wallet_home: options.wallet_home.as_deref(),
    })?;

    eprintln!("gate-admit precheck ok; transaction submitted by live runner");
    print_runner_output(&output);
    Ok(())
}

mod u128_decimal {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse::<u128>().map_err(serde::de::Error::custom)
    }
}

// ── inspect-private ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Eq, PartialEq)]
struct InspectPrivateOptions {
    account: String,
    mode: InspectMode,
    lez_repo: Option<PathBuf>,
    wallet_home: Option<PathBuf>,
    report: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InspectMode {
    LocalOnly,
    RequireProof,
}

fn parse_inspect_private(args: Vec<String>) -> Result<InspectPrivateOptions, CliError> {
    let mut account = None;
    let mut mode = None;
    let mut lez_repo = None;
    let mut wallet_home = None;
    let mut report = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                return Err(CliError::Usage(inspect_private_help()));
            }
            "--account" => {
                account = Some(
                    args.next()
                        .ok_or_else(|| CliError::Usage("--account needs a value".to_owned()))?,
                );
            }
            "--local-only" => set_mode(&mut mode, InspectMode::LocalOnly)?,
            "--require-proof" => set_mode(&mut mode, InspectMode::RequireProof)?,
            "--lez-repo" => {
                lez_repo = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--lez-repo needs a value".to_owned())
                })?));
            }
            "--wallet-home" => {
                wallet_home = Some(PathBuf::from(args.next().ok_or_else(|| {
                    CliError::Usage("--wallet-home needs a value".to_owned())
                })?));
            }
            "--report" => report = true,
            _ => {
                return Err(CliError::Usage(format!(
                    "unknown inspect-private argument: {arg}"
                )))
            }
        }
    }

    let account = account
        .ok_or_else(|| CliError::Usage("inspect-private needs --account <id>".to_owned()))?;

    Ok(InspectPrivateOptions {
        account: normalize_private_account_id(&account),
        mode: mode.unwrap_or(InspectMode::LocalOnly),
        lez_repo,
        wallet_home,
        report,
    })
}

fn set_mode(mode: &mut Option<InspectMode>, next: InspectMode) -> Result<(), CliError> {
    if mode.is_some_and(|current| current != next) {
        return Err(CliError::Usage(
            "--local-only and --require-proof are mutually exclusive".to_owned(),
        ));
    }
    *mode = Some(next);
    Ok(())
}

fn run_inspect_private(options: InspectPrivateOptions) -> Result<(), CliError> {
    let repo_root = repo_root();
    let script = repo_root.join(INSPECT_SCRIPT);
    let result_dir = unique_result_dir();
    fs::create_dir_all(&result_dir)?;

    let mut command = Command::new(&script);
    command
        .current_dir(&repo_root)
        .env("PRIVATE_ACCOUNT", &options.account)
        .env("RESULT_DIR", &result_dir);

    if let Some(lez_repo) = &options.lez_repo {
        command.env("LOGOS_LEZ_REPO", lez_repo);
    }
    if let Some(wallet_home) = &options.wallet_home {
        command.env("NSSA_WALLET_HOME_DIR", wallet_home);
    }

    match options.mode {
        InspectMode::LocalOnly => {
            command.arg("--local-only");
        }
        InspectMode::RequireProof => {
            command.arg("--require-proof");
        }
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(CliError::ScriptFailed {
            command: "inspect-private".to_owned(),
            status: output.status.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    if options.report {
        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(());
    }

    let json_path = find_single_json_file(&result_dir)?;
    let json = fs::read_to_string(&json_path).map_err(|source| CliError::FileRead {
        path: json_path,
        source,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&json).map_err(|error| CliError::Usage(error.to_string()))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value).expect("json should serialize")
    );

    Ok(())
}

// ── arg parsing ───────────────────────────────────────────────────────────────

fn parse_args(args: Vec<String>) -> Result<CommandArgs, CliError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(CommandArgs::Help);
    };

    match command.as_str() {
        "-h" | "--help" | "help" => Ok(CommandArgs::Help),
        "inspect-private" => parse_inspect_private(args.collect()).map(CommandArgs::InspectPrivate),
        "prove" => parse_prove(args.collect()).map(CommandArgs::Prove),
        "verify" => parse_verify(args.collect()).map(CommandArgs::Verify),
        "gate-register-presenter" => {
            parse_gate_register_presenter(args.collect()).map(CommandArgs::GateRegisterPresenter)
        }
        "gate-init" => parse_gate_init(args.collect()).map(CommandArgs::GateInit),
        "gate-admit" => parse_gate_admit(args.collect()).map(CommandArgs::GateAdmit),
        _ => Err(CliError::Usage(format!("unknown command: {command}"))),
    }
}

// ── utilities ─────────────────────────────────────────────────────────────────

fn normalize_private_account_id(account: &str) -> String {
    account
        .strip_prefix("Private/")
        .unwrap_or(account)
        .to_owned()
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under <repo>/crates/attestation-cli")
        .to_path_buf()
}

fn unique_result_dir() -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    env::temp_dir().join(format!("balance-attest-cli-{}-{now}", std::process::id()))
}

fn find_single_json_file(dir: &Path) -> Result<PathBuf, CliError> {
    let mut json_files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            json_files.push(path);
        }
    }

    json_files.sort();
    json_files
        .pop()
        .ok_or_else(|| CliError::JsonMissing(dir.to_path_buf()))
}

fn load_gate_file(gate_path: &Path) -> Result<GateFile, CliError> {
    let gate_json = fs::read_to_string(gate_path).map_err(|source| CliError::FileRead {
        path: gate_path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&gate_json).map_err(|source| CliError::WitnessParse {
        path: gate_path.to_path_buf(),
        source,
    })
}

fn load_verified_inputs(
    envelope_path: &Path,
    gate_path: &Path,
) -> Result<(BalanceAttestationEnvelope, GateFile, ExpectedGate), CliError> {
    let envelope_json = fs::read_to_string(envelope_path).map_err(|source| CliError::FileRead {
        path: envelope_path.to_path_buf(),
        source,
    })?;
    let envelope: BalanceAttestationEnvelope =
        serde_json::from_str(&envelope_json).map_err(|source| CliError::WitnessParse {
            path: envelope_path.to_path_buf(),
            source,
        })?;

    let gate = load_gate_file(gate_path)?;
    let expected = ExpectedGate {
        context_id: expected_context_id(&gate),
        threshold: gate.threshold,
        presentation_challenge: gate.presentation_challenge,
    };

    Ok((envelope, gate, expected))
}

fn expected_context_id(gate: &GateFile) -> Digest32 {
    let ctx_params = ContextBindingParams {
        chain_id: gate.chain_id,
        circuit_image_id: Digest32(balance_attestation_image_id()),
        verifier_id: gate.verifier_id,
        gate_id: gate.gate_id,
        threshold: gate.threshold,
    };
    derive_context_id(&ctx_params)
}

fn ensure_public_account(flag: &str, account: &str) -> Result<(), CliError> {
    if account.starts_with("Public/") {
        Ok(())
    } else {
        Err(CliError::Usage(format!("{flag} must be Public/<id>")))
    }
}

fn parse_presenter_pubkey_hex(value: &str) -> Result<PresenterPubkey, CliError> {
    let bytes = hex::decode(value.trim_start_matches("0x"))
        .map_err(|error| CliError::Usage(format!("invalid --presenter-pubkey-hex: {error}")))?;
    PresenterPubkey::from_slice(&bytes)
        .map_err(|error| CliError::Usage(format!("invalid --presenter-pubkey-hex: {error}")))
}

fn prepare_gate_execution(
    program_bin: Option<PathBuf>,
    runner_bin: Option<PathBuf>,
    skip_build: bool,
) -> Result<(PathBuf, PathBuf, PathBuf), CliError> {
    let repo_root = repo_root();
    let program_bin = program_bin.unwrap_or_else(|| repo_root.join(LEZ_PROGRAM_BIN));
    let runner_bin = runner_bin.unwrap_or_else(|| repo_root.join(SPIKE_08_RUNNER_BIN));

    if !skip_build {
        run_subprocess(
            "build lez-verifier-program",
            Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg(repo_root.join(LEZ_PROGRAM_MANIFEST))
                .current_dir(&repo_root),
        )?;
        run_subprocess(
            "build spike-08 runner",
            Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg(repo_root.join(SPIKE_08_RUNNER_MANIFEST))
                .current_dir(&repo_root),
        )?;
    }

    if !program_bin.is_file() {
        return Err(CliError::Usage(format!(
            "program bin not found at {}; run cargo build --release --manifest-path {}",
            program_bin.display(),
            repo_root.join(LEZ_PROGRAM_MANIFEST).display()
        )));
    }
    if !runner_bin.is_file() {
        return Err(CliError::Usage(format!(
            "runner bin not found at {}; run cargo build --release --manifest-path {}",
            runner_bin.display(),
            repo_root.join(SPIKE_08_RUNNER_MANIFEST).display()
        )));
    }

    Ok((repo_root, program_bin, runner_bin))
}

struct GateRunnerCommand<'a> {
    repo_root: &'a Path,
    runner_bin: &'a Path,
    program_bin: &'a Path,
    gate_account: &'a str,
    admin_account: Option<&'a str>,
    presenter_account: &'a str,
    presenter_pubkey_hex: &'a str,
    chain_id_hex: &'a str,
    verifier_id_hex: &'a str,
    gate_id_hex: &'a str,
    threshold: u128,
    inner_image_id_hex: &'a str,
    nullifier_hex: &'a str,
    presenter_id_hex: &'a str,
    mode: &'a str,
    wallet_home: Option<&'a Path>,
}

fn run_gate_runner(invocation: GateRunnerCommand<'_>) -> Result<std::process::Output, CliError> {
    let mut command = Command::new(invocation.runner_bin);
    command
        .current_dir(invocation.repo_root)
        .arg("--program-bin")
        .arg(invocation.program_bin)
        .arg("--gate-account")
        .arg(invocation.gate_account)
        .arg("--presenter-account")
        .arg(invocation.presenter_account)
        .arg("--presenter-pubkey-hex")
        .arg(invocation.presenter_pubkey_hex)
        .arg("--chain-id-hex")
        .arg(invocation.chain_id_hex)
        .arg("--verifier-id-hex")
        .arg(invocation.verifier_id_hex)
        .arg("--gate-id-hex")
        .arg(invocation.gate_id_hex)
        .arg("--threshold")
        .arg(invocation.threshold.to_string())
        .arg("--inner-image-id-hex")
        .arg(invocation.inner_image_id_hex)
        .arg("--nullifier-hex")
        .arg(invocation.nullifier_hex)
        .arg("--presenter-id-hex")
        .arg(invocation.presenter_id_hex)
        .arg("--mode")
        .arg(invocation.mode);

    if let Some(admin_account) = invocation.admin_account {
        command.arg("--admin-account").arg(admin_account);
    }
    if let Some(wallet_home) = invocation.wallet_home {
        command.env("NSSA_WALLET_HOME_DIR", wallet_home);
    }

    let output = command.output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(CliError::ScriptFailed {
            command: format!("{} runner", invocation.mode),
            status: output.status.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn print_runner_output(output: &std::process::Output) {
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
}

fn run_subprocess(label: &str, command: &mut Command) -> Result<(), CliError> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(CliError::ScriptFailed {
        command: label.to_owned(),
        status: output.status.to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

// ── help text ─────────────────────────────────────────────────────────────────

fn print_help() {
    println!(
        "usage:\n  balance-attest <command> [options]\n\ncommands:\n  inspect-private          Inspect local private wallet state\n  prove                    Prove a balance attestation from a witness JSON file\n  verify                   Verify a balance attestation envelope against a gate file\n  gate-register-presenter  Register a presenter pubkey account in the LEZ gate program\n  gate-init                Initialize a LEZ gate account from a gate file\n  gate-admit               Verify locally, then submit an Admit tx through the live runner\n\nRun `balance-attest <command> --help` for command-specific usage."
    );
}

fn verify_help() -> String {
    "usage: balance-attest verify --envelope <path.json> --gate <path.json>\n\n\
     Verifies a balance attestation envelope (produced by `prove`) against the\n\
     verifier's expected gate parameters. The gate file format is:\n\
     { \"chain_id\": \"hex\", \"verifier_id\": \"hex\", \"gate_id\": \"hex\", \"presentation_challenge\": \"hex\", \"threshold\": \"<u128 decimal>\" }\n\n\
     The circuit_image_id used for verification is always the compiled\n\
     BALANCE_ATTESTATION_ID — callers cannot override it.\n\
     On success, prints a one-line JSON status with the journal's public fields.\n\
     On failure, exits non-zero with a structured error code."
        .to_owned()
}

fn prove_help() -> String {
    "usage: balance-attest prove --witness <path.json> [--out <path.json>]\n\n\
     Reads a witness JSON file (produced by build-witness or manually assembled),\n\
     generates a RISC Zero balance attestation proof, and writes the envelope JSON\n\
     to <out> or stdout. Set RISC0_DEV_MODE=1 for fast (non-production) proving.\n\n\
     The witness file must contain: { witness: {...}, chain_id: \"hex\", gate_id: \"hex\", presentation_challenge: \"hex\" }\n\
     WARNING: the witness file contains private key material — handle it securely."
        .to_owned()
}

fn inspect_private_help() -> String {
    "usage: balance-attest inspect-private --account Private/<id> [--local-only|--require-proof] [--lez-repo <path>] [--wallet-home <path>] [--report]".to_owned()
}

fn gate_register_presenter_help() -> String {
    "usage: balance-attest gate-register-presenter --presenter-account Public/<id> --admin-account Public/<id> --presenter-pubkey-hex <hex64> [--execute] [--wallet-home <path>] [--program-bin <path>] [--runner-bin <path>] [--skip-build]\n\n\
     Without --execute, validates the public inputs and prints a dry-run JSON status.\n\
     With --execute, builds the deployable program and Spike 08 runner if needed, then submits Instruction::RegisterPresenter.\n\
     Use a fresh/default-owned presenter account. The admin account signs this setup transaction and must not be reused for init in the same block."
        .to_owned()
}

fn gate_init_help() -> String {
    "usage: balance-attest gate-init --gate <path.json> --gate-account Public/<id> --admin-account Public/<id> [--execute] [--wallet-home <path>] [--program-bin <path>] [--runner-bin <path>] [--skip-build]\n\n\
     Reads the gate file used by `verify`, derives the exact context_id and compiled inner image id, and prepares Instruction::InitGate.\n\
     Without --execute, prints a dry-run JSON status. With --execute, builds the deployable program and Spike 08 runner if needed, then submits the init transaction.\n\
     Use a fresh/default-owned gate account and a separate fresh admin account from gate-register-presenter."
        .to_owned()
}

fn gate_admit_help() -> String {
    "usage: balance-attest gate-admit --envelope <path.json> --gate <path.json> --gate-account Public/<id> --presenter-account Public/<id> [--execute] [--wallet-home <path>] [--program-bin <path>] [--runner-bin <path>] [--skip-build]\n\n\
     Runs the same verification as `balance-attest verify` before any LEZ transaction is submitted.\n\
     Without --execute, prints a dry-run JSON status. With --execute, builds the deployable program and Spike 08 runner if needed, then submits Instruction::Admit.\n\
     The presenter account must already be registered with the deployed LEZ program so account.data[..32] equals envelope.presenter_pubkey."
        .to_owned()
}

// ── error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(io::Error),
    FileRead {
        path: PathBuf,
        source: io::Error,
    },
    ScriptFailed {
        command: String,
        status: String,
        stdout: String,
        stderr: String,
    },
    JsonMissing(PathBuf),
    WitnessParse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Prove(String),
    Verify(VerifyError),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::FileRead { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            Self::ScriptFailed {
                command,
                status,
                stdout,
                stderr,
            } => {
                writeln!(f, "{command} failed with status {status}")?;
                if !stderr.trim().is_empty() {
                    writeln!(f, "\nstderr:\n{stderr}")?;
                }
                if !stdout.trim().is_empty() {
                    writeln!(f, "\nstdout:\n{stdout}")?;
                }
                Ok(())
            }
            Self::JsonMissing(path) => write!(
                f,
                "inspect-private produced no json under {}",
                path.display()
            ),
            Self::WitnessParse { path, source } => {
                write!(
                    f,
                    "failed to parse witness file {}: {source}",
                    path.display()
                )
            }
            Self::Prove(message) => write!(f, "proving failed: {message}"),
            Self::Verify(error) => write!(f, "verify failed [{}]: {error}", error.code()),
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_private_prefix() {
        assert_eq!(normalize_private_account_id("Private/abc"), "abc");
        assert_eq!(normalize_private_account_id("abc"), "abc");
    }

    #[test]
    fn parses_inspect_private_defaults_to_local_only() {
        let parsed = parse_args(vec![
            "inspect-private".to_owned(),
            "--account".to_owned(),
            "Private/abc".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::InspectPrivate(InspectPrivateOptions {
                account: "abc".to_owned(),
                mode: InspectMode::LocalOnly,
                lez_repo: None,
                wallet_home: None,
                report: false,
            })
        );
    }

    #[test]
    fn rejects_conflicting_modes() {
        let error = parse_args(vec![
            "inspect-private".to_owned(),
            "--account".to_owned(),
            "abc".to_owned(),
            "--local-only".to_owned(),
            "--require-proof".to_owned(),
        ])
        .unwrap_err();

        assert!(error.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn parses_paths_and_report_flag() {
        let parsed = parse_args(vec![
            "inspect-private".to_owned(),
            "--account".to_owned(),
            "abc".to_owned(),
            "--require-proof".to_owned(),
            "--lez-repo".to_owned(),
            "/lez".to_owned(),
            "--wallet-home".to_owned(),
            "/wallet".to_owned(),
            "--report".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::InspectPrivate(InspectPrivateOptions {
                account: "abc".to_owned(),
                mode: InspectMode::RequireProof,
                lez_repo: Some(PathBuf::from("/lez")),
                wallet_home: Some(PathBuf::from("/wallet")),
                report: true,
            })
        );
    }

    #[test]
    fn parses_prove_with_witness_file() {
        let parsed = parse_args(vec![
            "prove".to_owned(),
            "--witness".to_owned(),
            "/tmp/witness.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::Prove(ProveOptions {
                witness: PathBuf::from("/tmp/witness.json"),
                out: None,
            })
        );
    }

    #[test]
    fn parses_prove_with_out_path() {
        let parsed = parse_args(vec![
            "prove".to_owned(),
            "--witness".to_owned(),
            "witness.json".to_owned(),
            "--out".to_owned(),
            "envelope.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::Prove(ProveOptions {
                witness: PathBuf::from("witness.json"),
                out: Some(PathBuf::from("envelope.json")),
            })
        );
    }

    #[test]
    fn rejects_prove_without_witness() {
        let error = parse_args(vec!["prove".to_owned()]).unwrap_err();
        assert!(error.to_string().contains("--witness"));
    }

    #[test]
    fn rejects_unknown_prove_flag() {
        let error = parse_args(vec![
            "prove".to_owned(),
            "--witness".to_owned(),
            "w.json".to_owned(),
            "--bogus".to_owned(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--bogus"));
    }

    #[test]
    fn parses_verify_with_envelope_and_gate() {
        let parsed = parse_args(vec![
            "verify".to_owned(),
            "--envelope".to_owned(),
            "envelope.json".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::Verify(VerifyOptions {
                envelope: PathBuf::from("envelope.json"),
                gate: PathBuf::from("gate.json"),
            })
        );
    }

    #[test]
    fn rejects_verify_without_envelope() {
        let error = parse_args(vec![
            "verify".to_owned(),
            "--gate".to_owned(),
            "g.json".to_owned(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--envelope"));
    }

    #[test]
    fn rejects_verify_without_gate() {
        let error = parse_args(vec![
            "verify".to_owned(),
            "--envelope".to_owned(),
            "e.json".to_owned(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--gate"));
    }

    #[test]
    fn parses_gate_register_presenter_dry_run() {
        let parsed = parse_args(vec![
            "gate-register-presenter".to_owned(),
            "--presenter-account".to_owned(),
            "Public/presenter".to_owned(),
            "--admin-account".to_owned(),
            "Public/admin".to_owned(),
            "--presenter-pubkey-hex".to_owned(),
            valid_presenter_pubkey_hex().to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::GateRegisterPresenter(GateRegisterPresenterOptions {
                presenter_account: "Public/presenter".to_owned(),
                admin_account: "Public/admin".to_owned(),
                presenter_pubkey_hex: valid_presenter_pubkey_hex().to_owned(),
                program_bin: None,
                runner_bin: None,
                wallet_home: None,
                execute: false,
                skip_build: false,
            })
        );
    }

    #[test]
    fn parses_gate_register_presenter_execute_options() {
        let parsed = parse_args(vec![
            "gate-register-presenter".to_owned(),
            "--presenter-account".to_owned(),
            "Public/presenter".to_owned(),
            "--admin-account".to_owned(),
            "Public/admin".to_owned(),
            "--presenter-pubkey-hex".to_owned(),
            valid_presenter_pubkey_hex().to_owned(),
            "--program-bin".to_owned(),
            "program.bin".to_owned(),
            "--runner-bin".to_owned(),
            "runner".to_owned(),
            "--wallet-home".to_owned(),
            "/wallet".to_owned(),
            "--execute".to_owned(),
            "--skip-build".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::GateRegisterPresenter(GateRegisterPresenterOptions {
                presenter_account: "Public/presenter".to_owned(),
                admin_account: "Public/admin".to_owned(),
                presenter_pubkey_hex: valid_presenter_pubkey_hex().to_owned(),
                program_bin: Some(PathBuf::from("program.bin")),
                runner_bin: Some(PathBuf::from("runner")),
                wallet_home: Some(PathBuf::from("/wallet")),
                execute: true,
                skip_build: true,
            })
        );
    }

    #[test]
    fn rejects_gate_register_presenter_non_public_accounts() {
        let error = parse_args(vec![
            "gate-register-presenter".to_owned(),
            "--presenter-account".to_owned(),
            "presenter".to_owned(),
            "--admin-account".to_owned(),
            "Public/admin".to_owned(),
            "--presenter-pubkey-hex".to_owned(),
            valid_presenter_pubkey_hex().to_owned(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--presenter-account"));
    }

    #[test]
    fn parses_gate_init_dry_run() {
        let parsed = parse_args(vec![
            "gate-init".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
            "--gate-account".to_owned(),
            "Public/gate".to_owned(),
            "--admin-account".to_owned(),
            "Public/admin".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::GateInit(GateInitOptions {
                gate: PathBuf::from("gate.json"),
                gate_account: "Public/gate".to_owned(),
                admin_account: "Public/admin".to_owned(),
                program_bin: None,
                runner_bin: None,
                wallet_home: None,
                execute: false,
                skip_build: false,
            })
        );
    }

    #[test]
    fn parses_gate_init_execute_options() {
        let parsed = parse_args(vec![
            "gate-init".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
            "--gate-account".to_owned(),
            "Public/gate".to_owned(),
            "--admin-account".to_owned(),
            "Public/admin".to_owned(),
            "--program-bin".to_owned(),
            "program.bin".to_owned(),
            "--runner-bin".to_owned(),
            "runner".to_owned(),
            "--wallet-home".to_owned(),
            "/wallet".to_owned(),
            "--execute".to_owned(),
            "--skip-build".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::GateInit(GateInitOptions {
                gate: PathBuf::from("gate.json"),
                gate_account: "Public/gate".to_owned(),
                admin_account: "Public/admin".to_owned(),
                program_bin: Some(PathBuf::from("program.bin")),
                runner_bin: Some(PathBuf::from("runner")),
                wallet_home: Some(PathBuf::from("/wallet")),
                execute: true,
                skip_build: true,
            })
        );
    }

    #[test]
    fn rejects_gate_init_without_admin() {
        let error = parse_args(vec![
            "gate-init".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
            "--gate-account".to_owned(),
            "Public/gate".to_owned(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--admin-account"));
    }

    #[test]
    fn validates_presenter_pubkey_hex() {
        assert!(parse_presenter_pubkey_hex(valid_presenter_pubkey_hex()).is_ok());
        assert!(parse_presenter_pubkey_hex("not-hex").is_err());
    }

    #[test]
    fn parses_gate_admit_dry_run() {
        let parsed = parse_args(vec![
            "gate-admit".to_owned(),
            "--envelope".to_owned(),
            "envelope.json".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
            "--gate-account".to_owned(),
            "Public/gate".to_owned(),
            "--presenter-account".to_owned(),
            "Public/presenter".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::GateAdmit(GateAdmitOptions {
                envelope: PathBuf::from("envelope.json"),
                gate: PathBuf::from("gate.json"),
                gate_account: "Public/gate".to_owned(),
                presenter_account: "Public/presenter".to_owned(),
                program_bin: None,
                runner_bin: None,
                wallet_home: None,
                execute: false,
                skip_build: false,
            })
        );
    }

    #[test]
    fn parses_gate_admit_execute_options() {
        let parsed = parse_args(vec![
            "gate-admit".to_owned(),
            "--envelope".to_owned(),
            "envelope.json".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
            "--gate-account".to_owned(),
            "Public/gate".to_owned(),
            "--presenter-account".to_owned(),
            "Public/presenter".to_owned(),
            "--program-bin".to_owned(),
            "program.bin".to_owned(),
            "--runner-bin".to_owned(),
            "runner".to_owned(),
            "--wallet-home".to_owned(),
            "/wallet".to_owned(),
            "--execute".to_owned(),
            "--skip-build".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            parsed,
            CommandArgs::GateAdmit(GateAdmitOptions {
                envelope: PathBuf::from("envelope.json"),
                gate: PathBuf::from("gate.json"),
                gate_account: "Public/gate".to_owned(),
                presenter_account: "Public/presenter".to_owned(),
                program_bin: Some(PathBuf::from("program.bin")),
                runner_bin: Some(PathBuf::from("runner")),
                wallet_home: Some(PathBuf::from("/wallet")),
                execute: true,
                skip_build: true,
            })
        );
    }

    #[test]
    fn rejects_gate_admit_non_public_accounts() {
        let error = parse_args(vec![
            "gate-admit".to_owned(),
            "--envelope".to_owned(),
            "envelope.json".to_owned(),
            "--gate".to_owned(),
            "gate.json".to_owned(),
            "--gate-account".to_owned(),
            "gate".to_owned(),
            "--presenter-account".to_owned(),
            "Public/presenter".to_owned(),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--gate-account"));
    }

    fn valid_presenter_pubkey_hex() -> &'static str {
        "9ac20335eb38768d2052be1dbbc3c8f6178407458e51e6b4ad22f1d91758895b"
    }
}
