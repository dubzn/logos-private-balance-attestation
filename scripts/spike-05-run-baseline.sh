#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 3 ]]; then
  echo "usage: spike-05-run-baseline.sh <label> <risc0-dev-mode> <output-md>" >&2
  exit 2
fi

LABEL="$1"
RISC0_MODE="$2"
OUTPUT_MD="$3"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
mkdir -p "$(dirname "$OUTPUT_MD")"
RESULTS_DIR="$(cd "$(dirname "$OUTPUT_MD")" && pwd)"
OUTPUT_MD="$RESULTS_DIR/$(basename "$OUTPUT_MD")"
LOG_DIR="$RESULTS_DIR/logs/$LABEL-$(date +%Y%m%d-%H%M%S)"
START_TOTAL="$(date +%s)"

mkdir -p "$LOG_DIR"

format_duration() {
  local seconds="$1"
  local hours=$((seconds / 3600))
  local minutes=$(((seconds % 3600) / 60))
  local secs=$((seconds % 60))
  printf '%02d:%02d:%02d' "$hours" "$minutes" "$secs"
}

escape_cell() {
  local value="$1"
  value="${value//$'\n'/ }"
  value="${value//|/\\|}"
  value="${value//\`/\\\`}"
  printf '%s' "$value"
}

summarize_log() {
  local log_file="$1"
  local summary
  summary="$(tail -n 10 "$log_file" | tr '\n' ' ' | sed -E 's/[[:space:]]+/ /g')"
  if [[ "${#summary}" -gt 260 ]]; then
    summary="${summary:0:257}..."
  fi
  printf 'log: %s; %s' "${log_file#$ROOT_DIR/}" "$summary"
}

write_header() {
  {
    printf '# Spike 05 %s Baseline\n\n' "$LABEL"
    printf -- '- Date: `%s`\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf -- '- RISC0_DEV_MODE: `%s`\n' "$RISC0_MODE"
    printf -- '- LEZ_REPO: `%s`\n' "$LEZ_REPO"
    printf -- '- Logs: `%s`\n\n' "${LOG_DIR#$ROOT_DIR/}"
    printf '| Step | Command | Status | Output | Duration |\n'
    printf '| --- | --- | --- | --- | --- |\n'
  } > "$OUTPUT_MD"
}

append_row() {
  local step="$1"
  local command="$2"
  local status="$3"
  local output="$4"
  local duration="$5"
  printf '| %s | `%s` | %s | %s | %s |\n' \
    "$(escape_cell "$step")" \
    "$(escape_cell "$command")" \
    "$(escape_cell "$status")" \
    "$(escape_cell "$output")" \
    "$(escape_cell "$duration")" >> "$OUTPUT_MD"
}

finish_total() {
  local status="$1"
  local output="$2"
  local now elapsed
  now="$(date +%s)"
  elapsed=$((now - START_TOTAL))
  append_row "total" "-" "$status" "$output" "$(format_duration "$elapsed")"
}

run_step() {
  local step="$1"
  local command="$2"
  local log_file="$LOG_DIR/${step// /_}.log"
  local started elapsed status summary

  printf '[%s] %s\n' "$LABEL" "$step"
  started="$(date +%s)"
  if (cd "$ROOT_DIR" && eval "$command") > "$log_file" 2>&1; then
    status="ok"
  else
    status="fail"
  fi
  local now
  now="$(date +%s)"
  elapsed=$((now - started))
  summary="$(summarize_log "$log_file")"
  append_row "$step" "$command" "$status" "$summary" "$(format_duration "$elapsed")"

  if [[ "$status" != "ok" ]]; then
    finish_total "fail" "failed at $step; result file: ${OUTPUT_MD#$ROOT_DIR/}"
    cat "$OUTPUT_MD"
    exit 1
  fi
}

case "$RISC0_MODE" in
  0|1) ;;
  *)
    echo "risc0-dev-mode must be 0 or 1, got: $RISC0_MODE" >&2
    exit 2
    ;;
esac

write_header

run_step "check-risc0-version" \
  "LEZ_REPO=\"$LEZ_REPO\" ./scripts/check-risc0-version.sh"

PROGRAM_BIN="$LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/binding_attestation_spike.bin"
if [[ "${SPIKE05_SKIP_BUILD:-0}" == "1" ]]; then
  if [[ ! -f "$PROGRAM_BIN" ]]; then
    append_row "build-binding-circuit" "SPIKE05_SKIP_BUILD=1" "fail" \
      "missing existing ELF: $PROGRAM_BIN" "00:00:00"
    finish_total "fail" "missing existing ELF for skipped build"
    cat "$OUTPUT_MD"
    exit 1
  fi
  append_row "build-binding-circuit" "SPIKE05_SKIP_BUILD=1" "skipped" \
    "using existing ELF: $PROGRAM_BIN" "00:00:00"
else
  run_step "build-binding-circuit" \
    "LEZ_REPO=\"$LEZ_REPO\" RISC0_DEV_MODE=\"$RISC0_MODE\" ./scripts/spike-04-build-binding-circuit.sh"
fi

run_step "prove-fixture-valid" \
  "LEZ_REPO=\"$LEZ_REPO\" RISC0_DEV_MODE=\"$RISC0_MODE\" SKIP_BUILD=1 ./scripts/spike-04-run-binding-circuit.sh fixture-valid"

if [[ "${SPIKE05_INCLUDE_CONTEXT_VARIANT:-0}" == "1" ]]; then
  run_step "prove-context-variant" \
    "LEZ_REPO=\"$LEZ_REPO\" RISC0_DEV_MODE=\"$RISC0_MODE\" SKIP_BUILD=1 ./scripts/spike-04-run-binding-circuit.sh fixture-context-variant"
fi

if [[ "${SPIKE05_RUN_LIVE:-0}" == "1" ]]; then
  if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
    finish_total "fail" "SPIKE05_RUN_LIVE=1 requires PRIVATE_ACCOUNT"
    cat "$OUTPUT_MD"
    exit 1
  fi
  run_step "prove-live" \
    "LEZ_REPO=\"$LEZ_REPO\" RISC0_DEV_MODE=\"$RISC0_MODE\" SKIP_BUILD=1 PRIVATE_ACCOUNT=\"$PRIVATE_ACCOUNT\" THRESHOLD=\"${THRESHOLD:-25}\" ./scripts/spike-04-run-binding-circuit.sh live"
fi

finish_total "ok" "result file: ${OUTPUT_MD#$ROOT_DIR/}"
cat "$OUTPUT_MD"
