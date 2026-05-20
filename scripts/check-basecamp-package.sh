#!/usr/bin/env bash
# Build and inspect the Basecamp ui_qml package.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASECAMP_DIR="$ROOT_DIR/apps/basecamp"
SKIP_BUILD=0

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/check-basecamp-package.sh [--skip-build]

options:
  --skip-build   Inspect apps/basecamp/result without running nix build.

checks:
  - nix build .#install
  - result/plugins/balance_attestation exists
  - manifest/metadata identify a ui_qml module
  - QML view exists
  - plugin and replica factory shared libraries exist
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

step() {
  printf '\n\033[1;36m== %s ==\033[0m\n' "$1"
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

require_file() {
  [[ -f "$1" || -L "$1" ]] || fail "missing $1"
}

require_glob() {
  local pattern="$1"
  compgen -G "$pattern" >/dev/null || fail "missing file matching $pattern"
}

require_grep() {
  local pattern="$1"
  local file="$2"
  grep -qE "$pattern" "$file" || fail "$file does not match $pattern"
}

if [[ "$SKIP_BUILD" != "1" ]]; then
  command -v nix >/dev/null || fail "nix is required to build the Basecamp package"
  step "1/3 Build Basecamp package"
  (cd "$BASECAMP_DIR" && nix build .#install)
else
  step "1/3 Build Basecamp package"
  echo "Skipped by --skip-build."
fi

PLUGIN_DIR="$BASECAMP_DIR/result/plugins/balance_attestation"
MANIFEST="$PLUGIN_DIR/manifest.json"
METADATA="$PLUGIN_DIR/metadata.json"
VIEW="$PLUGIN_DIR/src/qml/BalanceAttestation.qml"

step "2/3 Inspect install tree"
[[ -d "$PLUGIN_DIR" ]] || fail "missing $PLUGIN_DIR"
require_file "$MANIFEST"
require_file "$METADATA"
require_file "$VIEW"
require_file "$PLUGIN_DIR/variant"
require_glob "$PLUGIN_DIR/balance_attestation_plugin."*
require_glob "$PLUGIN_DIR/balance_attestation_replica_factory."*

step "3/3 Validate metadata"
require_grep '"name"[[:space:]]*:[[:space:]]*"balance_attestation"' "$MANIFEST"
require_grep '"type"[[:space:]]*:[[:space:]]*"ui_qml"' "$MANIFEST"
require_grep '"view"[[:space:]]*:[[:space:]]*"src/qml/BalanceAttestation.qml"' "$MANIFEST"
require_grep '"name"[[:space:]]*:[[:space:]]*"balance_attestation"' "$METADATA"
require_grep '"type"[[:space:]]*:[[:space:]]*"ui_qml"' "$METADATA"
require_grep '"view"[[:space:]]*:[[:space:]]*"src/qml/BalanceAttestation.qml"' "$METADATA"

printf '\nBasecamp package check passed.\n'
printf 'Plugin dir: %s\n' "$PLUGIN_DIR"
