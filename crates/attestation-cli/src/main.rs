use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

use attestation_core::{
    derive_context_id, BalanceAttestationEnvelope, ContextBindingParams, Digest32,
};
use attestation_prover::{
    balance_attestation_image_id, prove_attestation, AttestationPublicParams,
    BalanceAttestationWitness,
};
use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};
use serde::Deserialize;

const INSPECT_SCRIPT: &str = "scripts/m2-inspect-private-account.sh";

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
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum CommandArgs {
    Help,
    InspectPrivate(InspectPrivateOptions),
    Prove(ProveOptions),
    Verify(VerifyOptions),
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
            _ => {
                return Err(CliError::Usage(format!(
                    "unknown prove argument: {arg}"
                )))
            }
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

    let envelope =
        prove_attestation(&input.witness, &params).map_err(|e| CliError::Prove(e.to_string()))?;

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
            _ => {
                return Err(CliError::Usage(format!(
                    "unknown verify argument: {arg}"
                )))
            }
        }
    }

    let envelope = envelope
        .ok_or_else(|| CliError::Usage("verify needs --envelope <path>".to_owned()))?;
    let gate = gate.ok_or_else(|| CliError::Usage("verify needs --gate <path>".to_owned()))?;

    Ok(VerifyOptions { envelope, gate })
}

fn run_verify(options: VerifyOptions) -> Result<(), CliError> {
    let envelope_json = fs::read_to_string(&options.envelope).map_err(|source| {
        CliError::FileRead {
            path: options.envelope.clone(),
            source,
        }
    })?;
    let envelope: BalanceAttestationEnvelope =
        serde_json::from_str(&envelope_json).map_err(|source| CliError::WitnessParse {
            path: options.envelope.clone(),
            source,
        })?;

    let gate_json = fs::read_to_string(&options.gate).map_err(|source| CliError::FileRead {
        path: options.gate.clone(),
        source,
    })?;
    let gate: GateFile =
        serde_json::from_str(&gate_json).map_err(|source| CliError::WitnessParse {
            path: options.gate.clone(),
            source,
        })?;

    let ctx_params = ContextBindingParams {
        chain_id: gate.chain_id,
        circuit_image_id: Digest32(balance_attestation_image_id()),
        verifier_id: gate.verifier_id,
        gate_id: gate.gate_id,
        threshold: gate.threshold,
    };
    let expected = ExpectedGate {
        context_id: derive_context_id(&ctx_params),
        min_threshold: gate.threshold,
    };

    match verify_envelope(&envelope, &expected) {
        Ok(()) => {
            println!(
                "{{\"status\":\"ok\",\"presenter_id\":\"{}\",\"context_id\":\"{}\",\"context_nullifier\":\"{}\",\"threshold\":\"{}\"}}",
                envelope.journal.presenter_id.to_hex(),
                envelope.journal.context_id.to_hex(),
                envelope.journal.context_nullifier.to_hex(),
                envelope.journal.threshold,
            );
            Ok(())
        }
        Err(error) => Err(CliError::Verify(error)),
    }
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
        "inspect-private" => {
            parse_inspect_private(args.collect()).map(CommandArgs::InspectPrivate)
        }
        "prove" => parse_prove(args.collect()).map(CommandArgs::Prove),
        "verify" => parse_verify(args.collect()).map(CommandArgs::Verify),
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

// ── help text ─────────────────────────────────────────────────────────────────

fn print_help() {
    println!(
        "usage:\n  balance-attest <command> [options]\n\ncommands:\n  inspect-private   Inspect local private wallet state\n  prove             Prove a balance attestation from a witness JSON file\n  verify            Verify a balance attestation envelope against a gate file\n\nRun `balance-attest <command> --help` for command-specific usage."
    );
}

fn verify_help() -> String {
    "usage: balance-attest verify --envelope <path.json> --gate <path.json>\n\n\
     Verifies a balance attestation envelope (produced by `prove`) against the\n\
     verifier's expected gate parameters. The gate file format is:\n\
     { \"chain_id\": \"hex\", \"verifier_id\": \"hex\", \"gate_id\": \"hex\", \"threshold\": \"<u128 decimal>\" }\n\n\
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
     The witness file must contain: { witness: {...}, chain_id: \"hex\", gate_id: \"hex\" }\n\
     WARNING: the witness file contains private key material — handle it securely."
        .to_owned()
}

fn inspect_private_help() -> String {
    "usage: balance-attest inspect-private --account Private/<id> [--local-only|--require-proof] [--lez-repo <path>] [--wallet-home <path>] [--report]".to_owned()
}

// ── error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(io::Error),
    FileRead { path: PathBuf, source: io::Error },
    ScriptFailed { status: String, stdout: String, stderr: String },
    JsonMissing(PathBuf),
    WitnessParse { path: PathBuf, source: serde_json::Error },
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
            Self::ScriptFailed { status, stdout, stderr } => {
                writeln!(f, "inspect-private failed with status {status}")?;
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
                write!(f, "failed to parse witness file {}: {source}", path.display())
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
}
