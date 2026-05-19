#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

require_logos_lez_repo "$ROOT_DIR" examples/program_deployment/Cargo.toml examples/program_deployment/methods/guest/Cargo.toml
PROGRAM_DIR="$LOGOS_LEZ_REPO/examples/program_deployment"

mkdir -p "$PROGRAM_DIR/methods/guest/src/bin"
mkdir -p "$PROGRAM_DIR/src/bin"

cp "$ROOT_DIR/spikes/spike-09-ppe-gate/lez/guest/src/bin/ppe_balance_gate.rs" \
  "$PROGRAM_DIR/methods/guest/src/bin/ppe_balance_gate.rs"

cp "$ROOT_DIR/spikes/spike-09-ppe-gate/lez/runner/src/bin/run_ppe_balance_gate.rs" \
  "$PROGRAM_DIR/src/bin/run_ppe_balance_gate.rs"

ensure_dep() {
  local file="$1"
  local dep="$2"
  if ! grep -q "^${dep}\\.workspace = true" "$file"; then
    printf '\n%s.workspace = true\n' "$dep" >> "$file"
  fi
}

ensure_dep "$PROGRAM_DIR/Cargo.toml" "anyhow"
ensure_dep "$PROGRAM_DIR/Cargo.toml" "serde"
ensure_dep "$PROGRAM_DIR/Cargo.toml" "hex"
ensure_dep "$PROGRAM_DIR/Cargo.toml" "sha2"
ensure_dep "$PROGRAM_DIR/methods/guest/Cargo.toml" "borsh"
ensure_dep "$PROGRAM_DIR/methods/guest/Cargo.toml" "serde"

echo "Installed Spike 09 PPE gate sources into $PROGRAM_DIR"
