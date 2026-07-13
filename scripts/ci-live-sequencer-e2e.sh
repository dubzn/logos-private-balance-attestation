#!/usr/bin/env bash
# CI-only live sequencer E2E for LP-0005.
#
# The caller must provide a dedicated, disposable LEZ checkout. This script
# resets that checkout's standalone sequencer state, creates an ephemeral
# wallet, prepares a private account, runs the real wallet/sequencer witness
# path in RISC0 dev mode, and publishes only sanitized reports.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

if [[ "${CI:-}" != "true" && "${ALLOW_LOCAL_CI_E2E:-}" != "1" ]]; then
  cat >&2 <<'EOF'
This script resets sequencer state and is intended for CI or a disposable LEZ clone.

Set CI=true in GitHub Actions. For a local disposable clone, explicitly set:
  ALLOW_LOCAL_CI_E2E=1
EOF
  exit 2
fi

require_logos_lez_repo "$ROOT_DIR" \
  Cargo.toml \
  lee/state_machine/core \
  lez/wallet \
  lez/sequencer/service/configs/debug/sequencer_config.json

LEZ_REAL_PATH="$(cd "$LOGOS_LEZ_REPO" && pwd -P)"
PROJECT_REAL_PATH="$(cd "$ROOT_DIR" && pwd -P)"
if [[ "$LEZ_REAL_PATH" == "$PROJECT_REAL_PATH" || "$LEZ_REAL_PATH" == / || "$LEZ_REAL_PATH" == "$HOME" ]]; then
  echo "Refusing to reset unsafe LOGOS_LEZ_REPO path: $LEZ_REAL_PATH" >&2
  exit 2
fi

RUN_ROOT="${CI_E2E_RUN_ROOT:-$ROOT_DIR/.demo-runs/ci-live}"
SEQUENCER_PORT="${CI_SEQUENCER_PORT:-3040}"
PRIVATE_DIR="$RUN_ROOT/private"
PUBLIC_DIR="$RUN_ROOT/public"
WALLET_HOME="$PRIVATE_DIR/wallet"
PROOF_DIR="$PRIVATE_DIR/proof"
PREPARE_DIR="$PRIVATE_DIR/prepare"
SEQUENCER_LOG="$PRIVATE_DIR/sequencer.log"
SETUP_LOG="$PRIVATE_DIR/wallet-setup.log"
PUBLIC_REPORT="$PUBLIC_DIR/report.md"
SEQUENCER_PID=""
REPO_LEZ_LINK="$ROOT_DIR/logos"
ORIGINAL_LEZ_LINK_TARGET=""
CREATED_REPO_LEZ_LINK=0

if [[ -L "$REPO_LEZ_LINK" ]]; then
  ORIGINAL_LEZ_LINK_TARGET="$(readlink "$REPO_LEZ_LINK")"
elif [[ ! -e "$REPO_LEZ_LINK" ]]; then
  CREATED_REPO_LEZ_LINK=1
fi

rm -rf "$RUN_ROOT"
mkdir -p "$PRIVATE_DIR" "$PUBLIC_DIR"

if ! [[ "$SEQUENCER_PORT" =~ ^[0-9]+$ ]] || (( SEQUENCER_PORT < 1 || SEQUENCER_PORT > 65535 )); then
  echo "CI_SEQUENCER_PORT must be a valid TCP port." >&2
  exit 2
fi

sanitize_and_cleanup() {
  status=$?
  set +e
  trap - EXIT INT TERM
  if [[ -n "$SEQUENCER_PID" ]] && kill -0 "$SEQUENCER_PID" 2>/dev/null; then
    kill "$SEQUENCER_PID" 2>/dev/null
    wait "$SEQUENCER_PID" 2>/dev/null
  fi

  if [[ -f "$SEQUENCER_LOG" ]]; then
    tail -200 "$SEQUENCER_LOG" > "$PUBLIC_DIR/sequencer-tail.log"
  fi
  printf '{"status":"%s","exit_code":%d}\n' \
    "$([[ "$status" -eq 0 ]] && printf ok || printf failed)" \
    "$status" > "$PUBLIC_DIR/ci-status.json"

  if [[ -n "$ORIGINAL_LEZ_LINK_TARGET" ]]; then
    rm -f "$REPO_LEZ_LINK"
    ln -s "$ORIGINAL_LEZ_LINK_TARGET" "$REPO_LEZ_LINK"
  elif [[ "$CREATED_REPO_LEZ_LINK" -eq 1 && -L "$REPO_LEZ_LINK" ]]; then
    rm -f "$REPO_LEZ_LINK"
  fi

  rm -rf "$PRIVATE_DIR"
  exit "$status"
}
trap sanitize_and_cleanup EXIT INT TERM

export LOGOS_LEZ_REPO="$LEZ_REAL_PATH"
export LEZ_REPO="$LOGOS_LEZ_REPO"
export NSSA_WALLET_HOME_DIR="$WALLET_HOME"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
export RISC0_DEV_MODE=1
export RUST_LOG="${RUST_LOG:-info}"
export PATH="$LOGOS_LEZ_REPO/target/debug:$PATH"

ensure_repo_local_lez_link "$ROOT_DIR"

WALLET_BIN="$LOGOS_LEZ_REPO/target/debug/wallet"
SEQUENCER_BIN="$LOGOS_LEZ_REPO/target/debug/sequencer_service"
SEQUENCER_CONFIG="$LOGOS_LEZ_REPO/lez/sequencer/service/configs/debug/sequencer_config.json"

for binary in "$WALLET_BIN" "$SEQUENCER_BIN"; do
  if [[ ! -x "$binary" ]]; then
    echo "Missing prebuilt CI binary: $binary" >&2
    echo "Build wallet and standalone sequencer before running this script." >&2
    exit 2
  fi
