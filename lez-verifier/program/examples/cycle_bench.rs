//! RISC Zero cycle benchmark for the deployable LP-0005 LEZ gate program.
//!
//! This follows the metric used by upstream LEZ `tools/cycle_bench`:
//! successful guest executions report deterministic `SessionInfo::cycles()`.
//! Rejected executions return an executor error before `SessionInfo` exists, so
//! they report wall time and the deterministic BA error code, but no cycle
//! count.

use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, ensure, Context, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{
    account::{Account, AccountId, AccountWithMetadata},
    program::{ProgramId, ProgramOutput},
};
use lez_verifier_program::{BALANCE_ATTESTATION_PROGRAM_ELF, BALANCE_ATTESTATION_PROGRAM_ID};
use risc0_zkvm::{default_executor, serde::to_vec, ExecutorEnv};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const PRESENTER_DOMAIN: &[u8] = b"logos-balance-attestation/v1/presenter";
const GATE_STATE_MAGIC: [u8; 4] = *b"BAT1";

#[derive(Debug)]
struct Args {
    iterations: usize,
    json_out: PathBuf,
    markdown_out: PathBuf,
    lez_ref: String,
    generated_at: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct GateState {
    magic: [u8; 4],
    version: u16,
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    expected_inner_image_id: [u8; 32],
    admitted_nullifiers: Vec<[u8; 32]>,
}

#[derive(BorshSerialize)]
struct OuterJournal {
    version: u16,
    inner_image_id: [u8; 32],
    gate_context_id: [u8; 32],
    accepted_context_nullifier: [u8; 32],
    accepted_presenter_id: [u8; 32],
    accepted_threshold: u128,
}

#[derive(Serialize, Deserialize)]
enum Instruction {
    RegisterPresenter {
        presenter_pubkey: [u8; 32],
    },
    InitGate {
        chain_id: [u8; 32],
        verifier_id: [u8; 32],
        gate_id: [u8; 32],
        threshold: u128,
        expected_inner_image_id: [u8; 32],
    },
    Admit {
        outer_journal: Vec<u8>,
    },
}

#[derive(Debug, Serialize)]
struct TimingStats {
    best_ms: f64,
    mean_ms: f64,
    stdev_ms: f64,
    samples: usize,
}

#[derive(Debug, Serialize)]
struct SuccessResult {
    operation: &'static str,
    user_cycles: u64,
    segments: usize,
    timing: TimingStats,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct RejectionResult {
    operation: &'static str,
    expected_error_code: &'static str,
    user_cycles: Option<u64>,
    timing: TimingStats,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    schema_version: u16,
    generated_at: String,
    lez_ref: String,
    risc0_zkvm: &'static str,
    metric: &'static str,
    iterations: usize,
    cu_available_from_executor: bool,
    successful_operations: Vec<SuccessResult>,
    rejected_operations: Vec<RejectionResult>,
}

struct Fixture {
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    threshold: u128,
    expected_inner_image_id: [u8; 32],
    presenter_pubkey: [u8; 32],
    nullifier: [u8; 32],
    gate_account_id: AccountId,
    presenter_account_id: AccountId,
}

impl Fixture {
    fn new() -> Self {
        Self {
            chain_id: [0x11; 32],
            verifier_id: [0x22; 32],
            gate_id: [0x33; 32],
            threshold: 25,
            expected_inner_image_id: [0x44; 32],
            presenter_pubkey: [0x55; 32],
            nullifier: [0xAA; 32],
            gate_account_id: AccountId::new([0x66; 32]),
            presenter_account_id: AccountId::new([0x77; 32]),
        }
    }

    fn context_id(&self) -> [u8; 32] {
        hash_segments(&[
            CONTEXT_DOMAIN,
            &self.chain_id,
            &self.expected_inner_image_id,
            &self.verifier_id,
            &self.gate_id,
            &self.threshold.to_le_bytes(),
        ])
    }

    fn presenter_id(&self) -> [u8; 32] {
        hash_segments(&[PRESENTER_DOMAIN, &self.presenter_pubkey])
    }

