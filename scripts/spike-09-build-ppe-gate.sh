#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

require_logos_lez_repo "$ROOT_DIR" Cargo.toml examples/program_deployment/methods/guest/Cargo.toml
export_default_wallet_home
export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"

"$ROOT_DIR/scripts/check-risc0-version.sh"
"$ROOT_DIR/scripts/spike-09-install-lez-sources.sh"

cd "$LOGOS_LEZ_REPO"

echo "[1/1] Building ppe_balance_gate.bin through LEZ program_deployment guest build"
cargo risczero build --manifest-path examples/program_deployment/methods/guest/Cargo.toml

echo "Built ppe_balance_gate.bin under:"
echo "$LOGOS_LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/ppe_balance_gate.bin"
