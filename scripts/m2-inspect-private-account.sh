#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$REPO_ROOT/scripts/common-env.sh"
RESULT_DIR="${RESULT_DIR:-$REPO_ROOT/.spike-results/m2-private-account-inspect}"
TIMESTAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
REPORT="$RESULT_DIR/$TIMESTAMP.md"
OUTPUT_JSON="$RESULT_DIR/$TIMESTAMP.json"
OUTPUT_STDERR="$RESULT_DIR/$TIMESTAMP.stderr.log"
HEALTH_LOG="$RESULT_DIR/$TIMESTAMP-wallet-health.log"
REQUIRE_PROOF=0
LOCAL_ONLY=0

usage() {
  cat >&2 <<'EOF'
usage:
  PRIVATE_ACCOUNT=<private-account-id-without-Private> scripts/m2-inspect-private-account.sh [--local-only|--require-proof]

env:
  LOGOS_LEZ_REPO or LEZ_REPO       Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR            Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --local-only)
      LOCAL_ONLY=1
      shift
      ;;
    --require-proof)
      REQUIRE_PROOF=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
  echo "PRIVATE_ACCOUNT is required." >&2
  usage
  exit 2
fi

if [[ "$LOCAL_ONLY" == "1" && "$REQUIRE_PROOF" == "1" ]]; then
  echo "--local-only and --require-proof are mutually exclusive." >&2
  usage
  exit 2
fi

require_logos_lez_repo "$REPO_ROOT" Cargo.toml
LEZ_STATE_REL_PATH="$(lez_state_crate_rel_path)"
LEZ_STATE_PACKAGE_NAME="$(lez_state_crate_package_name)"
LEZ_CORE_REL_PATH="$(lez_core_crate_rel_path)"
LEZ_CORE_PACKAGE_NAME="$(lez_core_crate_package_name)"
LEZ_WALLET_REL_PATH="$(lez_wallet_crate_rel_path)"
export_default_wallet_home
export_default_risc0_recursion_cache "$REPO_ROOT"
export_macos_python_framework_rustflags

mkdir -p "$RESULT_DIR"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/balance-attest-m2-wallet.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

duration() {
  local start="$1"
  local end
  end="$(date +%s)"
  printf "%02d:%02d:%02d" $(((end - start) / 3600)) $((((end - start) % 3600) / 60)) $(((end - start) % 60))
}

status="ok"
started="$(date +%s)"

if [[ "$LOCAL_ONLY" == "1" ]]; then
  health_status="skipped"
  health_duration="00:00:00"
  echo "skipped because --local-only was set" > "$HEALTH_LOG"
else
  health_started="$(date +%s)"
  if ! require_wallet_storage > "$HEALTH_LOG" 2>&1; then
    health_status="fail"
    status="fail"
  elif wallet check-health < /dev/null >> "$HEALTH_LOG" 2>&1; then
    health_status="ok"
  else
    health_status="fail"
    status="fail"
  fi
  health_duration="$(duration "$health_started")"
fi

inspect_status="skipped"
inspect_duration="00:00:00"

if [[ "$health_status" == "ok" || "$LOCAL_ONLY" == "1" ]]; then
  mkdir -p "$TMP_DIR/src"
  cat > "$TMP_DIR/Cargo.toml" <<EOF
[package]
name = "balance_attest_m2_private_account_inspect"
version = "0.1.0"
edition = "2021"

[dependencies]
attestation-core = { path = "$REPO_ROOT/crates/attestation-core" }
attestation-prover = { path = "$REPO_ROOT/crates/attestation-prover" }
lez_state = { package = "$LEZ_STATE_PACKAGE_NAME", path = "$LOGOS_LEZ_REPO/$LEZ_STATE_REL_PATH" }
lez_core = { package = "$LEZ_CORE_PACKAGE_NAME", path = "$LOGOS_LEZ_REPO/$LEZ_CORE_REL_PATH", features = ["host"] }
wallet = { path = "$LOGOS_LEZ_REPO/$LEZ_WALLET_REL_PATH" }
anyhow = "1"
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
EOF

  cat > "$TMP_DIR/src/main.rs" <<'EOF'
