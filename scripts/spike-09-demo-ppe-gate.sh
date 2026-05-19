#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

require_logos_lez_repo "$ROOT_DIR" Cargo.toml wallet examples/program_deployment/Cargo.toml
export_default_wallet_home

AMOUNT="${AMOUNT:-42}"
PASS_THRESHOLD="${PASS_THRESHOLD:-1}"
FAIL_THRESHOLD="${FAIL_THRESHOLD:-$((AMOUNT + 1))}"
DEPLOY_SETTLE_SECONDS="${DEPLOY_SETTLE_SECONDS:-20}"
ACCOUNT_WAIT_SECONDS="${ACCOUNT_WAIT_SECONDS:-90}"
FUNDING_WAIT_SECONDS="${FUNDING_WAIT_SECONDS:-120}"
PRIVATE_FUNDING_WAIT_SECONDS="${PRIVATE_FUNDING_WAIT_SECONDS:-180}"
RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
RUN_LABEL_SUFFIX="${RUN_LABEL_SUFFIX:-$(date -u +%Y%m%dT%H%M%SZ)}"
export RISC0_DEV_MODE

extract_account_id() {
  local prefix="$1"
  sed -n "s/.*account_id ${prefix}\/\([^[:space:]]*\).*/\1/p" | tail -n 1
}

new_public_account() {
  local label="$1"
  local output
  local account_id
  output="$(wallet account new public --label "$label")"
  printf '%s\n' "$output" >&2
  account_id="$(printf '%s\n' "$output" | extract_account_id Public)"
  if [[ -z "$account_id" ]]; then
    echo "Could not parse public account id for $label." >&2
    exit 1
  fi
  printf 'Public/%s\n' "$account_id"
}

new_private_account() {
  local label="$1"
  local output
  local account_id
  output="$(wallet account new private --label "$label")"
  printf '%s\n' "$output" >&2
  account_id="$(printf '%s\n' "$output" | extract_account_id Private)"
  if [[ -z "$account_id" ]]; then
    echo "Could not parse private account id for $label." >&2
    exit 1
  fi
  printf 'Private/%s\n' "$account_id"
}

