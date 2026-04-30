#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
PROGRAM_BIN="$LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/private_balance_gate.bin"
ACCOUNT_WAIT_SECONDS="${ACCOUNT_WAIT_SECONDS:-45}"
ACCOUNT_POLL_SECONDS="${ACCOUNT_POLL_SECONDS:-3}"
GATE_MARKER_VALUE="${GATE_MARKER:-private-balance-gate-ok}"
THRESHOLD_VALUE="${THRESHOLD:-1}"

if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
  echo "Set PRIVATE_ACCOUNT to a local private account id without Private/." >&2
  exit 1
fi

if [[ -z "${GATE_ACCOUNT:-}" ]]; then
  echo "Set GATE_ACCOUNT to a local public account id without Public/." >&2
  exit 1
fi

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEZ_REPO/.wallet-local}"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  "$ROOT_DIR/scripts/spike-01-build-private-gate.sh"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing private_balance_gate binary: $PROGRAM_BIN" >&2
  exit 1
fi

if [[ "${SKIP_DEPLOY:-0}" != "1" ]]; then
  echo "[1/2] Deploying private_balance_gate program"
  wallet deploy-program "$PROGRAM_BIN"
else
  echo "[1/2] Skipping deploy"
fi

echo "[2/2] Running private balance gate"
cd "$LEZ_REPO/examples/program_deployment"

cargo run --bin run_private_balance_gate \
  "$PROGRAM_BIN" \
  "$PRIVATE_ACCOUNT" \
  "$GATE_ACCOUNT" \
  "$THRESHOLD_VALUE" \
  "$GATE_MARKER_VALUE"

marker_hex="$(printf '%s' "$GATE_MARKER_VALUE" | od -An -tx1 | tr -d ' \n')"
account_output=""

echo "Polling public gate account for up to ${ACCOUNT_WAIT_SECONDS}s..."
deadline=$((SECONDS + ACCOUNT_WAIT_SECONDS))
while (( SECONDS <= deadline )); do
  account_output="$(wallet account get --account-id "Public/$GATE_ACCOUNT" 2>&1 || true)"

  if printf '%s\n' "$account_output" | grep -qi "$marker_hex"; then
    echo "Private balance gate marker found in public gate account data."
    exit 0
  fi

  sleep "$ACCOUNT_POLL_SECONDS"
done

cat >&2 <<REPORT

Private balance gate marker was not found in the public gate account.

Expected marker hex:
$marker_hex

Account output:
$account_output

Check sequencer logs for the exact failure.
REPORT

exit 1
