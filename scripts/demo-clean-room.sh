#!/usr/bin/env bash
# Clean-room evaluator runner for LP-0005.
#
# Default path is wallet-free:
#   - public hygiene checks
#   - quick proof/verify demo
#   - local Messaging/off-chain demo
#
# Optional live paths:
#   --with-live   runs wallet preflight, private-account preparation, and
#                 the full local sequencer + Workable gate E2E.
#   --with-ppe    runs wallet preflight and the Spike 09 PPE-native gate demo.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
CLEAN_ROOM_DIR="${CLEAN_ROOM_DIR:-$ROOT_DIR/.demo-runs/clean-room/$TIMESTAMP}"
LOG_DIR="$CLEAN_ROOM_DIR/logs"
REPORT="$CLEAN_ROOM_DIR/report.md"
RUN_JSON="$CLEAN_ROOM_DIR/run.json"

RISC0_MODE="${RISC0_DEV_MODE:-1}"
DEMO_MODE_FLAG="--dev-mode"
WITH_TESTS=0
WITH_LEZ=0
WITH_LIVE=0
WITH_PPE=0

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/demo-clean-room.sh [--dev-mode|--real-prover] [--with-tests] [--with-lez] [--with-live] [--with-ppe]

default:
  Runs public hygiene, quick proof/verify, and local Messaging demo.

options:
  --dev-mode      Set RISC0_DEV_MODE=1. Fast, non-production receipts.
  --real-prover   Set RISC0_DEV_MODE=0. Slower, cryptographic receipts.
  --with-tests    Include cargo fmt/test in the public hygiene check.
  --with-lez      Include LEZ compatibility checks.
  --with-live     Also run prepare-local-private-account + full local E2E.
                  Requires PRIVATE_ACCOUNT=Private/<id>.
  --with-ppe      Also run Spike 09 PPE-native gate demo.
                  Requires local sequencer + wallet home.

env:
  CLEAN_ROOM_DIR             Output directory. Defaults to .demo-runs/clean-room/<timestamp>.
  PRIVATE_ACCOUNT            Required for --with-live.
  THRESHOLD                  Optional threshold, defaults to 1 in downstream scripts.
  LOGOS_LEZ_REPO             Required by --with-lez/live/ppe unless ../logos-execution-zone exists.
  NSSA_WALLET_HOME_DIR       Optional wallet home.

examples:
  scripts/demo-clean-room.sh
  scripts/demo-clean-room.sh --real-prover
  PRIVATE_ACCOUNT=Private/<id> scripts/demo-clean-room.sh --real-prover --with-lez --with-live --with-ppe
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev-mode)
      RISC0_MODE="1"
      DEMO_MODE_FLAG="--dev-mode"
      shift
      ;;
    --real-prover)
      RISC0_MODE="0"
      DEMO_MODE_FLAG="--real-prover"
      shift
      ;;
    --with-tests)
      WITH_TESTS=1
      shift
      ;;
    --with-lez)
      WITH_LEZ=1
      shift
      ;;
    --with-live)
      WITH_LIVE=1
      shift
      ;;
    --with-ppe)
      WITH_PPE=1
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

export RISC0_DEV_MODE="$RISC0_MODE"
mkdir -p "$LOG_DIR"

duration() {
  local start="$1"
  local end
  end="$(date +%s)"
  printf "%02d:%02d:%02d" $(((end - start) / 3600)) $((((end - start) % 3600) / 60)) $(((end - start) % 60))
}

escape_cell() {
  local value="$1"
  value="${value//$'\n'/ }"
  value="${value//|/\\|}"
  printf '%s' "$value"
}

sanitize_sensitive_output() {
  sed -E \
    -e 's#Private/[1-9A-HJ-NP-Za-km-z]{20,}#Private/<redacted>#g' \
    -e 's/(With npk )[0-9a-f]+/\1<redacted>/g' \
    -e 's/(With vpk )[0-9a-f]+/\1<redacted>/g'
}

write_header() {
  {
    printf '# Clean-Room Demo Run\n\n'
    printf -- '- Date: `%s`\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf -- '- RISC0_DEV_MODE: `%s`\n' "$RISC0_MODE"
    printf -- '- Output: `%s`\n\n' "$CLEAN_ROOM_DIR"
    printf '| Step | Command | Status | Output | Duration |\n'
    printf '| --- | --- | --- | --- | --- |\n'
  } > "$REPORT"
}

append_row() {
  local step="$1"
  local command="$2"
  local status="$3"
  local output="$4"
  local elapsed="$5"

  printf '| %s | `%s` | %s | %s | %s |\n' \
    "$(escape_cell "$step")" \
    "$(escape_cell "$command")" \
    "$(escape_cell "$status")" \
    "$(escape_cell "$output")" \
    "$(escape_cell "$elapsed")" >> "$REPORT"
}

