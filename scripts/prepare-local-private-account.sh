#!/usr/bin/env bash
# Prepare a local private LEZ account for LP-0005 demos.
#
# This helper is intentionally local-first and testnet/devnet agnostic. It:
#   1. checks wallet/sequencer health
#   2. initializes the private account under authenticated-transfer if needed
#   3. funds it through the local Pinata program if balance < THRESHOLD
#   4. verifies that getProofForCommitment is available
#
# Verbose wallet output is written to OUT_DIR logs to keep terminal output sane.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.demo-runs/prepare-private-account/$TIMESTAMP}"
THRESHOLD="${THRESHOLD:-1}"
PRIVATE_ACCOUNT="${PRIVATE_ACCOUNT:-${1:-}}"
RISC0_DEV_MODE="${RISC0_DEV_MODE:-0}"
export RISC0_DEV_MODE

usage() {
  cat >&2 <<'EOF'
usage:
  PRIVATE_ACCOUNT=Private/<id> [THRESHOLD=1] scripts/prepare-local-private-account.sh
  scripts/prepare-local-private-account.sh Private/<id>

env:
  LOGOS_LEZ_REPO or LEZ_REPO       Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR            Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.
  THRESHOLD                       Minimum private balance needed for the demo. Default: 1.
  RISC0_DEV_MODE                  Must match the running sequencer. Default: 0.
  OUT_DIR                         Output directory for logs.

This script uses the local Pinata program to fund the selected private account.
It is for local demos only.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "$PRIVATE_ACCOUNT" ]]; then
  echo "PRIVATE_ACCOUNT is required." >&2
  usage
  exit 2
fi

if [[ "$PRIVATE_ACCOUNT" != Private/* ]]; then
  PRIVATE_ACCOUNT="Private/$PRIVATE_ACCOUNT"
fi

if ! [[ "$THRESHOLD" =~ ^[0-9]+$ ]]; then
  echo "THRESHOLD must be a non-negative integer." >&2
  exit 2
fi

require_logos_lez_repo "$ROOT_DIR" Cargo.toml
lez_wallet_crate_rel_path >/dev/null
lez_core_crate_rel_path >/dev/null
export_default_wallet_home
export_default_risc0_recursion_cache "$ROOT_DIR"

mkdir -p "$OUT_DIR"

HEALTH_LOG="$OUT_DIR/wallet-health.log"
BEFORE_LOG="$OUT_DIR/account-before.log"
INIT_LOG="$OUT_DIR/auth-transfer-init.log"
PINATA_LOG="$OUT_DIR/pinata-claim.log"
SYNC_LOG="$OUT_DIR/sync-private.log"
AFTER_LOG="$OUT_DIR/account-after.log"
INSPECT_LOG="$OUT_DIR/inspect-private.log"

note() {
  printf '%s\n' "$*"
}

account_get() {
  local out="$1"
  NSSA_WALLET_HOME_DIR="$NSSA_WALLET_HOME_DIR" \
  LEE_WALLET_HOME_DIR="$LEE_WALLET_HOME_DIR" \
    wallet account get --account-id "$PRIVATE_ACCOUNT" > "$out" 2>&1
}

account_is_uninitialized() {
  grep -q "Account is Uninitialized" "$1"
}

account_balance() {
  local log="$1"
  python3 - "$log" <<'PY'
import json
import sys

path = sys.argv[1]
balance = None
with open(path, "r", encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "balance" in value:
            balance = int(value["balance"])

print("" if balance is None else balance)
PY
}

run_logged() {
  local label="$1"
  local log="$2"
  shift 2
  note "  - $label..."
  if "$@" > "$log" 2>&1; then
    note "    ok ($log)"
  else
    local status=$?
    note "    failed ($log)"
    sed -n '1,80p' "$log" >&2 || true
    return "$status"
  fi
}

note "Preparing local private account for LP-0005 demo"
note "  LOGOS_LEZ_REPO=$LOGOS_LEZ_REPO"
note "  NSSA_WALLET_HOME_DIR=$NSSA_WALLET_HOME_DIR"
note "  LEE_WALLET_HOME_DIR=$LEE_WALLET_HOME_DIR"
note "  PRIVATE_ACCOUNT=$PRIVATE_ACCOUNT"
note "  THRESHOLD=$THRESHOLD"
note "  RISC0_DEV_MODE=$RISC0_DEV_MODE"
note "  OUT_DIR=$OUT_DIR"

note "  - wallet/sequencer health..."
if require_wallet_health "$HEALTH_LOG"; then
  note "    ok ($HEALTH_LOG)"
else
  note "    failed ($HEALTH_LOG)" >&2
  exit 1
fi

account_get "$BEFORE_LOG"
before_balance="$(account_balance "$BEFORE_LOG")"

if account_is_uninitialized "$BEFORE_LOG"; then
  note "Private account is uninitialized; initializing it under authenticated-transfer."
  run_logged \
    "wallet auth-transfer init" \
    "$INIT_LOG" \
    wallet auth-transfer init --account-id "$PRIVATE_ACCOUNT"
  run_logged "wallet account sync-private" "$SYNC_LOG" wallet account sync-private
  account_get "$BEFORE_LOG"
  before_balance="$(account_balance "$BEFORE_LOG")"
fi

if [[ -z "$before_balance" ]]; then
  before_balance=0
fi

if (( before_balance < THRESHOLD )); then
  note "Private balance ($before_balance) is below threshold ($THRESHOLD); claiming Pinata."
  run_logged \
    "wallet pinata claim" \
    "$PINATA_LOG" \
    wallet pinata claim --to "$PRIVATE_ACCOUNT"
  run_logged "wallet account sync-private" "$SYNC_LOG" wallet account sync-private
else
  note "Private balance already satisfies threshold: $before_balance >= $THRESHOLD."
fi

account_get "$AFTER_LOG"
after_balance="$(account_balance "$AFTER_LOG")"
if [[ -z "$after_balance" ]]; then
  after_balance=0
fi

if (( after_balance < THRESHOLD )); then
  {
    echo "Private account is still below threshold after preparation."
    echo "  balance=$after_balance threshold=$THRESHOLD"
    echo "  account log=$AFTER_LOG"
  } >&2
  exit 1
fi

run_logged \
  "attestation inspect-private --require-proof" \
  "$INSPECT_LOG" \
  cargo run -p attestation-cli -- inspect-private --account "$PRIVATE_ACCOUNT" --require-proof

note
note "Local private account is ready."
note "  balance=$after_balance"
note "  inspect log=$INSPECT_LOG"
note "  logs=$OUT_DIR"
