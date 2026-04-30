#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
PROGRAM_DIR="$LEZ_REPO/examples/program_deployment"

if [[ ! -d "$PROGRAM_DIR" ]]; then
  echo "Missing LEZ program_deployment dir: $PROGRAM_DIR" >&2
  exit 1
fi

mkdir -p "$PROGRAM_DIR/methods/guest/src/bin"
mkdir -p "$PROGRAM_DIR/src/bin"

cp "$ROOT_DIR/spikes/balance-circuit/lez/guest/src/bin/balance_attestation_spike.rs" \
  "$PROGRAM_DIR/methods/guest/src/bin/balance_attestation_spike.rs"

cp "$ROOT_DIR/spikes/balance-circuit/lez/runner/src/bin/prove_balance_attestation_spike.rs" \
  "$PROGRAM_DIR/src/bin/prove_balance_attestation_spike.rs"

ensure_dep() {
  local file="$1"
  local dep="$2"
  if ! grep -q "^${dep}\\.workspace = true" "$file"; then
    printf '\n%s.workspace = true\n' "$dep" >> "$file"
  fi
}

ensure_dep "$PROGRAM_DIR/Cargo.toml" "serde"
ensure_dep "$PROGRAM_DIR/Cargo.toml" "serde_json"
ensure_dep "$PROGRAM_DIR/Cargo.toml" "hex"
ensure_dep "$PROGRAM_DIR/Cargo.toml" "risc0-zkvm"
ensure_dep "$PROGRAM_DIR/methods/guest/Cargo.toml" "serde"

echo "Installed balance circuit spike sources into $PROGRAM_DIR"

