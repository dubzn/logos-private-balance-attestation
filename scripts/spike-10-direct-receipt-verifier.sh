#!/usr/bin/env bash
# Compile or run the explicit in-guest RISC Zero receipt verifier spike.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPIKE_DIR="$ROOT_DIR/spikes/spike-10-direct-receipt-verifier"
MODE="${1:-compile-only}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.spike-results/spike-10-direct-receipt-verifier/$TIMESTAMP}"

case "$MODE" in
  compile-only)
    ;;
  real-succinct|real-groth16)
    if [[ "${RISC0_DEV_MODE:-}" != "0" ]]; then
      echo "real-succinct requires RISC0_DEV_MODE=0" >&2
      exit 2
    fi
    ;;
  *)
    echo "usage: scripts/spike-10-direct-receipt-verifier.sh [compile-only|real-succinct|real-groth16]" >&2
    exit 2
    ;;
esac

mkdir -p "$OUT_DIR"

echo "Spike 10: explicit in-guest receipt verification"
echo "  mode=$MODE"
echo "  RISC0_DEV_MODE=${RISC0_DEV_MODE:-unset}"
echo "  OUT_DIR=$OUT_DIR"

OUT_DIR="$OUT_DIR" cargo run --release \
  --manifest-path "$SPIKE_DIR/Cargo.toml" \
  --bin run_spike -- "$MODE" \
  2>&1 | tee "$OUT_DIR/run.log"

echo "Spike artifacts: $OUT_DIR"
