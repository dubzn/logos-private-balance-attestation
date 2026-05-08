#!/usr/bin/env bash
# Local-sequencer E2E for LP-0005.
#
# Pipeline:
#   1. Build the CLI.
#   2. Check wallet/sequencer health.
#   3. Build witness.json + gate.json from real wallet private state and the
#      real getProofForCommitment path.
#   4. Prove with attestation-cli.
#   5. Verify the envelope off-chain.
#   6. Write a Markdown/JSON run report.
#
# Required:
#   PRIVATE_ACCOUNT=Private/<id> or <id-without-prefix>
#
# Optional:
#   LOGOS_LEZ_REPO or LEZ_REPO       Defaults to ../logos-execution-zone when present.
#   NSSA_WALLET_HOME_DIR            Defaults to $LOGOS_LEZ_REPO/.wallet-local
#   THRESHOLD                       Defaults to 1
#   RISC0_DEV_MODE                  Defaults to 1
#   DEMO_DIR                        Defaults to .demo-runs/local-sequencer/<timestamp>
#   CHAIN_ID_HEX                    Defaults to 0x10 repeated 32 bytes
#   VERIFIER_ID_HEX                 Defaults to 0x20 repeated 32 bytes
#   GATE_ID_HEX                     Defaults to 0x30 repeated 32 bytes
#   PRESENTATION_CHALLENGE_HEX      Defaults to 0x44 repeated 32 bytes
#   PRESENTER_SECRET_HEX            Defaults to 0x55 repeated 32 bytes

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEMO_DIR="${DEMO_DIR:-$ROOT_DIR/.demo-runs/local-sequencer/$TIMESTAMP}"
DEV_MODE="${RISC0_DEV_MODE:-1}"
THRESHOLD="${THRESHOLD:-1}"

repeat_byte_hex() {
  local byte="$1"
  printf "%0.s$byte" {1..32}
}

CHAIN_ID_HEX="${CHAIN_ID_HEX:-$(repeat_byte_hex 10)}"
VERIFIER_ID_HEX="${VERIFIER_ID_HEX:-$(repeat_byte_hex 20)}"
GATE_ID_HEX="${GATE_ID_HEX:-$(repeat_byte_hex 30)}"
PRESENTATION_CHALLENGE_HEX="${PRESENTATION_CHALLENGE_HEX:-$(repeat_byte_hex 44)}"
PRESENTER_SECRET_HEX="${PRESENTER_SECRET_HEX:-$(repeat_byte_hex 55)}"

