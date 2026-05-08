#!/usr/bin/env bash
# Fail-fast wallet/sequencer preflight for local LP-0005 demos.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.demo-runs/preflight/$TIMESTAMP}"
HEALTH_LOG="$OUT_DIR/wallet-health.log"

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/check-wallet-preflight.sh

env:
  LOGOS_LEZ_REPO or LEZ_REPO       Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR            Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.
  OUT_DIR                         Output directory for wallet-health.log.

This check is intentionally non-interactive. If wallet setup/password input is
needed, it prints the exact setup commands instead of hanging.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

require_logos_lez_repo "$ROOT_DIR" wallet nssa/core
export_default_wallet_home

mkdir -p "$OUT_DIR"

printf 'LOGOS_LEZ_REPO=%s\n' "$LOGOS_LEZ_REPO"
printf 'NSSA_WALLET_HOME_DIR=%s\n' "$NSSA_WALLET_HOME_DIR"
printf 'wallet health log=%s\n' "$HEALTH_LOG"

require_wallet_health "$HEALTH_LOG"

printf 'Wallet/sequencer preflight ok.\n'
