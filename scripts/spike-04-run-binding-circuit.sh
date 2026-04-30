#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
PROGRAM_BIN="$LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/binding_attestation_spike.bin"
MODE="${1:-fixture-all}"
THRESHOLD_VALUE="${THRESHOLD:-1}"

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEZ_REPO/.wallet-local}"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  "$ROOT_DIR/scripts/spike-04-build-binding-circuit.sh"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing binding_attestation_spike binary: $PROGRAM_BIN" >&2
  exit 1
fi

cd "$LEZ_REPO/examples/program_deployment"

run_mode() {
  local mode="$1"
  shift || true
  cargo run --bin prove_binding_attestation_spike -- "$PROGRAM_BIN" "$mode" "$@"
}

case "$MODE" in
  fixture-all)
    echo "[1/6] fixture-valid"
    run_mode fixture-valid
    echo "[2/6] fixture-below-threshold"
    run_mode fixture-below-threshold
    echo "[3/6] fixture-bad-root"
    run_mode fixture-bad-root
    echo "[4/6] fixture-bad-presenter"
    run_mode fixture-bad-presenter
    echo "[5/6] fixture-bad-nullifier"
    run_mode fixture-bad-nullifier
    echo "[6/6] fixture-context-variant"
    run_mode fixture-context-variant
    ;;
  live|live-below-threshold)
    if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
      echo "Set PRIVATE_ACCOUNT to an initialized private account id without Private/." >&2
      exit 1
    fi
    run_mode "$MODE" "$PRIVATE_ACCOUNT" "$THRESHOLD_VALUE"
    ;;
  fixture-valid|fixture-below-threshold|fixture-bad-root|fixture-bad-presenter|fixture-bad-nullifier|fixture-context-variant)
    run_mode "$MODE"
    ;;
  *)
    echo "Unknown mode: $MODE" >&2
    echo "Valid modes: fixture-all, fixture-valid, fixture-below-threshold, fixture-bad-root, fixture-bad-presenter, fixture-bad-nullifier, fixture-context-variant, live, live-below-threshold" >&2
    exit 2
    ;;
esac