usage() {
  cat >&2 <<'EOF'
usage:
  PRIVATE_ACCOUNT=Private/<id> scripts/demo-local-sequencer-e2e.sh

env:
  LOGOS_LEZ_REPO or LEZ_REPO       Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR            Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.
  THRESHOLD                       Public threshold to prove. Defaults to 1.
  RISC0_DEV_MODE                  Defaults to 1. Use 0 for real proving.
  DEMO_DIR                        Output directory.
  CHAIN_ID_HEX                    32-byte hex context field.
  VERIFIER_ID_HEX                 32-byte hex context field.
  GATE_ID_HEX                     32-byte hex context field.
  PRESENTATION_CHALLENGE_HEX      32-byte verifier session challenge.
  PRESENTER_SECRET_HEX            32-byte hex Schnorr presenter secret.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
  echo "PRIVATE_ACCOUNT is required." >&2
  usage
  exit 2
fi

require_logos_lez_repo "$ROOT_DIR" wallet nssa/core
export_default_wallet_home

PRIVATE_ACCOUNT_ID="${PRIVATE_ACCOUNT#Private/}"

mkdir -p "$DEMO_DIR"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/balance-attest-local-e2e.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

WITNESS_JSON="$DEMO_DIR/witness.json"
GATE_JSON="$DEMO_DIR/gate.json"
BUILD_SUMMARY_JSON="$DEMO_DIR/build-witness-summary.json"
ENVELOPE_JSON="$DEMO_DIR/envelope.json"
VERIFY_JSON="$DEMO_DIR/verify.json"
RUN_JSON="$DEMO_DIR/run.json"
REPORT="$DEMO_DIR/report.md"
HEALTH_LOG="$DEMO_DIR/wallet-health.log"
PROVE_LOG="$DEMO_DIR/prove.log"

step() {
  printf '\n\033[1;36m== %s ==\033[0m\n' "$1"
}

note() {
  printf '   %s\n' "$1"
}

duration() {
  local start="$1"
  local end
  end="$(date +%s)"
  printf "%02d:%02d:%02d" $(((end - start) / 3600)) $((((end - start) % 3600) / 60)) $(((end - start) % 60))
}

TOTAL_STARTED="$(date +%s)"

step "1/6 Build CLI"
build_started="$(date +%s)"
note "Output: $DEMO_DIR"
note "LOGOS_LEZ_REPO=$LOGOS_LEZ_REPO"
note "NSSA_WALLET_HOME_DIR=$NSSA_WALLET_HOME_DIR"
note "RISC0_DEV_MODE=$DEV_MODE"
cargo build --quiet -p attestation-cli
build_duration="$(duration "$build_started")"

step "2/6 Check wallet/sequencer health"
health_started="$(date +%s)"
require_wallet_health "$HEALTH_LOG"
health_duration="$(duration "$health_started")"
note "wallet health log: $HEALTH_LOG"

step "3/6 Build real witness from wallet + getProofForCommitment"
mkdir -p "$TMP_DIR/src"
cat > "$TMP_DIR/Cargo.toml" <<EOF
[package]
name = "balance_attest_local_sequencer_e2e_witness"
version = "0.1.0"
edition = "2021"

[dependencies]
attestation-core = { path = "$ROOT_DIR/crates/attestation-core" }
attestation-prover = { path = "$ROOT_DIR/crates/attestation-prover" }
nssa = { path = "$LOGOS_LEZ_REPO/nssa" }
nssa_core = { path = "$LOGOS_LEZ_REPO/nssa/core", features = ["host"] }
wallet = { path = "$LOGOS_LEZ_REPO/wallet" }
anyhow = "1"
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
EOF

cat > "$TMP_DIR/src/main.rs" <<'EOF'
use std::{fs, path::PathBuf};

use anyhow::{ensure, Context, Result};
use attestation_core::{Digest32, HexBytes, LezMembershipProof, PresenterSecret};
use attestation_prover::{
    balance_attestation_image_id, build_balance_attestation_witness,
    build_private_account_inspect_report, inspect_membership_proof, AttestationPublicParams,
    BalanceAttestationWitness, BalanceAttestationWitnessSummary, PresenterWitness,
    PrivateAccountInspectReport, PrivateAccountInspectSource, PrivateAccountInspectStatus,
    PrivateAccountWitness,
};
use nssa::AccountId;
use nssa_core::compute_digest_for_path;
use serde::Serialize;
use wallet::WalletCore;

#[derive(Serialize)]
struct ProveInput<'a> {
    witness: &'a BalanceAttestationWitness,
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
    threshold: String,
}

#[derive(Serialize)]
struct BuildSummary {
    inspect: PrivateAccountInspectReport,
    witness: BalanceAttestationWitnessSummary,
    witness_path: String,
    gate_path: String,
    redaction_note: &'static str,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let account_id_raw = args
        .next()
        .context("usage: build <private-account-id> <threshold> <chain-id-hex> <verifier-id-hex> <gate-id-hex> <presentation-challenge-hex> <presenter-secret-hex> <out-dir>")?;
    let threshold: u128 = args
        .next()
        .context("missing threshold")?
        .parse()
        .context("threshold should be a u128 decimal")?;
    let chain_id = Digest32(parse_32_hex(
        "CHAIN_ID_HEX",
        &args.next().context("missing chain id")?,
    )?);
    let verifier_id = Digest32(parse_32_hex(
        "VERIFIER_ID_HEX",
        &args.next().context("missing verifier id")?,
    )?);
    let gate_id = Digest32(parse_32_hex(
        "GATE_ID_HEX",
        &args.next().context("missing gate id")?,
    )?);
    let presentation_challenge = Digest32(parse_32_hex(
        "PRESENTATION_CHALLENGE_HEX",
        &args.next().context("missing presentation challenge")?,
    )?);
    let presenter_secret = PresenterSecret::new(parse_32_hex(
        "PRESENTER_SECRET_HEX",
        &args.next().context("missing presenter secret")?,
    )?)
    .map_err(|error| anyhow::anyhow!("invalid presenter secret: {error}"))?;
    let out_dir = PathBuf::from(args.next().context("missing out dir")?);

