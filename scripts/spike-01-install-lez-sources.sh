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

cp "$ROOT_DIR/spikes/private-balance-gate/lez/guest/src/bin/private_balance_gate.rs" \
  "$PROGRAM_DIR/methods/guest/src/bin/private_balance_gate.rs"

cp "$ROOT_DIR/spikes/private-balance-gate/lez/runner/src/bin/run_private_balance_gate.rs" \
  "$PROGRAM_DIR/src/bin/run_private_balance_gate.rs"

ensure_dep() {
  local file="$1"
  local dep="$2"
  if ! grep -q "^${dep}\\.workspace = true" "$file"; then
    printf '\n%s.workspace = true\n' "$dep" >> "$file"
  fi
}

ensure_dep "$PROGRAM_DIR/Cargo.toml" "serde"
ensure_dep "$PROGRAM_DIR/methods/guest/Cargo.toml" "serde"

echo "Installed private balance gate spike sources into $PROGRAM_DIR"

