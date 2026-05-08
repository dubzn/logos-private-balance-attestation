#!/usr/bin/env bash
# Local LEZ gate E2E for LP-0005.
#
# Pipeline:
#   1. Load a public envelope + gate file from a local-sequencer proof run.
#   2. Build the deployable LEZ gate program, Spike 08 runner, and CLI.
#   3. Deploy the program to the local sequencer.
#   4. Create fresh public accounts unless provided by env.
#   5. Register presenter, initialize gate, submit host-verified admit.
#   6. Read wallet state and assert the gate account persisted the nullifier.
#
# Required:
#   A prior `scripts/demo-local-sequencer-e2e.sh` run with envelope.json/gate.json.
#
# Optional:
#   RUN_DIR or RUN                    Source run dir. Defaults to latest .demo-runs/local-sequencer/*.
#   LOGOS_LEZ_REPO or LEZ_REPO        Defaults to ../logos-execution-zone when present.
#   NSSA_WALLET_HOME_DIR              Defaults to $LOGOS_LEZ_REPO/.wallet-local.
#   DEMO_DIR                          Defaults to .demo-runs/local-gate/<timestamp>.
#   GATE_ACCOUNT                      Public/<id> or bare id. Created if unset.
#   PRESENTER_ACCOUNT                 Public/<id> or bare id. Created if unset.
#   REGISTER_ADMIN_ACCOUNT/REGISTER_ADMIN  Public/<id> or bare id. Created if unset.
#   INIT_ADMIN_ACCOUNT/INIT_ADMIN     Public/<id> or bare id. Created if unset.
#   RISC0_DEV_MODE                    Defaults to 1. Should match the source envelope mode.
#   CHECK_DUPLICATE                   Defaults to 1; submits duplicate admit and checks it does not apply.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"
export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEMO_DIR="${DEMO_DIR:-$ROOT_DIR/.demo-runs/local-gate/$TIMESTAMP}"
LOG_DIR="$DEMO_DIR/logs"
REPORT="$DEMO_DIR/report.md"
RUN_JSON="$DEMO_DIR/run.json"

PROGRAM_BIN="${PROGRAM_BIN:-$ROOT_DIR/lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin}"
RUNNER_BIN="${RUNNER_BIN:-$ROOT_DIR/spikes/spike-08-program-chaining/lez/runner/target/release/spike_08_run}"
CLI_BIN="$ROOT_DIR/target/debug/balance-attest"

POLL_SECONDS="${POLL_SECONDS:-5}"
DEPLOY_SETTLE_SECONDS="${DEPLOY_SETTLE_SECONDS:-15}"
REGISTER_WAIT_SECONDS="${REGISTER_WAIT_SECONDS:-120}"
INIT_WAIT_SECONDS="${INIT_WAIT_SECONDS:-120}"
ADMIT_WAIT_SECONDS="${ADMIT_WAIT_SECONDS:-120}"
DUPLICATE_SETTLE_SECONDS="${DUPLICATE_SETTLE_SECONDS:-45}"
CHECK_DUPLICATE="${CHECK_DUPLICATE:-1}"
DEFAULT_PROGRAM_OWNER_B58='"program_owner":"11111111111111111111111111111111"'