    let account_id: AccountId = account_id_raw
        .parse()
        .context("private account id should be valid base58 without Private/ prefix")?;

    let wallet_core = WalletCore::from_env().context("wallet should initialize from env")?;
    let Some((key_chain, account, _identifier)) = wallet_core
        .storage()
        .user_data
        .get_private_account(account_id)
    else {
        anyhow::bail!("private account was not found in local wallet storage");
    };

    let wallet_commitment = wallet_core
        .get_private_account_commitment(account_id)
        .context("private account commitment should exist in local wallet storage")?;
    let wallet_commitment_bytes = wallet_commitment.to_byte_array();

    let private_account = PrivateAccountWitness {
        account_id: Digest32(account_id.into_value()),
        npk: Digest32(key_chain.nullifier_public_key.to_byte_array()),
        program_owner: account.program_owner,
        balance: account.balance,
        nonce: account.nonce.0,
        data: HexBytes::new(account.data.as_ref().to_vec()),
    };

    ensure!(
        private_account.commitment().as_bytes() == &wallet_commitment_bytes,
        "local commitment reconstruction did not match wallet commitment"
    );
    ensure!(
        private_account.balance >= threshold,
        "private account balance is below the requested threshold"
    );

    let membership_proof = wallet_core
        .check_private_account_initialized(account_id)
        .await
        .context("sequencer getProofForCommitment call should succeed")?
        .context("membership proof was not found for the current private commitment")?;

    let wallet_root = compute_digest_for_path(&wallet_commitment, &membership_proof);
    let proof = LezMembershipProof {
        index: membership_proof.0 as u64,
        siblings: membership_proof.1.iter().copied().map(Digest32).collect(),
    };
    let proof_inspection =
        inspect_membership_proof(&private_account, &proof, Some(Digest32(wallet_root)));
    ensure!(
        proof_inspection.core_root_matches_wallet_root,
        "core membership root did not match wallet/sequencer root"
    );

    let params = AttestationPublicParams {
        threshold,
        chain_id,
        verifier_id,
        gate_id,
        circuit_image_id: Digest32(balance_attestation_image_id()),
    };
    let witness = build_balance_attestation_witness(
        private_account,
        proof,
        PresenterWitness { presenter_secret },
        params,
    );

    fs::create_dir_all(&out_dir).context("create output dir")?;
    let witness_path = out_dir.join("witness.json");
    let gate_path = out_dir.join("gate.json");

    let prove_input = ProveInput {
        witness: &witness,
        chain_id,
        gate_id,
        presentation_challenge,
    };
    let gate = GateFile {
        chain_id,
        verifier_id,
        gate_id,
        presentation_challenge,
        threshold: threshold.to_string(),
    };

    fs::write(&witness_path, serde_json::to_string_pretty(&prove_input)?)
        .context("write witness json")?;
    fs::write(&gate_path, serde_json::to_string_pretty(&gate)?).context("write gate json")?;

    let inspect = build_private_account_inspect_report(PrivateAccountInspectStatus {
        account_id_raw,
        private_state_found: true,
        local_commitment_matches_wallet: true,
        membership_proof: Some(proof_inspection),
        source: PrivateAccountInspectSource::GetProofForCommitment,
    });
    let summary = BuildSummary {
        inspect,
        witness: witness.summary(),
        witness_path: witness_path.display().to_string(),
        gate_path: gate_path.display().to_string(),
        redaction_note: "witness.json is private and contains account id, npk, balance, nonce, account data, membership siblings, and presenter secret; this summary is sanitized",
    };

    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn parse_32_hex(name: &str, value: &str) -> Result<[u8; 32]> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(trimmed).with_context(|| format!("{name} should be valid hex"))?;
    ensure!(bytes.len() == 32, "{name} should be exactly 32 bytes");
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}
EOF

