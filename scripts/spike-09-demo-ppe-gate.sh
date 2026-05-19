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

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEMO_DIR="${DEMO_DIR:-$ROOT_DIR/.demo-runs/spike-09-ppe-gate/$TIMESTAMP}"
LOG_DIR="$DEMO_DIR/logs"
REPORT="$DEMO_DIR/report.md"
RUN_JSON="$DEMO_DIR/run.json"
mkdir -p "$LOG_DIR"

duration() {
  local start="$1"
  local end
  end="$(date +%s)"
  printf "%02d:%02d:%02d" $(((end - start) / 3600)) $((((end - start) % 3600) / 60)) $(((end - start) % 60))
}

extract_account_id() {
  local prefix="$1"
  sed -n "s/.*account_id ${prefix}\/\([^[:space:]]*\).*/\1/p" | tail -n 1
}

display_account() {
  local account="$1"
  if [[ "$account" == Private/* ]]; then
    printf 'Private/<redacted>\n'
  else
    printf '%s\n' "$account"
  fi
}

sanitize_sensitive_output() {
  sed -E \
    -e 's#Private/[1-9A-HJ-NP-Za-km-z]{20,}#Private/<redacted>#g' \
    -e 's/(With npk )[0-9a-f]+/\1<redacted>/g' \
    -e 's/(With vpk )[0-9a-f]+/\1<redacted>/g'
}

new_public_account() {
  local label="$1"
  local output
  local account_id
  output="$(wallet account new public --label "$label" 2>&1)"
  account_id="$(printf '%s\n' "$output" | extract_account_id Public)"
  if [[ -z "$account_id" ]]; then
    echo "Could not parse public account id for $label." >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  printf 'Created public account %s: Public/%s\n' "$label" "$account_id" >&2
  printf 'Public/%s\n' "$account_id"
}

new_private_account() {
  local label="$1"
  local output
  local account_id
  output="$(wallet account new private --label "$label" 2>&1)"
  account_id="$(printf '%s\n' "$output" | extract_account_id Private)"
  if [[ -z "$account_id" ]]; then
    echo "Could not parse private account id for $label." >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  printf 'Created private account %s: Private/<redacted>\n' "$label" >&2
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
      echo "Observed $(display_account "$account") balance=$balance"
      return 0
    fi
    sleep 5
  done

  echo "Timed out waiting for $(display_account "$account") balance >= $expected (last=${balance:-missing})." >&2
  if [[ "$account" != Private/* ]]; then
    wallet account get --account-id "$account" >&2 || true
  fi
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
  printf '%s\n' "$output" | sanitize_sensitive_output

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

TOTAL_STARTED="$(date +%s)"

echo "[0/8] Checking wallet and sequencer health"
health_started="$(date +%s)"
wallet check-health
health_duration="$(duration "$health_started")"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  echo "[1/8] Building PPE gate"
  build_started="$(date +%s)"
  "$ROOT_DIR/scripts/spike-09-build-ppe-gate.sh"
  build_status="ok"
else
  echo "[1/8] Skipping build"
  build_started="$(date +%s)"
  build_status="skipped"
fi
build_duration="$(duration "$build_started")"

PROGRAM_BIN="$LOGOS_LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/ppe_balance_gate.bin"
if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing ppe_balance_gate binary: $PROGRAM_BIN" >&2
  exit 1
fi

echo "[2/8] Deploying PPE gate"
deploy_started="$(date +%s)"
wallet deploy-program "$PROGRAM_BIN"
sleep "$DEPLOY_SETTLE_SECONDS"
deploy_duration="$(duration "$deploy_started")"

echo "[3/8] Creating fresh accounts"
accounts_started="$(date +%s)"
FUNDING_ACCOUNT="$(new_public_account "ppe-funding-$RUN_LABEL_SUFFIX")"
PRIVATE_ACCOUNT="$(new_private_account "ppe-holder-$RUN_LABEL_SUFFIX")"
GATE_ACCOUNT="$(new_public_account "ppe-gate-$RUN_LABEL_SUFFIX")"
PRESENTER_ACCOUNT="$(new_public_account "ppe-presenter-$RUN_LABEL_SUFFIX")"
accounts_duration="$(duration "$accounts_started")"

echo "Funding account: $FUNDING_ACCOUNT"
echo "Private holder: Private/<redacted>"
echo "Gate account: $GATE_ACCOUNT"
echo "Presenter account: $PRESENTER_ACCOUNT"

echo "[4/8] Initializing and funding private account"
funding_started="$(date +%s)"
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
funding_duration="$(duration "$funding_started")"

echo "[5/8] Positive PPE gate admit"
positive_started="$(date +%s)"
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
printf '%s\n' "$positive_output" | sanitize_sensitive_output
NULLIFIER_HEX="$(printf '%s\n' "$positive_output" | sed -n 's/^context_nullifier=//p' | tail -n 1)"
if [[ -z "$NULLIFIER_HEX" ]]; then
  echo "Could not parse context_nullifier from positive output." >&2
  exit 1
fi
positive_duration="$(duration "$positive_started")"

wallet account sync-private

echo "[6/8] Duplicate PPE gate admit should not append the same nullifier twice"
duplicate_started="$(date +%s)"
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
printf '%s\n' "$duplicate_output" | sanitize_sensitive_output

sleep 20
count="$(nullifier_count "$GATE_ACCOUNT" "$NULLIFIER_HEX")"
if (( count != 1 )); then
  echo "Expected exactly one nullifier after duplicate probe, found $count." >&2
  exit 1
fi

if [[ "$duplicate_status" -eq 0 ]]; then
  echo "Duplicate command returned success, but nullifier count stayed at one."
  duplicate_result="success-no-append"
else
  echo "Duplicate command failed or timed out as expected; nullifier count stayed at one."
  duplicate_result="rejected-or-timeout"
fi
duplicate_duration="$(duration "$duplicate_started")"

echo "[7/8] Negative PPE gate should fail above balance"
negative_started="$(date +%s)"
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
printf '%s\n' "$negative_output" | sanitize_sensitive_output

if [[ "$negative_status" -eq 0 ]]; then
  echo "Negative PPE gate unexpectedly succeeded." >&2
  exit 1
fi

if ! printf '%s\n' "$negative_output" | grep -q "private balance is below threshold"; then
  echo "Negative PPE gate failed, but not with the expected threshold error." >&2
  exit 1
fi
negative_duration="$(duration "$negative_started")"
total_duration="$(duration "$TOTAL_STARTED")"

cat > "$RUN_JSON" <<EOF
{
  "status": "ok",
  "risc0_dev_mode": "$RISC0_DEV_MODE",
  "demo_dir": "$DEMO_DIR",
  "amount": "$AMOUNT",
  "pass_threshold": "$PASS_THRESHOLD",
  "fail_threshold": "$FAIL_THRESHOLD",
  "gate_account": "$GATE_ACCOUNT",
  "presenter_account": "$PRESENTER_ACCOUNT",
  "context_nullifier": "$NULLIFIER_HEX",
  "nullifier_count": "$count",
  "duplicate_result": "$duplicate_result",
  "negative_result": "BA201 ThresholdMismatch",
  "durations": {
    "wallet_health": "$health_duration",
    "build": "$build_duration",
    "deploy": "$deploy_duration",
    "create_accounts": "$accounts_duration",
    "fund_private_holder": "$funding_duration",
    "positive_admit": "$positive_duration",
    "duplicate_admit": "$duplicate_duration",
    "insufficient_balance": "$negative_duration",
    "total": "$total_duration"
  }
}
EOF

cat > "$REPORT" <<EOF
# Spike 09 PPE-Native Gate Benchmark

| Phase | Status | Duration |
| --- | --- | ---: |
| Wallet/sequencer health | ok | $health_duration |
| Build PPE guest | $build_status | $build_duration |
| Deploy PPE program | ok | $deploy_duration |
| Create accounts | ok | $accounts_duration |
| Fund private holder | ok | $funding_duration |
| Positive admit | ok | $positive_duration |
| Duplicate admit | $duplicate_result | $duplicate_duration |
| Insufficient-balance rejection | BA201 ThresholdMismatch | $negative_duration |
| Total | ok | $total_duration |

## Result

- \`RISC0_DEV_MODE=$RISC0_DEV_MODE\`
- Gate account: \`$GATE_ACCOUNT\`
- Presenter account: \`$PRESENTER_ACCOUNT\`
- Context nullifier: \`$NULLIFIER_HEX\`
- Nullifier count in gate data: \`$count\`
- Positive path: private holder balance \`$AMOUNT\` >= threshold \`$PASS_THRESHOLD\`
- Negative path: threshold \`$FAIL_THRESHOLD\` rejected with \`BA201 ThresholdMismatch\`

## Scope

This is a local wall-clock benchmark for the Spike 09 PPE-native candidate. It
is not a devnet/testnet CU measurement. The private holder account id and wallet
private material are intentionally omitted from this report.
EOF

cat <<REPORT

Spike 09 PPE-native gate demo passed.

Gate account:
$GATE_ACCOUNT

Presenter account:
$PRESENTER_ACCOUNT

Context nullifier:
$NULLIFIER_HEX

Report:
$REPORT

This demonstrates a Logos-native private execution path:
private holder balance >= threshold -> public gate/nullifier state update.
REPORT
