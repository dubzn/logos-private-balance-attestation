#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
PROGRAM_BIN="$LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/private_balance_gate.bin"
AMOUNT="${AMOUNT:-42}"
PASS_THRESHOLD="${PASS_THRESHOLD:-1}"
FAIL_THRESHOLD="${FAIL_THRESHOLD:-$((AMOUNT + 1))}"
DEPLOY_SETTLE_SECONDS="${DEPLOY_SETTLE_SECONDS:-20}"
ACCOUNT_WAIT_SECONDS="${ACCOUNT_WAIT_SECONDS:-45}"

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEZ_REPO/.wallet-local}"

extract_account_id() {
  local prefix="$1"
  sed -n "s/.*account_id ${prefix}\/\([^[:space:]]*\).*/\1/p" | tail -n 1
}

new_public_account() {
  local output
  local account_id
  output="$(wallet account new public)"
  printf '%s\n' "$output" >&2
  account_id="$(printf '%s\n' "$output" | extract_account_id Public)"
  if [[ -z "$account_id" ]]; then
    echo "Could not parse public account id from wallet output." >&2
    exit 1
  fi
  printf '%s\n' "$account_id"
}

new_private_account() {
  local output
  local account_id
  output="$(wallet account new private)"
  printf '%s\n' "$output" >&2
  account_id="$(printf '%s\n' "$output" | extract_account_id Private)"
  if [[ -z "$account_id" ]]; then
    echo "Could not parse private account id from wallet output." >&2
    exit 1
  fi
  printf '%s\n' "$account_id"
}

echo "[0/7] Checking wallet and sequencer health"
wallet check-health

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "[1/7] Building private_balance_gate"
  "$ROOT_DIR/scripts/spike-01-build-private-gate.sh"
else
  echo "[1/7] Skipping build"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing private_balance_gate binary: $PROGRAM_BIN" >&2
  exit 1
fi

if [[ "${SKIP_DEPLOY:-0}" != "1" ]]; then
  echo "[2/7] Deploying private_balance_gate"
  wallet deploy-program "$PROGRAM_BIN"
  sleep "$DEPLOY_SETTLE_SECONDS"
else
  echo "[2/7] Skipping deploy"
fi

echo "[3/7] Creating fresh public funding and private holder accounts"
FUNDING_ACCOUNT="$(new_public_account)"
PRIVATE_ACCOUNT="$(new_private_account)"

echo "Funding account: Public/$FUNDING_ACCOUNT"
echo "Private holder: Private/$PRIVATE_ACCOUNT"

echo "[4/7] Initializing and funding public account"
wallet auth-transfer init --account-id "Public/$FUNDING_ACCOUNT"
wallet pinata claim --to "Public/$FUNDING_ACCOUNT"

echo "[5/7] Sending ${AMOUNT} tokens into private account"
wallet auth-transfer send \
  --from "Public/$FUNDING_ACCOUNT" \
  --to "Private/$PRIVATE_ACCOUNT" \
  --amount "$AMOUNT"

wallet account sync-private
wallet account get --account-id "Private/$PRIVATE_ACCOUNT"

echo "[6/7] Positive gate: threshold ${PASS_THRESHOLD}"
SUCCESS_GATE_ACCOUNT="$(new_public_account)"
PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
GATE_ACCOUNT="$SUCCESS_GATE_ACCOUNT" \
THRESHOLD="$PASS_THRESHOLD" \
SKIP_BUILD=1 \
SKIP_DEPLOY=1 \
ACCOUNT_WAIT_SECONDS="$ACCOUNT_WAIT_SECONDS" \
"$ROOT_DIR/scripts/spike-01-run-private-gate.sh"

wallet account sync-private

echo "[7/7] Negative gate: threshold ${FAIL_THRESHOLD}"
FAIL_GATE_ACCOUNT="$(new_public_account)"
set +e
negative_output="$(
  PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
  GATE_ACCOUNT="$FAIL_GATE_ACCOUNT" \
  THRESHOLD="$FAIL_THRESHOLD" \
  GATE_MARKER="private-balance-gate-fail-unexpected" \
  SKIP_BUILD=1 \
  SKIP_DEPLOY=1 \
  ACCOUNT_WAIT_SECONDS="$ACCOUNT_WAIT_SECONDS" \
  "$ROOT_DIR/scripts/spike-01-run-private-gate.sh" 2>&1
)"
negative_status=$?
set -e
printf '%s\n' "$negative_output"

if [[ "$negative_status" -eq 0 ]]; then
  echo "Negative gate unexpectedly succeeded." >&2
  exit 1
fi

if ! printf '%s\n' "$negative_output" | grep -q "private balance is below threshold"; then
  echo "Negative gate failed, but not with the expected threshold error." >&2
  exit 1
fi

cat <<REPORT

Spike 01 demo passed.

Positive gate account:
Public/$SUCCESS_GATE_ACCOUNT

Negative gate account:
Public/$FAIL_GATE_ACCOUNT

The positive gate wrote its marker on-chain. The negative gate did not, as
expected, because the private balance (${AMOUNT}) is below threshold
${FAIL_THRESHOLD}.
REPORT