witness_started="$(date +%s)"
cargo run --manifest-path "$TMP_DIR/Cargo.toml" --quiet -- \
  "$PRIVATE_ACCOUNT_ID" \
  "$THRESHOLD" \
  "$CHAIN_ID_HEX" \
  "$VERIFIER_ID_HEX" \
  "$GATE_ID_HEX" \
  "$PRESENTATION_CHALLENGE_HEX" \
  "$PRESENTER_SECRET_HEX" \
  "$DEMO_DIR" \
  > "$BUILD_SUMMARY_JSON"
witness_duration="$(duration "$witness_started")"
note "witness: $WITNESS_JSON (private)"
note "gate:    $GATE_JSON"
note "summary: $BUILD_SUMMARY_JSON"

step "4/6 Prove balance attestation"
prove_started="$(date +%s)"
RISC0_DEV_MODE="$DEV_MODE" cargo run --quiet -p attestation-cli -- \
  prove \
  --witness "$WITNESS_JSON" \
  --out "$ENVELOPE_JSON" \
  2> >(tee "$PROVE_LOG" >&2)
prove_duration="$(duration "$prove_started")"
envelope_bytes="$(wc -c < "$ENVELOPE_JSON" | tr -d ' ')"
note "envelope: $ENVELOPE_JSON (${envelope_bytes} bytes)"
note "prove log: $PROVE_LOG"

step "5/6 Verify envelope"
verify_started="$(date +%s)"
VERIFY_OUTPUT="$(
  RISC0_DEV_MODE="$DEV_MODE" cargo run --quiet -p attestation-cli -- \
    verify \
    --envelope "$ENVELOPE_JSON" \
    --gate "$GATE_JSON"
)"
verify_duration="$(duration "$verify_started")"
printf '%s\n' "$VERIFY_OUTPUT" > "$VERIFY_JSON"
python3 -m json.tool "$VERIFY_JSON" 2>/dev/null || cat "$VERIFY_JSON"

step "6/6 Write run report"
total_duration="$(duration "$TOTAL_STARTED")"
cat > "$RUN_JSON" <<EOF
{
  "status": "ok",
  "risc0_dev_mode": "$DEV_MODE",
  "threshold": "$THRESHOLD",
  "private_account": "Private/<redacted>",
  "demo_dir": "$DEMO_DIR",
  "witness_json": "$WITNESS_JSON",
  "gate_json": "$GATE_JSON",
  "envelope_json": "$ENVELOPE_JSON",
  "verify_json": "$VERIFY_JSON",
  "build_summary_json": "$BUILD_SUMMARY_JSON",
  "report_md": "$REPORT",
  "durations": {
    "build": "$build_duration",
    "wallet_health": "$health_duration",
    "build_witness": "$witness_duration",
    "prove": "$prove_duration",
    "verify": "$verify_duration",
    "total": "$total_duration"
  }
}
EOF

cat > "$REPORT" <<EOF
# Local Sequencer E2E

| Step | Command | Output | Duration |
| --- | --- | --- | --- |
| Build CLI | \`cargo build -p attestation-cli\` | workspace binary | $build_duration |
| Wallet health | \`wallet check-health\` | \`$HEALTH_LOG\` | $health_duration |
| Build witness | wallet + \`getProofForCommitment\` temp adapter | \`$BUILD_SUMMARY_JSON\` | $witness_duration |
| Prove | \`balance-attest prove\` | \`$ENVELOPE_JSON\` | $prove_duration |
| Verify | \`balance-attest verify\` | \`$VERIFY_JSON\` | $verify_duration |
| Total | - | \`$RUN_JSON\` | $total_duration |

## Notes

- \`witness.json\` is private. Do not publish it.
- The public proof envelope is \`$ENVELOPE_JSON\`.
- \`RISC0_DEV_MODE=$DEV_MODE\`.
- This run uses real wallet private state and the real wallet/sequencer
  \`getProofForCommitment\` path.
EOF

cat "$REPORT"

printf '\n\033[1;32mLocal sequencer E2E complete.\033[0m Artifacts in: %s\n' "$DEMO_DIR"
if [[ "$DEV_MODE" != "0" ]]; then
  printf '\033[1;33mNote:\033[0m RISC0_DEV_MODE=%s — receipts are NOT cryptographically valid.\n' "$DEV_MODE"
  printf '       Re-run with RISC0_DEV_MODE=0 for the real-prover variant.\n'
fi