run_step() {
  local step="$1"
  local display_command="$2"
  local command="$3"
  local log="$LOG_DIR/${step// /_}.log"
  local started elapsed status

  printf '\n\033[1;36m== %s ==\033[0m\n' "$step"
  started="$(date +%s)"
  set +e
  (cd "$ROOT_DIR" && eval "$command") 2>&1 | sanitize_sensitive_output | tee "$log"
  status=${PIPESTATUS[0]}
  set -e
  elapsed="$(duration "$started")"

  if [[ "$status" -eq 0 ]]; then
    append_row "$step" "$display_command" "ok" "\`$log\`" "$elapsed"
  else
    append_row "$step" "$display_command" "fail" "\`$log\`" "$elapsed"
    echo "Clean-room step failed: $step" >&2
    echo "Log: $log" >&2
    exit "$status"
  fi
}

skip_step() {
  local step="$1"
  local reason="$2"
  printf '\n\033[1;33m== %s skipped ==\033[0m\n%s\n' "$step" "$reason"
  append_row "$step" "-" "skipped" "$reason" "00:00:00"
}

clean_room_flags=()
if [[ "$WITH_TESTS" == "1" ]]; then
  clean_room_flags+=(--with-tests)
fi
if [[ "$WITH_LEZ" == "1" ]]; then
  clean_room_flags+=(--with-lez)
fi
clean_room_command="ALLOW_DIRTY=1 scripts/check-public-clean-room.sh"
if [[ "${#clean_room_flags[@]}" -gt 0 ]]; then
  clean_room_command+=" ${clean_room_flags[*]}"
fi

TOTAL_STARTED="$(date +%s)"
write_header

run_step "public hygiene" "$clean_room_command" "$clean_room_command"

run_step "quick proof verify" \
  "DEMO_DIR=<clean-room>/quick ./demo.sh --quick $DEMO_MODE_FLAG" \
  "DEMO_DIR=\"$CLEAN_ROOM_DIR/quick\" ./demo.sh --quick \"$DEMO_MODE_FLAG\""

run_step "local messaging" \
  "DEMO_DIR=<clean-room>/messaging ./demo.sh --messaging $DEMO_MODE_FLAG" \
  "DEMO_DIR=\"$CLEAN_ROOM_DIR/messaging\" ./demo.sh --messaging \"$DEMO_MODE_FLAG\""

if [[ "$WITH_LIVE" == "1" ]]; then
  if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
    echo "--with-live requires PRIVATE_ACCOUNT=Private/<id>" >&2
    exit 2
  fi
  run_step "wallet preflight" \
    "scripts/check-wallet-preflight.sh" \
    "scripts/check-wallet-preflight.sh"
  run_step "prepare private account" \
    "PRIVATE_ACCOUNT=Private/<redacted> scripts/prepare-local-private-account.sh" \
    "PRIVATE_ACCOUNT=\"$PRIVATE_ACCOUNT\" THRESHOLD=\"${THRESHOLD:-1}\" scripts/prepare-local-private-account.sh"
  run_step "full local e2e" \
    "PRIVATE_ACCOUNT=Private/<redacted> FULL_DEMO_DIR=<clean-room>/full ./demo.sh --full $DEMO_MODE_FLAG" \
    "PRIVATE_ACCOUNT=\"$PRIVATE_ACCOUNT\" THRESHOLD=\"${THRESHOLD:-1}\" FULL_DEMO_DIR=\"$CLEAN_ROOM_DIR/full\" ./demo.sh --full \"$DEMO_MODE_FLAG\""
else
  skip_step "full local e2e" "not requested; rerun with --with-live and PRIVATE_ACCOUNT=Private/<id>"
fi

if [[ "$WITH_PPE" == "1" ]]; then
  run_step "wallet preflight for ppe" \
    "scripts/check-wallet-preflight.sh" \
    "scripts/check-wallet-preflight.sh"
  run_step "ppe native gate" \
    "DEMO_DIR=<clean-room>/ppe-gate ./demo.sh --ppe-gate $DEMO_MODE_FLAG" \
    "DEMO_DIR=\"$CLEAN_ROOM_DIR/ppe-gate\" ./demo.sh --ppe-gate \"$DEMO_MODE_FLAG\""
else
  skip_step "ppe native gate" "not requested; rerun with --with-ppe"
fi

total_duration="$(duration "$TOTAL_STARTED")"
append_row "total" "-" "ok" "\`$RUN_JSON\`" "$total_duration"

cat > "$RUN_JSON" <<EOF
{
  "status": "ok",
  "risc0_dev_mode": "$RISC0_MODE",
  "clean_room_dir": "$CLEAN_ROOM_DIR",
  "with_tests": $WITH_TESTS,
  "with_lez": $WITH_LEZ,
  "with_live": $WITH_LIVE,
  "with_ppe": $WITH_PPE,
  "report": "$REPORT",
  "duration": "$total_duration"
}
EOF

cat >> "$REPORT" <<EOF

## Result

- Status: \`ok\`
- Total duration: \`$total_duration\`
- \`RISC0_DEV_MODE=$RISC0_MODE\`

## Privacy Note

Live runs may create private witness artifacts under the clean-room output
directory. Do not publish \`witness.json\`, wallet homes, or raw logs without
reviewing them first. This runner redacts private account ids and obvious
private key material from terminal/log output, but the generated witness files
remain private.
EOF

cat "$REPORT"
printf '\n\033[1;32mClean-room demo run complete.\033[0m Artifacts in: %s\n' "$CLEAN_ROOM_DIR"
