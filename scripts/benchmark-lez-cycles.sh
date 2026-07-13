#!/usr/bin/env bash
# Measure deterministic RISC Zero user cycles for the deployable LP-0005 LEZ
# gate program. The metric and execution shape follow upstream
# logos-execution-zone/tools/cycle_bench.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.demo-runs/cycle-bench/$TIMESTAMP}"
ITERATIONS="${ITERATIONS:-10}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<'EOF'
usage: scripts/benchmark-lez-cycles.sh

env:
  LOGOS_LEZ_REPO   Compatible latest logos-execution-zone checkout.
  ITERATIONS       Timed samples per operation after one warmup. Default: 10.
  OUT_DIR          Output directory for benchmark.json and report.md.
EOF
  exit 0
fi

if ! [[ "$ITERATIONS" =~ ^[1-9][0-9]*$ ]]; then
  echo "ITERATIONS must be a positive integer." >&2
  exit 2
fi

require_logos_lez_repo "$ROOT_DIR" Cargo.toml lee/state_machine/core
ensure_repo_local_lez_link "$ROOT_DIR"

LEZ_REF="$(git -C "$LOGOS_LEZ_REPO" rev-parse HEAD)"
GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
JSON_OUT="$OUT_DIR/benchmark.json"
MARKDOWN_OUT="$OUT_DIR/report.md"
RUN_LOG="$OUT_DIR/run.log"

mkdir -p "$OUT_DIR"

echo "LP-0005 LEZ cycle benchmark"
echo "  LOGOS_LEZ_REPO=$LOGOS_LEZ_REPO"
echo "  LEZ_REF=$LEZ_REF"
echo "  ITERATIONS=$ITERATIONS"
echo "  OUT_DIR=$OUT_DIR"

if cargo run --release \
  --manifest-path "$ROOT_DIR/lez-verifier/program/Cargo.toml" \
  --example cycle_bench -- \
  --iterations "$ITERATIONS" \
  --json-out "$JSON_OUT" \
  --markdown-out "$MARKDOWN_OUT" \
  --lez-ref "$LEZ_REF" \
  --generated-at "$GENERATED_AT" \
  > "$RUN_LOG" 2>&1; then
  cat "$MARKDOWN_OUT"
else
  status=$?
  echo "Cycle benchmark failed. Last 120 log lines:" >&2
  tail -120 "$RUN_LOG" >&2 || true
  exit "$status"
fi

echo
echo "Benchmark artifacts:"
echo "  $JSON_OUT"
echo "  $MARKDOWN_OUT"
echo "  $RUN_LOG"
