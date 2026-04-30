#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"

"$ROOT_DIR/scripts/check-risc0-version.sh"
"$ROOT_DIR/scripts/spike-00-install-lez-sources.sh"

cd "$LEZ_REPO"
export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"

echo "[1/1] Building receipt_gate.bin through LEZ program_deployment guest build"
cargo risczero build --manifest-path examples/program_deployment/methods/guest/Cargo.toml

echo "Built receipt_gate.bin under:"
echo "$LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/receipt_gate.bin"

