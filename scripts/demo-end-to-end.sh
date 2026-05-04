#!/usr/bin/env bash
# End-to-end demo for LP-0005.
#
# Pipeline:
#   1. Build the workspace (binaries: balance-attest, build-demo-fixtures).
#   2. Generate a fresh witness + gate JSON pair from a deterministic seed.
#   3. balance-attest prove --witness witness.json --out envelope.json
#   4. balance-attest verify --envelope envelope.json --gate gate.json
#
# By default this runs in RISC0_DEV_MODE=1 (fast, non-production proofs).
# Set RISC0_DEV_MODE=0 to record a real-prover demo for the LP-0005 video —
# in that mode the script expects the user to have a working risc0 prover
# environment on the host (rzup-installed toolchain, sufficient RAM).
#
# Usage:
#   scripts/demo-end-to-end.sh             # dev mode by default
#   RISC0_DEV_MODE=0 scripts/demo-end-to-end.sh
#   DEMO_DIR=/tmp/myrun scripts/demo-end-to-end.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEMO_DIR="${DEMO_DIR:-$ROOT_DIR/.demo-runs/$(date -u +%Y%m%dT%H%M%SZ)}"
DEV_MODE="${RISC0_DEV_MODE:-1}"

mkdir -p "$DEMO_DIR"

step() {
    printf '\n\033[1;36m== %s ==\033[0m\n' "$1"
}

note() {
    printf '   %s\n' "$1"
}

step "1/4 Build workspace"
note "Output: $DEMO_DIR"
note "RISC0_DEV_MODE=$DEV_MODE"
cargo build --quiet -p attestation-cli -p demo-runner

step "2/4 Build witness + gate fixtures"
RUSTFLAGS="" cargo run --quiet -p demo-runner --bin build-demo-fixtures -- "$DEMO_DIR"
note "wrote $DEMO_DIR/witness.json (private — keep secret)"
note "wrote $DEMO_DIR/gate.json   (public — share with the verifier)"

step "3/4 Prove balance attestation"
RISC0_DEV_MODE="$DEV_MODE" cargo run --quiet -p attestation-cli -- \
    prove \
    --witness "$DEMO_DIR/witness.json" \
    --out "$DEMO_DIR/envelope.json" 2> >(grep -v "^WARNING:" >&2 || true)
ENV_BYTES="$(wc -c <"$DEMO_DIR/envelope.json" | tr -d ' ')"
note "wrote $DEMO_DIR/envelope.json (${ENV_BYTES} bytes)"

step "4/4 Verify envelope (off-chain path)"
VERIFY_OUTPUT="$(
    RISC0_DEV_MODE="$DEV_MODE" cargo run --quiet -p attestation-cli -- \
        verify \
        --envelope "$DEMO_DIR/envelope.json" \
        --gate "$DEMO_DIR/gate.json"
)"
echo "$VERIFY_OUTPUT" | python3 -m json.tool 2>/dev/null || echo "$VERIFY_OUTPUT"

printf '\n\033[1;32mDemo complete.\033[0m Artifacts in: %s\n' "$DEMO_DIR"
if [[ "$DEV_MODE" != "0" ]]; then
    printf '\033[1;33mNote:\033[0m RISC0_DEV_MODE=%s — receipts are NOT cryptographically valid.\n' "$DEV_MODE"
    printf '       Re-run with RISC0_DEV_MODE=0 for a real proof (slower, production-grade).\n'
fi