nullifier_count() {
  local gate_account="$1"
  local nullifier_hex="$2"
  local data
  data="$(wallet account get --account-id "$gate_account" 2>/dev/null | sed -n 's/.*"data":"\([^"]*\)".*/\1/p' | tail -n 1)"
  awk -v haystack="$data" -v needle="$nullifier_hex" '
    BEGIN {
      count = 0
      while ((idx = index(haystack, needle)) > 0) {
        count += 1
        haystack = substr(haystack, idx + length(needle))
      }
      print count
    }
  '
}

account_balance() {
  local account="$1"
  wallet account get --account-id "$account" 2>/dev/null \
    | python3 -c '
import json
import sys

balance = ""
for line in sys.stdin:
    line = line.strip()
    if not line.startswith("{"):
        continue
    try:
        value = json.loads(line)
    except json.JSONDecodeError:
        continue
    if "balance" in value:
        balance = str(value["balance"])
print(balance)
'
}

wait_for_balance_at_least() {
  local account="$1"
  local expected="$2"
  local seconds="$3"
  local sync_private="${4:-0}"
  local deadline=$((SECONDS + seconds))
  local balance=""

  while (( SECONDS <= deadline )); do
    if [[ "$sync_private" == "1" ]]; then
      wallet account sync-private >/dev/null 2>&1 || true
    fi

    balance="$(account_balance "$account")"
    if [[ -n "$balance" ]] && (( balance >= expected )); then
      echo "Observed $account balance=$balance"
      return 0
    fi
    sleep 5
  done

  echo "Timed out waiting for $account balance >= $expected (last=${balance:-missing})." >&2
  wallet account get --account-id "$account" >&2 || true
  return 1
}

run_wallet_maybe_pending() {
  local label="$1"
  shift
  local output
  local status

  set +e
  output="$("$@" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"

  if [[ "$status" -eq 0 ]]; then
    return 0
  fi

  if printf '%s\n' "$output" | grep -q "Transaction not found in preconfigured amount of blocks"; then
    echo "$label returned a wallet wait timeout after submission; continuing with explicit polling."
    return 0
  fi

  echo "$label failed." >&2
  return "$status"
}

echo "[0/8] Checking wallet and sequencer health"
wallet check-health

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "[1/8] Building PPE gate"
  "$ROOT_DIR/scripts/spike-09-build-ppe-gate.sh"
else
  echo "[1/8] Skipping build"
fi

PROGRAM_BIN="$LOGOS_LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/ppe_balance_gate.bin"
if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing ppe_balance_gate binary: $PROGRAM_BIN" >&2
  exit 1
fi

echo "[2/8] Deploying PPE gate"
wallet deploy-program "$PROGRAM_BIN"
sleep "$DEPLOY_SETTLE_SECONDS"

echo "[3/8] Creating fresh accounts"
FUNDING_ACCOUNT="$(new_public_account "ppe-funding-$RUN_LABEL_SUFFIX")"
PRIVATE_ACCOUNT="$(new_private_account "ppe-holder-$RUN_LABEL_SUFFIX")"
GATE_ACCOUNT="$(new_public_account "ppe-gate-$RUN_LABEL_SUFFIX")"
PRESENTER_ACCOUNT="$(new_public_account "ppe-presenter-$RUN_LABEL_SUFFIX")"

echo "Funding account: $FUNDING_ACCOUNT"
echo "Private holder: $PRIVATE_ACCOUNT"
echo "Gate account: $GATE_ACCOUNT"
echo "Presenter account: $PRESENTER_ACCOUNT"

echo "[4/8] Initializing and funding private account"
run_wallet_maybe_pending "wallet auth-transfer init" \
  wallet auth-transfer init --account-id "$FUNDING_ACCOUNT"
wait_for_balance_at_least "$FUNDING_ACCOUNT" 0 "$FUNDING_WAIT_SECONDS"

run_wallet_maybe_pending "wallet pinata claim" \
  wallet pinata claim --to "$FUNDING_ACCOUNT"
wait_for_balance_at_least "$FUNDING_ACCOUNT" "$AMOUNT" "$FUNDING_WAIT_SECONDS"

run_wallet_maybe_pending "wallet auth-transfer send" \
  wallet auth-transfer send \
  --from "$FUNDING_ACCOUNT" \
  --to "$PRIVATE_ACCOUNT" \
  --amount "$AMOUNT"
wait_for_balance_at_least "$PRIVATE_ACCOUNT" "$AMOUNT" "$PRIVATE_FUNDING_WAIT_SECONDS" 1
wallet account get --account-id "$PRIVATE_ACCOUNT"

echo "[5/8] Positive PPE gate admit"
positive_output="$(
  PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
  GATE_ACCOUNT="$GATE_ACCOUNT" \
  PRESENTER_ACCOUNT="$PRESENTER_ACCOUNT" \
  THRESHOLD="$PASS_THRESHOLD" \
  SKIP_BUILD=1 \
  SKIP_DEPLOY=1 \
  ACCOUNT_WAIT_SECONDS="$ACCOUNT_WAIT_SECONDS" \
  "$ROOT_DIR/scripts/spike-09-run-ppe-gate.sh" 2>&1
)"
printf '%s\n' "$positive_output"
NULLIFIER_HEX="$(printf '%s\n' "$positive_output" | sed -n 's/^context_nullifier=//p' | tail -n 1)"
if [[ -z "$NULLIFIER_HEX" ]]; then
  echo "Could not parse context_nullifier from positive output." >&2
  exit 1
fi

wallet account sync-private

echo "[6/8] Duplicate PPE gate admit should not append the same nullifier twice"
set +e
duplicate_output="$(
  PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
  GATE_ACCOUNT="$GATE_ACCOUNT" \
  PRESENTER_ACCOUNT="$PRESENTER_ACCOUNT" \
  THRESHOLD="$PASS_THRESHOLD" \
  SKIP_BUILD=1 \
  SKIP_DEPLOY=1 \
  ACCOUNT_WAIT_SECONDS=25 \
  "$ROOT_DIR/scripts/spike-09-run-ppe-gate.sh" 2>&1
)"
duplicate_status=$?
set -e
printf '%s\n' "$duplicate_output"

sleep 20
count="$(nullifier_count "$GATE_ACCOUNT" "$NULLIFIER_HEX")"
if (( count != 1 )); then
  echo "Expected exactly one nullifier after duplicate probe, found $count." >&2
  exit 1
fi

if [[ "$duplicate_status" -eq 0 ]]; then
  echo "Duplicate command returned success, but nullifier count stayed at one."
else
  echo "Duplicate command failed or timed out as expected; nullifier count stayed at one."
fi

echo "[7/8] Negative PPE gate should fail above balance"
FAIL_GATE_ACCOUNT="$(new_public_account "ppe-gate-fail-$RUN_LABEL_SUFFIX")"
set +e
negative_output="$(
  PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
  GATE_ACCOUNT="$FAIL_GATE_ACCOUNT" \
  PRESENTER_ACCOUNT="$PRESENTER_ACCOUNT" \
  THRESHOLD="$FAIL_THRESHOLD" \
  SKIP_BUILD=1 \
  SKIP_DEPLOY=1 \
  ACCOUNT_WAIT_SECONDS=25 \
  "$ROOT_DIR/scripts/spike-09-run-ppe-gate.sh" 2>&1
)"
negative_status=$?
set -e
printf '%s\n' "$negative_output"

if [[ "$negative_status" -eq 0 ]]; then
  echo "Negative PPE gate unexpectedly succeeded." >&2
  exit 1
fi

if ! printf '%s\n' "$negative_output" | grep -q "private balance is below threshold"; then
  echo "Negative PPE gate failed, but not with the expected threshold error." >&2
  exit 1
fi

cat <<REPORT

Spike 09 PPE-native gate demo passed.

Gate account:
$GATE_ACCOUNT

Presenter account:
$PRESENTER_ACCOUNT

Context nullifier:
$NULLIFIER_HEX

This demonstrates a Logos-native private execution path:
private holder balance >= threshold -> public gate/nullifier state update.
REPORT