use anyhow::{Context, Result};
use attestation_core::{Digest32, HexBytes, LezMembershipProof};
use attestation_prover::{
    build_private_account_inspect_report, inspect_membership_proof, PrivateAccountInspectSource,
    PrivateAccountInspectStatus, PrivateAccountWitness,
};
use lez_core::compute_digest_for_path;
use lez_state::AccountId;
use wallet::WalletCore;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let account_id_raw = args
        .next()
        .context("usage: inspect <private-account-id-without-Private> [--local-only|--require-proof]")?;
    let mut require_proof = false;
    let mut local_only = false;
    for arg in args {
        match arg.as_str() {
            "--require-proof" => require_proof = true,
            "--local-only" => local_only = true,
            _ => anyhow::bail!("unknown argument: {arg}"),
        }
    }
    if local_only && require_proof {
        anyhow::bail!("--local-only and --require-proof are mutually exclusive");
    }
    let account_id: AccountId = account_id_raw
        .parse()
        .context("private account id should be valid base58 without Private/ prefix")?;

    let wallet_core = WalletCore::from_env().context("wallet should initialize from env")?;
    let Some(found_account) = wallet_core
        .storage()
        .key_chain()
        .private_account(account_id)
    else {
        anyhow::bail!("private account was not found in local wallet storage");
    };
    let key_chain = found_account.key_chain;
    let account = found_account.account;

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
    let core_commitment = private_account.commitment();

    let membership_proof = if local_only {
        None
    } else {
        wallet_core
            .check_private_account_initialized(account_id)
            .await
            .context("sequencer getProofForCommitment call should succeed")?
    };

    if require_proof && membership_proof.is_none() {
        anyhow::bail!("membership proof was not found for the current private commitment");
    }

    let membership_proof = membership_proof.as_ref().map(|proof| {
        let wallet_root = compute_digest_for_path(&wallet_commitment, proof);
        let core_proof = LezMembershipProof {
            index: proof.0 as u64,
            siblings: proof.1.iter().copied().map(Digest32).collect(),
        };
        inspect_membership_proof(&private_account, &core_proof, Some(Digest32(wallet_root)))
    });

    let report = build_private_account_inspect_report(PrivateAccountInspectStatus {
        account_id_raw,
        private_state_found: true,
        local_commitment_matches_wallet: core_commitment.as_bytes() == &wallet_commitment_bytes,
        membership_proof,
        source: if local_only {
            PrivateAccountInspectSource::LocalOnly
        } else {
            PrivateAccountInspectSource::GetProofForCommitment
        },
    });

    println!("{}", serde_json::to_string_pretty(&report)?);

    Ok(())
}
EOF

  inspect_args=("$PRIVATE_ACCOUNT")
  if [[ "$LOCAL_ONLY" == "1" ]]; then
    inspect_args+=("--local-only")
  fi
  if [[ "$REQUIRE_PROOF" == "1" ]]; then
    inspect_args+=("--require-proof")
  fi

  inspect_started="$(date +%s)"
  if cargo run --manifest-path "$TMP_DIR/Cargo.toml" --quiet -- "${inspect_args[@]}" \
    > "$OUTPUT_JSON" 2> "$OUTPUT_STDERR"
  then
    inspect_status="ok"
  else
    inspect_status="fail"
    status="fail"
  fi
  inspect_duration="$(duration "$inspect_started")"
fi

total_duration="$(duration "$started")"

{
  echo "# M2 Private Account Inspect"
  echo
  echo "| Step | Command | Status | Output | Duration |"
  echo "| --- | --- | --- | --- | --- |"
  echo "| health | wallet check-health | $health_status | $HEALTH_LOG | $health_duration |"
  echo "| inspect | temp Cargo project using WalletCore | $inspect_status | $OUTPUT_JSON | $inspect_duration |"
  echo "| total | - | $status | $REPORT | $total_duration |"
  echo
  echo "## JSON Output"
  echo
  echo '```json'
  if [[ -s "$OUTPUT_JSON" ]]; then
    cat "$OUTPUT_JSON"
  else
    echo "null"
  fi
  echo '```'
  if [[ "$inspect_status" == "fail" && -s "$OUTPUT_STDERR" ]]; then
    echo
    echo "## Inspect stderr"
    echo
    echo '```text'
    sed -n '1,120p' "$OUTPUT_STDERR"
    echo '```'
  fi
} > "$REPORT"

cat "$REPORT"

if [[ "$status" != "ok" ]]; then
  exit 1
fi