done

echo "Resetting disposable standalone sequencer state."
rm -rf "$LOGOS_LEZ_REPO/rocksdb"
rm -f "$LOGOS_LEZ_REPO/bedrock_signing_key"

echo "Starting standalone sequencer."
(
  cd "$LOGOS_LEZ_REPO"
  exec "$SEQUENCER_BIN" "$SEQUENCER_CONFIG" --port "$SEQUENCER_PORT"
) > "$SEQUENCER_LOG" 2>&1 &
SEQUENCER_PID=$!

CI_SEQUENCER_PORT="$SEQUENCER_PORT" python3 - <<'PY'
import os
import socket
import time

port = int(os.environ["CI_SEQUENCER_PORT"])
deadline = time.time() + 90
while time.time() < deadline:
    sock = socket.socket()
    sock.settimeout(1)
    try:
        sock.connect(("127.0.0.1", port))
        break
    except OSError:
        time.sleep(1)
    finally:
        sock.close()
else:
    raise SystemExit(f"sequencer did not listen on 127.0.0.1:{port} within 90 seconds")
PY

if ! kill -0 "$SEQUENCER_PID" 2>/dev/null; then
  echo "Sequencer exited before wallet setup." >&2
  tail -100 "$SEQUENCER_LOG" >&2 || true
  exit 1
fi

echo "Creating ephemeral CI wallet and private account."
mkdir -p "$WALLET_HOME"
cat > "$WALLET_HOME/wallet_config.json" <<EOF
{
  "sequencer_addr": "http://127.0.0.1:$SEQUENCER_PORT/",
  "seq_poll_timeout": "1s",
  "seq_tx_poll_max_blocks": 30,
  "seq_poll_max_retries": 10,
  "seq_block_poll_max_amount": 100
}
EOF
printf 'ci-only-password\n' \
  | "$WALLET_BIN" account new private --label ci-private \
    > "$SETUP_LOG" 2>&1

PRIVATE_ACCOUNT="$(grep -Eo 'Private/[1-9A-HJ-NP-Za-km-z]+' "$SETUP_LOG" | tail -1)"
if [[ -z "$PRIVATE_ACCOUNT" ]]; then
  echo "Wallet did not report a private account id." >&2
  exit 1
fi

echo "Checking wallet/sequencer compatibility."
"$WALLET_BIN" check-health > "$PRIVATE_DIR/wallet-health.log" 2>&1

echo "Initializing and funding the ephemeral private account."
OUT_DIR="$PREPARE_DIR" \
PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
THRESHOLD=1 \
RISC0_DEV_MODE=1 \
  "$ROOT_DIR/scripts/prepare-local-private-account.sh" \
  > "$PRIVATE_DIR/prepare-command.log" 2>&1

echo "Running live wallet + getProofForCommitment E2E."
DEMO_DIR="$PROOF_DIR" \
PRIVATE_ACCOUNT="$PRIVATE_ACCOUNT" \
THRESHOLD=1 \
RISC0_DEV_MODE=1 \
  "$ROOT_DIR/scripts/demo-local-sequencer-e2e.sh" \
  > "$PRIVATE_DIR/proof-command.log" 2>&1

if [[ "$(jq -r '.status' "$PROOF_DIR/verify.json")" != "ok" ]]; then
  echo "Live E2E verifier did not return status=ok." >&2
  exit 1
fi

cp "$PROOF_DIR/verify.json" "$PUBLIC_DIR/verify.json"
jq '{
  status,
  risc0_dev_mode,
  threshold,
  durations
}' "$PROOF_DIR/run.json" > "$PUBLIC_DIR/proof-run.json"
jq '{
  inspect: {
    private_state_found: .inspect.private_state_found,
    local_commitment_matches_wallet: .inspect.local_commitment_matches_wallet,
    membership_proof_found: .inspect.membership_proof_found,
    proof_index: .inspect.proof_index,
    proof_depth: .inspect.proof_depth,
    commitment_root_hex: .inspect.commitment_root_hex,
    core_root_matches_wallet_root: .inspect.core_root_matches_wallet_root,
    proof_source: .inspect.proof_source
  },
  witness: {
    threshold: .witness.threshold,
    commitment_root_hex: .witness.commitment_root_hex,
    context_id_hex: .witness.context_id_hex,
    context_nullifier_hex: .witness.context_nullifier_hex,
    presenter_pubkey_hex: .witness.presenter_pubkey_hex,
    presenter_id_hex: .witness.presenter_id_hex,
    verifier_id_hex: .witness.verifier_id_hex,
    circuit_image_id_hex: .witness.circuit_image_id_hex,
    proof_index: .witness.proof_index,
    proof_depth: .witness.proof_depth
  }
}' "$PROOF_DIR/build-witness-summary.json" \
  > "$PUBLIC_DIR/build-witness-summary.json"

LEZ_REF="$(git -C "$LOGOS_LEZ_REPO" rev-parse HEAD)"
cat > "$PUBLIC_REPORT" <<EOF
# CI Live Sequencer E2E

| Field | Value |
| --- | --- |
| Status | \`ok\` |
| LEZ ref | \`$LEZ_REF\` |
| RISC Zero mode | \`RISC0_DEV_MODE=1\` |
| Wallet | ephemeral; deleted before artifact upload |
| Private witness | deleted before artifact upload |
| Verify status | \`ok\` |

The job started a standalone LEZ sequencer, created a fresh wallet/private
account, initialized and funded it locally, fetched the real
\`getProofForCommitment\` membership path, generated a dev-mode attestation,
and verified the public envelope. Only sanitized reports are uploaded.
EOF

echo "CI live sequencer E2E passed."
echo "Sanitized artifacts: $PUBLIC_DIR"
