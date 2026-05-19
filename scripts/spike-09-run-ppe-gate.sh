#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

require_logos_lez_repo "$ROOT_DIR" Cargo.toml examples/program_deployment/Cargo.toml
export_default_wallet_home

PROGRAM_BIN="${PROGRAM_BIN:-$LOGOS_LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/ppe_balance_gate.bin}"
THRESHOLD="${THRESHOLD:-1}"
CHAIN_ID_HEX="${CHAIN_ID_HEX:-1111111111111111111111111111111111111111111111111111111111111111}"
VERIFIER_ID_HEX="${VERIFIER_ID_HEX:-2222222222222222222222222222222222222222222222222222222222222222}"
GATE_ID_HEX="${GATE_ID_HEX:-3333333333333333333333333333333333333333333333333333333333333333}"
ACCOUNT_WAIT_SECONDS="${ACCOUNT_WAIT_SECONDS:-90}"
ACCOUNT_POLL_SECONDS="${ACCOUNT_POLL_SECONDS:-5}"
RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
export RISC0_DEV_MODE

usage() {
  cat >&2 <<'EOF'
usage:
  PRIVATE_ACCOUNT=Private/<id> GATE_ACCOUNT=Public/<id> PRESENTER_ACCOUNT=Public/<id> \
    scripts/spike-09-run-ppe-gate.sh

env:
  LOGOS_LEZ_REPO / LEZ_REPO     logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR          Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.
  PROGRAM_BIN                   Optional ppe_balance_gate.bin override.
  THRESHOLD                     Gate threshold. Default: 1.
  CHAIN_ID_HEX, VERIFIER_ID_HEX, GATE_ID_HEX  32-byte hex gate context.
  SKIP_BUILD=1                  Do not rebuild/install sources.
  SKIP_DEPLOY=1                 Do not deploy program before running.
EOF
}

normalize_account() {
  local value="$1"
  local prefix="$2"
  if [[ "$value" == "$prefix/"* ]]; then
    printf '%s\n' "$value"
  else
    printf '%s/%s\n' "$prefix" "$value"
  fi
}

extract_gate_data_hex() {
  sed -n 's/.*"data":"\([^"]*\)".*/\1/p' | tail -n 1
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "${PRIVATE_ACCOUNT:-}" || -z "${GATE_ACCOUNT:-}" || -z "${PRESENTER_ACCOUNT:-}" ]]; then
  echo "PRIVATE_ACCOUNT, GATE_ACCOUNT, and PRESENTER_ACCOUNT are required." >&2
  usage
  exit 2
fi

PRIVATE_ACCOUNT="$(normalize_account "$PRIVATE_ACCOUNT" Private)"
GATE_ACCOUNT="$(normalize_account "$GATE_ACCOUNT" Public)"
PRESENTER_ACCOUNT="$(normalize_account "$PRESENTER_ACCOUNT" Public)"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  "$ROOT_DIR/scripts/spike-09-build-ppe-gate.sh"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing PPE gate program binary: $PROGRAM_BIN" >&2
  exit 1
fi

if [[ "${SKIP_DEPLOY:-0}" != "1" ]]; then
  echo "[1/3] Deploying ppe_balance_gate program"
  wallet deploy-program "$PROGRAM_BIN"
else
  echo "[1/3] Skipping deploy"
fi

echo "[2/3] Running PPE-native private balance gate"
cd "$LOGOS_LEZ_REPO/examples/program_deployment"
set +e
runner_output="$(
  cargo run --bin run_ppe_balance_gate -- \
  --program-bin "$PROGRAM_BIN" \
  --private-account "$PRIVATE_ACCOUNT" \
  --gate-account "$GATE_ACCOUNT" \
  --presenter-account "$PRESENTER_ACCOUNT" \
  --threshold "$THRESHOLD" \
  --chain-id-hex "$CHAIN_ID_HEX" \
  --verifier-id-hex "$VERIFIER_ID_HEX" \
  --gate-id-hex "$GATE_ID_HEX" 2>&1
)"
runner_status=$?
set -e
printf '%s\n' "$runner_output"
if [[ "$runner_status" -ne 0 ]]; then
  exit "$runner_status"
fi

EXPECTED_NULLIFIER_HEX="$(printf '%s\n' "$runner_output" | sed -n 's/^context_nullifier=//p' | tail -n 1)"
if [[ -z "$EXPECTED_NULLIFIER_HEX" ]]; then
  echo "Could not parse context_nullifier from runner output." >&2
  exit 1
fi

echo "[3/3] Polling gate account for nullifier"
deadline=$((SECONDS + ACCOUNT_WAIT_SECONDS))
account_output=""
while (( SECONDS <= deadline )); do
  account_output="$(wallet account get --account-id "$GATE_ACCOUNT" 2>&1 || true)"
  gate_data="$(printf '%s\n' "$account_output" | extract_gate_data_hex)"
  if [[ "$gate_data" == *"$EXPECTED_NULLIFIER_HEX"* ]]; then
    echo "PPE gate nullifier recorded."
    echo "context_nullifier=$EXPECTED_NULLIFIER_HEX"
    exit 0
  fi
  sleep "$ACCOUNT_POLL_SECONDS"
done

cat >&2 <<REPORT

PPE gate nullifier was not found in the public gate account.

Expected nullifier hex:
$EXPECTED_NULLIFIER_HEX

Account output:
$account_output

Check sequencer logs for the exact failure.
REPORT

exit 1