usage() {
  cat >&2 <<'EOF'
usage:
  RUN_DIR=.demo-runs/local-sequencer/<timestamp> scripts/demo-local-gate-e2e.sh

env:
  RUN_DIR or RUN                    Source proof run dir containing envelope.json and gate.json.
  LOGOS_LEZ_REPO or LEZ_REPO        Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR              Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.
  DEMO_DIR                          Output directory for logs/report.
  RISC0_DEV_MODE                    Defaults to 1. Use the same mode as the source envelope.
  GATE_ACCOUNT                      Optional fresh public gate account.
  PRESENTER_ACCOUNT                 Optional fresh public presenter account.
  REGISTER_ADMIN_ACCOUNT            Optional fresh public setup signer for RegisterPresenter.
  INIT_ADMIN_ACCOUNT                Optional fresh public setup signer for InitGate.
  CHECK_DUPLICATE                   Defaults to 1.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

require_logos_lez_repo "$ROOT_DIR" wallet
export_default_wallet_home
ensure_repo_local_lez_link "$ROOT_DIR"

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

json_field() {
  local file="$1"
  local path="$2"
  python3 - "$file" "$path" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    value = json.load(fh)
for key in sys.argv[2].split("."):
    value = value[key]
print(value)
PY
}

resolve_source_run_dir() {
  local source="${RUN_DIR:-${RUN:-}}"
  if [[ -z "$source" ]]; then
    source="$(find "$ROOT_DIR/.demo-runs/local-sequencer" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort | tail -n 1 || true)"
  fi
  if [[ -z "$source" ]]; then
    echo "No source proof run found. Run scripts/demo-local-sequencer-e2e.sh first or set RUN_DIR." >&2
    exit 2
  fi
  if [[ ! -d "$source" && -d "$ROOT_DIR/$source" ]]; then
    source="$ROOT_DIR/$source"
  fi
  (cd "$source" && pwd)
}

normalize_public_account() {
  local value="$1"
  value="${value#Public/}"
  printf '%s\n' "$value"
}

extract_account_id() {
  local prefix="$1"
  sed -n "s/.*account_id ${prefix}\/\([^[:space:]]*\).*/\1/p" | tail -n 1
}

extract_data_hex() {
  sed -n 's/.*"data":"\([^"]*\)".*/\1/p' | tail -n 1
}

new_public_account() {
  local label="$1"
  local out log
  log="$LOG_DIR/create-$label.log"
  out="$(wallet account new public)"
  printf '%s\n' "$out" | tee "$log" >&2
  local id
  id="$(printf '%s\n' "$out" | extract_account_id Public)"
  if [[ -z "$id" ]]; then
    echo "Could not parse public account id for $label" >&2
    exit 1
  fi
  printf '%s\n' "$id"
}

run_logged() {
  local label="$1"
  shift
  local log="$LOG_DIR/$label.log"
  {
    printf '+'
    printf ' %q' "$@"
    printf '\n\n'
  } > "$log"
  set +e
  "$@" >> "$log" 2>&1
  local status=$?
  set -e
  cat "$log"
  return "$status"
}

account_info() {
  local account_id="$1"
  wallet account get --account-id "Public/$account_id" 2>/dev/null || true
}

count_hex_occurrences() {
  local haystack="$1"
  local needle="$2"
  awk -v haystack="$haystack" -v needle="$needle" '
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

gate_nullifier_count() {
  local nullifier_hex="$1"
  local info data
  info="$(account_info "$GATE_ACCOUNT_ID")"
  data="$(printf '%s\n' "$info" | extract_data_hex)"
  count_hex_occurrences "$data" "$nullifier_hex"
}

wait_for_presenter_registered() {
  local deadline=$((SECONDS + REGISTER_WAIT_SECONDS))
  local info data
  while (( SECONDS < deadline )); do
    info="$(account_info "$PRESENTER_ACCOUNT_ID")"
    data="$(printf '%s\n' "$info" | extract_data_hex)"
    if [[ "$data" == "$PRESENTER_PUBKEY_HEX"* ]] \
       && ! printf '%s' "$info" | grep -q "$DEFAULT_PROGRAM_OWNER_B58"; then
      printf '%s\n' "$info" > "$LOG_DIR/presenter-account-final.log"
      return 0
    fi
    sleep "$POLL_SECONDS"
  done
  echo "Presenter account did not register within $REGISTER_WAIT_SECONDS seconds." >&2
  printf '%s\n' "$info" >&2
  return 1
}

wait_for_gate_initialized() {
  local deadline=$((SECONDS + INIT_WAIT_SECONDS))
  local info
  while (( SECONDS < deadline )); do
    info="$(account_info "$GATE_ACCOUNT_ID")"
    if printf '%s' "$info" | grep -q '"data":"42415431' \
       && ! printf '%s' "$info" | grep -q "$DEFAULT_PROGRAM_OWNER_B58"; then
      printf '%s\n' "$info" > "$LOG_DIR/gate-account-after-init.log"
      return 0
    fi
    sleep "$POLL_SECONDS"
  done
  echo "Gate account did not initialize within $INIT_WAIT_SECONDS seconds." >&2
  printf '%s\n' "$info" >&2
  return 1
}

wait_for_nullifier_count_at_least() {
  local nullifier_hex="$1"
  local min_count="$2"
  local deadline=$((SECONDS + ADMIT_WAIT_SECONDS))
  local count=0
  while (( SECONDS < deadline )); do
    count="$(gate_nullifier_count "$nullifier_hex")"
    if (( count >= min_count )); then
      echo "Observed nullifier $nullifier_hex count=$count"
      account_info "$GATE_ACCOUNT_ID" > "$LOG_DIR/gate-account-after-admit.log"
      return 0
    fi
    sleep "$POLL_SECONDS"
  done
  echo "Timed out waiting for nullifier count >= $min_count (last count=$count)" >&2
  return 1
}

wait_for_duplicate_to_settle() {
  local nullifier_hex="$1"
  local deadline=$((SECONDS + DUPLICATE_SETTLE_SECONDS))
  local count=0
  while (( SECONDS < deadline )); do
    count="$(gate_nullifier_count "$nullifier_hex")"
    if (( count > 1 )); then
      echo "Duplicate nullifier applied unexpectedly: count=$count" >&2
      return 1
    fi
    sleep "$POLL_SECONDS"
  done
  echo "Duplicate nullifier did not apply after settle window: count=$count"
  return 0
}

TOTAL_STARTED="$(date +%s)"
SOURCE_RUN_DIR="$(resolve_source_run_dir)"
ENVELOPE_JSON="$SOURCE_RUN_DIR/envelope.json"
GATE_JSON="$SOURCE_RUN_DIR/gate.json"

if [[ ! -f "$ENVELOPE_JSON" || ! -f "$GATE_JSON" ]]; then
  echo "Source run must contain envelope.json and gate.json: $SOURCE_RUN_DIR" >&2
  exit 2
fi

mkdir -p "$LOG_DIR"

PRESENTER_PUBKEY_HEX="$(json_field "$ENVELOPE_JSON" presenter_pubkey)"
CONTEXT_NULLIFIER_HEX="$(json_field "$ENVELOPE_JSON" journal.context_nullifier)"
PRESENTER_ID_HEX="$(json_field "$ENVELOPE_JSON" journal.presenter_id)"
CONTEXT_ID_HEX="$(json_field "$ENVELOPE_JSON" journal.context_id)"

step "1/8 Build CLI, LEZ program, and runner"
build_started="$(date +%s)"
note "source run: $SOURCE_RUN_DIR"
note "output:     $DEMO_DIR"
note "RISC0_DEV_MODE=$RISC0_DEV_MODE"
run_logged build-cli cargo build --quiet -p attestation-cli
run_logged build-program cargo build --quiet --release --manifest-path "$ROOT_DIR/lez-verifier/program/Cargo.toml"
run_logged build-runner cargo build --quiet --release --manifest-path "$ROOT_DIR/spikes/spike-08-program-chaining/lez/runner/Cargo.toml"
build_duration="$(duration "$build_started")"

if [[ ! -x "$CLI_BIN" || ! -f "$PROGRAM_BIN" || ! -x "$RUNNER_BIN" ]]; then
  echo "Expected binaries were not produced." >&2
  exit 1
fi

step "2/8 Wallet/sequencer health and program deploy"
health_started="$(date +%s)"
run_logged wallet-health wallet check-health
if ! run_logged deploy-program wallet deploy-program "$PROGRAM_BIN"; then
  note "wallet deploy-program failed. Continuing because the program may already exist; later txs will prove it."
fi
sleep "$DEPLOY_SETTLE_SECONDS"
health_duration="$(duration "$health_started")"

step "3/8 Create or load public accounts"
accounts_started="$(date +%s)"
GATE_ACCOUNT_ID="$(normalize_public_account "${GATE_ACCOUNT:-}")"
PRESENTER_ACCOUNT_ID="$(normalize_public_account "${PRESENTER_ACCOUNT:-}")"
REGISTER_ADMIN_ACCOUNT_ID="$(normalize_public_account "${REGISTER_ADMIN_ACCOUNT:-${REGISTER_ADMIN:-}}")"
INIT_ADMIN_ACCOUNT_ID="$(normalize_public_account "${INIT_ADMIN_ACCOUNT:-${INIT_ADMIN:-}}")"

if [[ -z "$GATE_ACCOUNT_ID" ]]; then
  GATE_ACCOUNT_ID="$(new_public_account gate)"
fi
if [[ -z "$PRESENTER_ACCOUNT_ID" ]]; then
  PRESENTER_ACCOUNT_ID="$(new_public_account presenter)"
fi
if [[ -z "$REGISTER_ADMIN_ACCOUNT_ID" ]]; then
  REGISTER_ADMIN_ACCOUNT_ID="$(new_public_account register-admin)"
fi
if [[ -z "$INIT_ADMIN_ACCOUNT_ID" ]]; then
  INIT_ADMIN_ACCOUNT_ID="$(new_public_account init-admin)"
fi
accounts_duration="$(duration "$accounts_started")"

cat > "$DEMO_DIR/accounts.env" <<EOF
export GATE_ACCOUNT="Public/$GATE_ACCOUNT_ID"
export PRESENTER_ACCOUNT="Public/$PRESENTER_ACCOUNT_ID"
export REGISTER_ADMIN="Public/$REGISTER_ADMIN_ACCOUNT_ID"
export INIT_ADMIN="Public/$INIT_ADMIN_ACCOUNT_ID"
EOF

cat "$DEMO_DIR/accounts.env"

COMMON_ARGS=(
  --wallet-home "$NSSA_WALLET_HOME_DIR"
  --program-bin "$PROGRAM_BIN"
  --runner-bin "$RUNNER_BIN"
  --skip-build
)

step "4/8 Dry-run gate commands"
dry_started="$(date +%s)"
"$CLI_BIN" gate-register-presenter \
  --presenter-account "Public/$PRESENTER_ACCOUNT_ID" \
  --admin-account "Public/$REGISTER_ADMIN_ACCOUNT_ID" \
  --presenter-pubkey-hex "$PRESENTER_PUBKEY_HEX" \
  "${COMMON_ARGS[@]}" \
  > "$DEMO_DIR/dry-register-presenter.json"
"$CLI_BIN" gate-init \
  --gate "$GATE_JSON" \
  --gate-account "Public/$GATE_ACCOUNT_ID" \
  --admin-account "Public/$INIT_ADMIN_ACCOUNT_ID" \
  "${COMMON_ARGS[@]}" \
  > "$DEMO_DIR/dry-init-gate.json"
"$CLI_BIN" gate-admit \
  --envelope "$ENVELOPE_JSON" \
  --gate "$GATE_JSON" \
  --gate-account "Public/$GATE_ACCOUNT_ID" \
  --presenter-account "Public/$PRESENTER_ACCOUNT_ID" \
  "${COMMON_ARGS[@]}" \
  > "$DEMO_DIR/dry-admit.json"
cat "$DEMO_DIR/dry-admit.json"
dry_duration="$(duration "$dry_started")"

step "5/8 Register presenter"
register_started="$(date +%s)"
run_logged execute-register-presenter "$CLI_BIN" gate-register-presenter \
  --presenter-account "Public/$PRESENTER_ACCOUNT_ID" \
  --admin-account "Public/$REGISTER_ADMIN_ACCOUNT_ID" \
  --presenter-pubkey-hex "$PRESENTER_PUBKEY_HEX" \
  --execute \
  "${COMMON_ARGS[@]}"
wait_for_presenter_registered
register_duration="$(duration "$register_started")"

step "6/8 Initialize gate"
init_started="$(date +%s)"
run_logged execute-init-gate "$CLI_BIN" gate-init \
  --gate "$GATE_JSON" \
  --gate-account "Public/$GATE_ACCOUNT_ID" \
  --admin-account "Public/$INIT_ADMIN_ACCOUNT_ID" \
  --execute \
  "${COMMON_ARGS[@]}"
wait_for_gate_initialized
init_duration="$(duration "$init_started")"

step "7/8 Admit host-verified envelope"
admit_started="$(date +%s)"
run_logged execute-admit "$CLI_BIN" gate-admit \
  --envelope "$ENVELOPE_JSON" \
  --gate "$GATE_JSON" \
  --gate-account "Public/$GATE_ACCOUNT_ID" \
  --presenter-account "Public/$PRESENTER_ACCOUNT_ID" \
  --execute \
  "${COMMON_ARGS[@]}"
wait_for_nullifier_count_at_least "$CONTEXT_NULLIFIER_HEX" 1
admit_duration="$(duration "$admit_started")"

duplicate_duration="00:00:00"
duplicate_status="skipped"
if [[ "$CHECK_DUPLICATE" == "1" ]]; then
  step "8/8 Duplicate admit should not append nullifier"
  duplicate_started="$(date +%s)"
  if run_logged execute-admit-duplicate "$CLI_BIN" gate-admit \
    --envelope "$ENVELOPE_JSON" \
    --gate "$GATE_JSON" \
    --gate-account "Public/$GATE_ACCOUNT_ID" \
    --presenter-account "Public/$PRESENTER_ACCOUNT_ID" \
    --execute \
    "${COMMON_ARGS[@]}"; then
    wait_for_duplicate_to_settle "$CONTEXT_NULLIFIER_HEX"
    duplicate_status="not-applied"
  else
    duplicate_status="rejected-before-submit"
  fi
  duplicate_duration="$(duration "$duplicate_started")"
fi

total_duration="$(duration "$TOTAL_STARTED")"
final_gate_account="$LOG_DIR/gate-account-final.log"
final_presenter_account="$LOG_DIR/presenter-account-final.log"
account_info "$GATE_ACCOUNT_ID" > "$final_gate_account"
account_info "$PRESENTER_ACCOUNT_ID" > "$final_presenter_account"
nullifier_count="$(gate_nullifier_count "$CONTEXT_NULLIFIER_HEX")"

cat > "$RUN_JSON" <<EOF
{
  "status": "ok",
  "risc0_dev_mode": "$RISC0_DEV_MODE",
  "source_run_dir": "$SOURCE_RUN_DIR",
  "demo_dir": "$DEMO_DIR",
  "envelope_json": "$ENVELOPE_JSON",
  "gate_json": "$GATE_JSON",
  "presenter_pubkey": "$PRESENTER_PUBKEY_HEX",
  "presenter_id": "$PRESENTER_ID_HEX",
  "context_id": "$CONTEXT_ID_HEX",
  "context_nullifier": "$CONTEXT_NULLIFIER_HEX",
  "nullifier_count": "$nullifier_count",
  "duplicate_status": "$duplicate_status",
  "accounts": {
    "gate": "Public/$GATE_ACCOUNT_ID",
    "presenter": "Public/$PRESENTER_ACCOUNT_ID",
    "register_admin": "Public/$REGISTER_ADMIN_ACCOUNT_ID",
    "init_admin": "Public/$INIT_ADMIN_ACCOUNT_ID"
  },
  "durations": {
    "build": "$build_duration",
    "health_and_deploy": "$health_duration",
    "accounts": "$accounts_duration",
    "dry_run": "$dry_duration",
    "register_presenter": "$register_duration",
    "init_gate": "$init_duration",
    "admit": "$admit_duration",
    "duplicate": "$duplicate_duration",
    "total": "$total_duration"
  }
}
EOF

cat > "$REPORT" <<EOF
# Local Gate E2E

| Step | Command | Output | Duration |
| --- | --- | --- | --- |
| Build | CLI + deployable LEZ program + Spike 08 runner | \`$LOG_DIR/build-cli.log\` | $build_duration |
| Health/deploy | \`wallet check-health\` + \`wallet deploy-program\` | \`$LOG_DIR/wallet-health.log\`, \`$LOG_DIR/deploy-program.log\` | $health_duration |
| Accounts | \`wallet account new public\` | \`$DEMO_DIR/accounts.env\` | $accounts_duration |
| Dry-run | \`gate-register-presenter\`, \`gate-init\`, \`gate-admit\` | \`$DEMO_DIR/dry-admit.json\` | $dry_duration |
| Register presenter | \`balance-attest gate-register-presenter --execute\` | \`$LOG_DIR/presenter-account-final.log\` | $register_duration |
| Init gate | \`balance-attest gate-init --execute\` | \`$LOG_DIR/gate-account-after-init.log\` | $init_duration |
| Admit | \`balance-attest gate-admit --execute\` | \`$LOG_DIR/gate-account-after-admit.log\` | $admit_duration |
| Duplicate check | second \`gate-admit --execute\` | $duplicate_status | $duplicate_duration |
| Total | - | \`$RUN_JSON\` | $total_duration |

## Public Fields

- Presenter account: \`Public/$PRESENTER_ACCOUNT_ID\`
- Gate account: \`Public/$GATE_ACCOUNT_ID\`
- Presenter id: \`$PRESENTER_ID_HEX\`
- Context id: \`$CONTEXT_ID_HEX\`
- Context nullifier: \`$CONTEXT_NULLIFIER_HEX\`
- Nullifier count in gate data: \`$nullifier_count\`

## Notes

- The proof envelope comes from \`$ENVELOPE_JSON\`.
- \`gate-admit\` performs host-side \`attestation_verifier::verify_envelope\`
  before submitting the LEZ transaction.
- The deployed LEZ program persists the admission/nullifier in
  \`account.data\`; it still does not cryptographically verify the receipt
  inside LEZ. This is the documented Workable path.
- \`RISC0_DEV_MODE=$RISC0_DEV_MODE\`.
EOF

cat "$REPORT"

printf '\n\033[1;32mLocal gate E2E complete.\033[0m Artifacts in: %s\n' "$DEMO_DIR"
if [[ "$nullifier_count" -lt 1 ]]; then
  echo "Expected nullifier was not found in gate account data." >&2
  exit 1
fi
