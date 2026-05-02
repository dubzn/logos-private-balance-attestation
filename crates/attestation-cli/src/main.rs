use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

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
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum CommandArgs {
    Help,
    InspectPrivate(InspectPrivateOptions),
}

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

#[derive(Debug)]
enum CliError {
    Usage(String),
    Io(io::Error),
    ScriptFailed {
        status: String,
        stdout: String,
        stderr: String,
    },
    JsonMissing(PathBuf),
    JsonRead {
        path: PathBuf,
        source: io::Error,
    },
}

fn parse_args(args: Vec<String>) -> Result<CommandArgs, CliError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(CommandArgs::Help);
    };

    match command.as_str() {
        "-h" | "--help" | "help" => Ok(CommandArgs::Help),
        "inspect-private" => parse_inspect_private(args.collect()).map(CommandArgs::InspectPrivate),
        _ => Err(CliError::Usage(format!("unknown command: {command}"))),
    }
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
    let json = fs::read_to_string(&json_path).map_err(|source| CliError::JsonRead {
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

fn print_help() {
    println!(
        "usage:\n  balance-attest inspect-private --account Private/<id> [--local-only|--require-proof] [--lez-repo <path>] [--wallet-home <path>] [--report]\n\ncommands:\n  inspect-private   Inspect local private wallet state without printing witness data"
    );
}

fn inspect_private_help() -> String {
    "usage: balance-attest inspect-private --account Private/<id> [--local-only|--require-proof] [--lez-repo <path>] [--wallet-home <path>] [--report]".to_owned()
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::ScriptFailed {
                status,
                stdout,
                stderr,
            } => {
                writeln!(formatter, "inspect-private failed with status {status}")?;
                if !stderr.trim().is_empty() {
                    writeln!(formatter, "\nstderr:\n{stderr}")?;
                }
                if !stdout.trim().is_empty() {
                    writeln!(formatter, "\nstdout:\n{stdout}")?;
                }
                Ok(())
            }
            Self::JsonMissing(path) => {
                write!(
                    formatter,
                    "inspect-private produced no json under {}",
                    path.display()
                )
            }
            Self::JsonRead { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

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
}