    fn outer_journal_bytes(&self) -> Vec<u8> {
        borsh::to_vec(&OuterJournal {
            version: 1,
            inner_image_id: self.expected_inner_image_id,
            gate_context_id: self.context_id(),
            accepted_context_nullifier: self.nullifier,
            accepted_presenter_id: self.presenter_id(),
            accepted_threshold: self.threshold,
        })
        .expect("encode outer journal")
    }
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let fixture = Fixture::new();

    let register_instruction = Instruction::RegisterPresenter {
        presenter_pubkey: fixture.presenter_pubkey,
    };
    let register_pre = vec![
        default_pre_state(fixture.presenter_account_id),
        authorized_pre_state(fixture.gate_account_id, &[0x01; 32]),
    ];

    let init_instruction = Instruction::InitGate {
        chain_id: fixture.chain_id,
        verifier_id: fixture.verifier_id,
        gate_id: fixture.gate_id,
        threshold: fixture.threshold,
        expected_inner_image_id: fixture.expected_inner_image_id,
    };
    let init_pre = vec![
        default_pre_state(fixture.gate_account_id),
        authorized_pre_state(fixture.presenter_account_id, &fixture.presenter_pubkey),
    ];

    let admit_instruction = Instruction::Admit {
        outer_journal: fixture.outer_journal_bytes(),
    };
    let admit_pre_empty = admit_pre_states(&fixture, Vec::new());
    let admitted_before = (0_u8..10).map(|value| [value; 32]).collect();
    let admit_pre_ten = admit_pre_states(&fixture, admitted_before);
    let duplicate_pre = admit_pre_states(&fixture, vec![fixture.nullifier]);

    let successful_operations = vec![
        run_success(
            "register_presenter",
            &register_pre,
            &register_instruction,
            args.iterations,
            "Claims a presenter account and stores its 32-byte public key.",
        )?,
        run_success(
            "init_gate",
            &init_pre,
            &init_instruction,
            args.iterations,
            "Creates BAT1 gate state with an empty nullifier list.",
        )?,
        run_success(
            "admit_empty_gate",
            &admit_pre_empty,
            &admit_instruction,
            args.iterations,
            "Validates gate fields and appends the first context nullifier.",
        )?,
        run_success(
            "admit_after_10",
            &admit_pre_ten,
            &admit_instruction,
            args.iterations,
            "Measures admit after scanning ten existing nullifiers.",
        )?,
    ];

    let rejected_operations = vec![run_rejection(
        "duplicate_admit",
        "BA206",
        &duplicate_pre,
        &admit_instruction,
        args.iterations,
        "RISC Zero returns an execution error before SessionInfo is available; cycles are not reported.",
    )?];

    let report = BenchmarkReport {
        schema_version: 1,
        generated_at: args.generated_at,
        lez_ref: args.lez_ref,
        risc0_zkvm: "3.0.5",
        metric: "risc0 SessionInfo::cycles() user cycles",
        iterations: args.iterations,
        cu_available_from_executor: false,
        successful_operations,
        rejected_operations,
    };

    write_json(&args.json_out, &report)?;
    write_markdown(&args.markdown_out, &report)?;

