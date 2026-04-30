#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"

if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
  echo "Set PRIVATE_ACCOUNT to an initialized local private account id without Private/." >&2
  exit 1
fi

export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEZ_REPO/.wallet-local}"

if [[ "${SKIP_INSTALL:-0}" != "1" ]]; then
  "$ROOT_DIR/scripts/spike-02-install-lez-sources.sh"
fi

echo "[1/2] Checking wallet/sequencer health"
wallet check-health

echo "[2/2] Inspecting private commitment membership proof"
cd "$LEZ_REPO/examples/program_deployment"

cargo run --bin inspect_private_commitment -- \
  "$PRIVATE_ACCOUNT" \
  --require-proof

