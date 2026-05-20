#!/usr/bin/env bash
# Lambda Prize entrypoint for LP-0005 demos.
#
# Modes:
#   --clean-room  Evaluator-style runner: hygiene + quick + messaging,
#                 with optional live paths.
#   --quick       Synthetic fixture: prove + verify, no wallet/sequencer.
#   --messaging   Synthetic fixture: proof message + local admission.
#   --full        Live local wallet/sequencer + Workable LEZ gate path.
#   --ppe-gate    Live local PPE-native LEZ balance gate spike.
#
# Proving mode:
#   --dev-mode    RISC0_DEV_MODE=1 (fast, non-production receipts).
#   --real-prover RISC0_DEV_MODE=0 (slower, cryptographic receipts).
#
# Default mode is --clean-room so the root demo is runnable without wallet
# setup. Use --full for the live wallet/sequencer path.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="clean-room"
RISC0_MODE="${RISC0_DEV_MODE:-1}"
CLEAN_ROOM_ARGS=()

usage() {
  cat >&2 <<'EOF'
usage:
  ./demo.sh [--clean-room|--quick|--messaging|--full|--ppe-gate] [--dev-mode|--real-prover] [clean-room options]

modes:
  --clean-room   Run scripts/demo-clean-room.sh. This is the default mode.
                 It runs public hygiene, quick proof/verify, and local
                 Messaging without requiring wallet or sequencer setup.

  --quick        Run scripts/demo-end-to-end.sh with deterministic fixtures.
                 No wallet, sequencer, or private account is required.

  --messaging    Run scripts/demo-local-messaging.sh with deterministic
                 fixtures. Demonstrates export, receive, verify, admit, and
                 duplicate-nullifier rejection over the local JSON transport.

  --full         Run scripts/demo-local-full-e2e.sh. Requires a local LEZ
                 sequencer, wallet home, and initialized/funded private account.

  --ppe-gate     Run scripts/spike-09-demo-ppe-gate.sh. Requires a local LEZ
                 sequencer and wallet home. Creates fresh local accounts,
                 checks private balance inside LEZ privacy-preserving
                 execution, writes public BAP1 gate/nullifier state, and
                 benchmarks the positive, duplicate, and negative paths.

proving:
  --dev-mode     Set RISC0_DEV_MODE=1. Fast, non-production receipts.
  --real-prover  Set RISC0_DEV_MODE=0. Slower, cryptographic receipts.

clean-room options:
  --with-tests   Include cargo fmt/test in the clean-room hygiene check.
  --with-lez     Include LEZ compatibility checks.
  --with-basecamp Include Nix Basecamp package build/inspection.
  --with-live    Include full local E2E. Requires PRIVATE_ACCOUNT=Private/<id>.
  --with-ppe     Include the Spike 09 PPE-native gate demo.

full-mode env:
  PRIVATE_ACCOUNT=Private/<id>        Required for --full.
  THRESHOLD=1                         Optional.
  LOGOS_LEZ_REPO=/path/to/lez         Optional if ../logos-execution-zone exists.
  NSSA_WALLET_HOME_DIR=/path/to/home  Optional.

examples:
  ./demo.sh
  ./demo.sh --clean-room --real-prover
  PRIVATE_ACCOUNT=Private/<id> ./demo.sh --clean-room --real-prover --with-lez --with-basecamp --with-live --with-ppe
  ./demo.sh --quick
  ./demo.sh --messaging --real-prover
  PRIVATE_ACCOUNT=Private/<id> ./demo.sh --full --real-prover
  ./demo.sh --ppe-gate --real-prover
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean-room)
      MODE="clean-room"
      shift
      ;;
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
    --ppe-gate)
      MODE="ppe-gate"
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
    --with-tests|--with-lez|--with-basecamp|--with-live|--with-ppe)
      CLEAN_ROOM_ARGS+=("$1")
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

if [[ "$MODE" != "clean-room" && "${#CLEAN_ROOM_ARGS[@]}" -gt 0 ]]; then
  echo "--with-tests/--with-lez/--with-basecamp/--with-live/--with-ppe only apply to --clean-room" >&2
  exit 2
fi

case "$MODE" in
  clean-room)
    if [[ "$RISC0_MODE" == "0" ]]; then
      CLEAN_ROOM_ARGS=(--real-prover "${CLEAN_ROOM_ARGS[@]}")
    else
      CLEAN_ROOM_ARGS=(--dev-mode "${CLEAN_ROOM_ARGS[@]}")
    fi
    exec "$ROOT_DIR/scripts/demo-clean-room.sh" "${CLEAN_ROOM_ARGS[@]}"
    ;;
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
  ppe-gate)
    exec "$ROOT_DIR/scripts/spike-09-demo-ppe-gate.sh"
    ;;
  *)
    echo "internal error: unknown mode $MODE" >&2
    exit 2
    ;;
esac