    println!("Cycle benchmark complete.");
    println!("JSON: {}", args.json_out.display());
    println!("Markdown: {}", args.markdown_out.display());
    print_summary(&report);
    Ok(())
}

fn run_success(
    operation: &'static str,
    pre_states: &[AccountWithMetadata],
    instruction: &Instruction,
    iterations: usize,
    note: &'static str,
) -> Result<SuccessResult> {
    let mut samples = Vec::with_capacity(iterations);
    let mut expected_cycles = None;
    let mut segments = 0;

    for iteration in 0..=iterations {
        let env = build_env(pre_states, instruction)?;
        let started = Instant::now();
        let session = default_executor()
            .execute(env, BALANCE_ATTESTATION_PROGRAM_ELF)
            .with_context(|| format!("{operation} guest execution failed"))?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let _: ProgramOutput = session
            .journal
            .decode()
            .with_context(|| format!("{operation} output decode failed"))?;

        if let Some(cycles) = expected_cycles {
            ensure!(
                cycles == session.cycles(),
                "{operation} cycle count changed across iterations"
            );
        } else {
            expected_cycles = Some(session.cycles());
        }
        segments = session.segments.len();
        if iteration > 0 {
            samples.push(elapsed_ms);
        }
    }

    Ok(SuccessResult {
        operation,
        user_cycles: expected_cycles.context("missing successful cycle sample")?,
        segments,
        timing: timing_stats(&samples),
        note,
    })
}

fn run_rejection(
    operation: &'static str,
    expected_error_code: &'static str,
    pre_states: &[AccountWithMetadata],
    instruction: &Instruction,
    iterations: usize,
    note: &'static str,
) -> Result<RejectionResult> {
    let mut samples = Vec::with_capacity(iterations);

    for iteration in 0..=iterations {
        let env = build_env(pre_states, instruction)?;
        let started = Instant::now();
        let error = default_executor()
            .execute(env, BALANCE_ATTESTATION_PROGRAM_ELF)
            .expect_err("rejection benchmark unexpectedly succeeded")
            .to_string();
        let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
        ensure!(
            error.contains(expected_error_code),
            "{operation} returned unexpected error: {error}"
        );
        if iteration > 0 {
            samples.push(elapsed_ms);
        }
    }

    Ok(RejectionResult {
        operation,
        expected_error_code,
        user_cycles: None,
        timing: timing_stats(&samples),
        note,
    })
}

fn build_env<'a>(
    pre_states: &[AccountWithMetadata],
    instruction: &Instruction,
) -> Result<ExecutorEnv<'a>> {
    let mut builder = ExecutorEnv::builder();
    let caller_program_id: Option<ProgramId> = None;
    let instruction_words: Vec<u32> = to_vec(instruction)?;
    builder
        .write(&BALANCE_ATTESTATION_PROGRAM_ID)?
        .write(&caller_program_id)?
        .write(&pre_states.to_vec())?
        .write(&instruction_words)?;
    builder.build()
}

fn timing_stats(samples: &[f64]) -> TimingStats {
    let samples_len = samples.len();
    let mean = samples.iter().sum::<f64>() / samples_len as f64;
    let variance = samples
        .iter()
        .map(|sample| {
            let delta = sample - mean;
            delta * delta
        })
        .sum::<f64>()
        / samples_len as f64;
    TimingStats {
        best_ms: samples.iter().copied().fold(f64::INFINITY, f64::min),
        mean_ms: mean,
        stdev_ms: variance.sqrt(),
        samples: samples_len,
    }
}

fn parse_args() -> Result<Args> {
    let mut iterations = 10_usize;
    let mut json_out = PathBuf::from("target/lp0005-cycle-bench.json");
    let mut markdown_out = PathBuf::from("target/lp0005-cycle-bench.md");
    let mut lez_ref = String::from("unknown");
    let mut generated_at = String::from("unknown");
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iterations" => {
                iterations = args
                    .next()
                    .context("--iterations needs a value")?
                    .parse()
                    .context("--iterations must be a positive integer")?;
            }
            "--json-out" => {
                json_out = PathBuf::from(args.next().context("--json-out needs a path")?);
            }
            "--markdown-out" => {
                markdown_out = PathBuf::from(args.next().context("--markdown-out needs a path")?);
            }
            "--lez-ref" => {
                lez_ref = args.next().context("--lez-ref needs a value")?;
            }
            "--generated-at" => {
                generated_at = args.next().context("--generated-at needs a value")?;
            }
            "-h" | "--help" => {
                println!(
                    "usage: cycle_bench [--iterations N] [--json-out PATH] [--markdown-out PATH] [--lez-ref SHA] [--generated-at ISO8601]"
                );
                std::process::exit(0);
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }

    ensure!(iterations > 0, "--iterations must be greater than zero");
    Ok(Args {
        iterations,
        json_out,
        markdown_out,
        lez_ref,
        generated_at,
    })
}

fn write_json(path: &Path, report: &BenchmarkReport) -> Result<()> {
    create_parent(path)?;
    fs::write(path, serde_json::to_vec_pretty(report)?)
        .with_context(|| format!("write {}", path.display()))
}

