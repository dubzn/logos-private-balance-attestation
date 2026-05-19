#!/usr/bin/env bash
# Lambda Prize entrypoint for LP-0005 demos.
#
# Modes:
#   --quick       Synthetic fixture: prove + verify, no wallet/sequencer.
#   --messaging   Synthetic fixture: proof message + local admission.
#   --full        Live local wallet/sequencer + Workable LEZ gate path.
#
# Proving mode:
#   --dev-mode    RISC0_DEV_MODE=1 (fast, non-production receipts).
#   --real-prover RISC0_DEV_MODE=0 (slower, cryptographic receipts).
#
# Default mode is --full to preserve the original submission entrypoint.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="full"
RISC0_MODE="${RISC0_DEV_MODE:-1}"

usage() {
  cat >&2 <<'EOF'
usage:
  ./demo.sh [--quick|--messaging|--full] [--dev-mode|--real-prover]

modes:
  --quick        Run scripts/demo-end-to-end.sh with deterministic fixtures.
                 No wallet, sequencer, or private account is required.

  --messaging    Run scripts/demo-local-messaging.sh with deterministic
                 fixtures. Demonstrates export, receive, verify, admit, and
                 duplicate-nullifier rejection over the local JSON transport.

  --full         Run scripts/demo-local-full-e2e.sh. Requires a local LEZ
                 sequencer, wallet home, and initialized/funded private account.
                 This is the default mode.

proving:
  --dev-mode     Set RISC0_DEV_MODE=1. Fast, non-production receipts.
  --real-prover  Set RISC0_DEV_MODE=0. Slower, cryptographic receipts.

full-mode env:
  PRIVATE_ACCOUNT=Private/<id>        Required for --full.
  THRESHOLD=1                         Optional.
  LOGOS_LEZ_REPO=/path/to/lez         Optional if ../logos-execution-zone exists.
  NSSA_WALLET_HOME_DIR=/path/to/home  Optional.

examples:
  ./demo.sh --quick
  ./demo.sh --messaging --real-prover
  PRIVATE_ACCOUNT=Private/<id> ./demo.sh --full --real-prover
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      MODE="quick"
      shift
      ;;
    --messaging)
      MODE="messaging"
      shift
      ;;
    --full)
      MODE="full"
      shift
      ;;
    --dev-mode)
      RISC0_MODE="1"
      shift
      ;;
    --real-prover)
      RISC0_MODE="0"
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

export RISC0_DEV_MODE="$RISC0_MODE"

case "$MODE" in
  quick)
    exec "$ROOT_DIR/scripts/demo-end-to-end.sh"
    ;;
  messaging)
    exec "$ROOT_DIR/scripts/demo-local-messaging.sh"
    ;;
  full)
    if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
      cat >&2 <<'EOF'
PRIVATE_ACCOUNT is required for ./demo.sh --full.

For a no-wallet smoke test, run:
  ./demo.sh --quick

For the local Messaging/off-chain path, run:
  ./demo.sh --messaging

For the full local sequencer path:
  1. Start the LEZ sequencer from LOGOS_LEZ_REPO.
  2. Run scripts/check-wallet-preflight.sh.
  3. Run scripts/prepare-local-private-account.sh.
  4. Re-run: PRIVATE_ACCOUNT=Private/<id> ./demo.sh --full --real-prover
EOF
      exit 2
    fi
    exec "$ROOT_DIR/scripts/demo-local-full-e2e.sh"
    ;;
  *)
    echo "internal error: unknown mode $MODE" >&2
    exit 2
    ;;
esac
