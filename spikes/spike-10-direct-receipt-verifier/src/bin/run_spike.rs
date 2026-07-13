use std::{
    env, fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{bail, ensure, Context, Result};
use risc0_zkvm::{default_executor, default_prover, ExecutorEnv, Prover, ProverOpts, Receipt};
use serde_json::json;
use spike_10_direct_receipt_verifier::{
    DIRECT_RECEIPT_VERIFIER_ELF, DIRECT_RECEIPT_VERIFIER_ID, TOY_STATEMENT_ELF, TOY_STATEMENT_ID,
};

fn main() -> Result<()> {
    let mode = env::args()
        .nth(1)
        .unwrap_or_else(|| "compile-only".to_owned());
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| "target/spike-10-output".to_owned()));
    fs::create_dir_all(&out_dir)?;

    match mode.as_str() {
        "compile-only" => write_compile_report(out_dir),
        "real-succinct" => run_real(out_dir, "succinct", ProverOpts::succinct()),
        "real-groth16" => run_real(out_dir, "groth16", ProverOpts::groth16()),
        _ => bail!("usage: run_spike [compile-only|real-succinct|real-groth16]"),
    }
}

fn write_compile_report(out_dir: PathBuf) -> Result<()> {
    let report = json!({
        "status": "compiled",
        "direct_receipt_verifier_image_id": image_id_hex(DIRECT_RECEIPT_VERIFIER_ID),
        "toy_statement_image_id": image_id_hex(TOY_STATEMENT_ID),
        "direct_receipt_verifier_elf_bytes": DIRECT_RECEIPT_VERIFIER_ELF.len(),
        "toy_statement_elf_bytes": TOY_STATEMENT_ELF.len(),
        "scope": "compile-only; no cryptographic receipt was verified"
    });
    fs::write(
        out_dir.join("compile-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn run_real(out_dir: PathBuf, receipt_kind: &'static str, prover_opts: ProverOpts) -> Result<()> {
    ensure!(
        !dev_mode_enabled(),
        "real receipt modes require RISC0_DEV_MODE=0; fake receipts are not cryptographic evidence"
    );

    let inner_started = Instant::now();
    let inner_env = ExecutorEnv::builder().write(&41_u64)?.build()?;
    let receipt = default_prover()
        .prove_with_opts(inner_env, TOY_STATEMENT_ELF, &prover_opts)
        .with_context(|| format!("produce real {receipt_kind} toy receipt"))?
        .receipt;
    let inner_duration = inner_started.elapsed();
    receipt
        .verify(TOY_STATEMENT_ID)
        .context("host verification of toy receipt")?;
    let result: u64 = receipt.journal.decode()?;
    ensure!(result == 42, "toy receipt journal should contain 42");

    let receipt_bytes = borsh::to_vec(&receipt)?;
    let valid = execute_direct_verifier(&receipt, &receipt_bytes)
        .context("execute direct verifier over valid receipt")?;
    let lez_limit = execute_with_lez_public_limit(&receipt, &receipt_bytes)?;

    let mut tampered_receipt = receipt.clone();
    tampered_receipt.journal.bytes[0] ^= 0x01;
    let tampered_bytes = borsh::to_vec(&tampered_receipt)?;
    let tampered = execute_rejected_verifier(&tampered_receipt, &tampered_bytes)?;

    let report = json!({
        "status": "ok",
        "risc0_dev_mode": 0,
        "receipt_kind": receipt_kind,
        "receipt_bytes": receipt_bytes.len(),
        "toy_statement_image_id": image_id_hex(TOY_STATEMENT_ID),
        "direct_receipt_verifier_image_id": image_id_hex(DIRECT_RECEIPT_VERIFIER_ID),
        "inner_prove_ms": duration_ms(inner_duration),
        "direct_verify": {
            "status": "accepted",
            "user_cycles": valid.cycles,
            "segments": valid.segments,
            "wall_ms": duration_ms(valid.elapsed)
        },
        "lez_public_limit": lez_limit,
        "tampered_verify": {
            "status": "rejected",
            "error_contains": "S10 direct cryptographic receipt verification failed",
            "wall_ms": duration_ms(tampered)
        }
    });
    fs::write(
        out_dir.join(format!("real-{receipt_kind}-report.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

struct ExecutionResult {
    cycles: u64,
    segments: usize,
    elapsed: Duration,
}

fn execute_direct_verifier(receipt: &Receipt, receipt_bytes: &[u8]) -> Result<ExecutionResult> {
    let env = verifier_env(receipt, receipt_bytes, None)?;
    let started = Instant::now();
    let session = default_executor()
        .execute(env, DIRECT_RECEIPT_VERIFIER_ELF)
        .context("direct verifier guest execution")?;
    let elapsed = started.elapsed();
    let verified: bool = session.journal.decode()?;
    ensure!(verified, "direct verifier journal should commit true");
    Ok(ExecutionResult {
        cycles: session.cycles(),
        segments: session.segments.len(),
        elapsed,
    })
}

fn execute_rejected_verifier(receipt: &Receipt, receipt_bytes: &[u8]) -> Result<Duration> {
    let env = verifier_env(receipt, receipt_bytes, None)?;
    let started = Instant::now();
    let error = default_executor()
        .execute(env, DIRECT_RECEIPT_VERIFIER_ELF)
        .expect_err("tampered receipt unexpectedly verified")
        .to_string();
    let elapsed = started.elapsed();
    ensure!(
        error.contains("S10 direct cryptographic receipt verification failed"),
        "tampered receipt failed unexpectedly: {error}"
    );
    Ok(elapsed)
}

fn execute_with_lez_public_limit(
    receipt: &Receipt,
    receipt_bytes: &[u8],
) -> Result<serde_json::Value> {
    const LEZ_PUBLIC_CYCLE_LIMIT: u64 = 1024 * 1024 * 32;

    let env = verifier_env(receipt, receipt_bytes, Some(LEZ_PUBLIC_CYCLE_LIMIT))?;
    let started = Instant::now();
    match default_executor().execute(env, DIRECT_RECEIPT_VERIFIER_ELF) {
        Ok(session) => Ok(json!({
            "status": "accepted",
            "configured_user_cycle_limit": LEZ_PUBLIC_CYCLE_LIMIT,
            "user_cycles": session.cycles(),
            "segments": session.segments.len(),
            "wall_ms": duration_ms(started.elapsed())
        })),
        Err(error) => Ok(json!({
            "status": "rejected",
            "configured_user_cycle_limit": LEZ_PUBLIC_CYCLE_LIMIT,
            "error": error.to_string(),
            "wall_ms": duration_ms(started.elapsed())
        })),
    }
}

fn verifier_env<'a>(
    receipt: &Receipt,
    receipt_bytes: &[u8],
    session_limit: Option<u64>,
) -> Result<ExecutorEnv<'a>> {
    let mut builder = ExecutorEnv::builder();
    builder.session_limit(session_limit);
    builder
        .write(&receipt_bytes.to_vec())?
        .write(&TOY_STATEMENT_ID)?
        .write(&receipt.journal.bytes)?;
    builder.build()
}

fn dev_mode_enabled() -> bool {
    env::var("RISC0_DEV_MODE")
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false)
}

fn image_id_hex(words: [u32; 8]) -> String {
    words
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