fn write_markdown(path: &Path, report: &BenchmarkReport) -> Result<()> {
    create_parent(path)?;
    let mut markdown = format!(
        "# LP-0005 LEZ Cycle Benchmark\n\n- Generated: `{}`\n- LEZ ref: `{}`\n- RISC Zero: `{}`\n- Samples: `{}` plus one discarded warmup\n- Metric: `{}`\n\n## Successful Operations\n\n| Operation | User cycles | Segments | Best ms | Mean +/- stdev ms |\n| --- | ---: | ---: | ---: | ---: |\n",
        report.generated_at,
        report.lez_ref,
        report.risc0_zkvm,
        report.iterations,
        report.metric,
    );
    for result in &report.successful_operations {
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {:.3} | {:.3} +/- {:.3} |\n",
            result.operation,
            result.user_cycles,
            result.segments,
            result.timing.best_ms,
            result.timing.mean_ms,
            result.timing.stdev_ms,
        ));
    }
    markdown.push_str(
        "\n## Rejected Operations\n\n| Operation | Error | User cycles | Best ms | Mean +/- stdev ms |\n| --- | --- | ---: | ---: | ---: |\n",
    );
    for result in &report.rejected_operations {
        markdown.push_str(&format!(
            "| `{}` | `{}` | unavailable | {:.3} | {:.3} +/- {:.3} |\n",
            result.operation,
            result.expected_error_code,
            result.timing.best_ms,
            result.timing.mean_ms,
            result.timing.stdev_ms,
        ));
    }
    markdown.push_str(
        "\n## Interpretation\n\nThese are RISC Zero guest user cycles, matching upstream LEZ `tools/cycle_bench`. They are not labeled as chain CU because the current executor/RPC does not expose a CU field. Rejected execution returns an error before `SessionInfo` is available, so only wall time and the deterministic BA error code are recorded for that path.\n",
    );
    fs::write(path, markdown).with_context(|| format!("write {}", path.display()))
}

fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    Ok(())
}

fn print_summary(report: &BenchmarkReport) {
    println!("operation,user_cycles,segments,best_ms,mean_ms,stdev_ms");
    for result in &report.successful_operations {
        println!(
            "{},{},{},{:.3},{:.3},{:.3}",
            result.operation,
            result.user_cycles,
            result.segments,
            result.timing.best_ms,
            result.timing.mean_ms,
            result.timing.stdev_ms,
        );
    }
    for result in &report.rejected_operations {
        println!(
            "{},unavailable,unavailable,{:.3},{:.3},{:.3}",
            result.operation, result.timing.best_ms, result.timing.mean_ms, result.timing.stdev_ms,
        );
    }
}

fn default_pre_state(account_id: AccountId) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account::default(),
        is_authorized: true,
        account_id,
    }
}

fn authorized_pre_state(account_id: AccountId, pubkey: &[u8; 32]) -> AccountWithMetadata {
    let data = pubkey.to_vec().try_into().expect("pubkey fits in data");
    AccountWithMetadata {
        account: Account {
            data,
            ..Account::default()
        },
        is_authorized: true,
        account_id,
    }
}

fn registered_presenter_pre_state(account_id: AccountId, pubkey: &[u8; 32]) -> AccountWithMetadata {
    let mut state = authorized_pre_state(account_id, pubkey);
    state.account.program_owner = BALANCE_ATTESTATION_PROGRAM_ID;
    state
}

fn admit_pre_states(
    fixture: &Fixture,
    admitted_nullifiers: Vec<[u8; 32]>,
) -> Vec<AccountWithMetadata> {
    vec![
        AccountWithMetadata {
            account: initialized_gate_account(fixture, admitted_nullifiers),
            is_authorized: false,
            account_id: fixture.gate_account_id,
        },
        registered_presenter_pre_state(fixture.presenter_account_id, &fixture.presenter_pubkey),
    ]
}

fn initialized_gate_account(fixture: &Fixture, admitted_nullifiers: Vec<[u8; 32]>) -> Account {
    let bytes = borsh::to_vec(&GateState {
        magic: GATE_STATE_MAGIC,
        version: 1,
        chain_id: fixture.chain_id,
        verifier_id: fixture.verifier_id,
        gate_id: fixture.gate_id,
        threshold: fixture.threshold,
        expected_inner_image_id: fixture.expected_inner_image_id,
        admitted_nullifiers,
    })
    .expect("encode gate state");
    let data = bytes.try_into().expect("gate state fits in data");
    Account {
        program_owner: BALANCE_ATTESTATION_PROGRAM_ID,
        data,
        ..Account::default()
    }
}

fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for segment in segments {
        hasher.update((segment.len() as u64).to_le_bytes());
        hasher.update(segment);
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}
