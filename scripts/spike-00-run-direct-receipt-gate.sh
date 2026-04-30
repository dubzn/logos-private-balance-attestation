#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
PROGRAM_BIN="$LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker/receipt_gate.bin"
SEQUENCER_SETTLE_SECONDS="${SEQUENCER_SETTLE_SECONDS:-3}"
SPIKE_MARKER_VALUE="${SPIKE_MARKER:-receipt-gate-ok}"

if [[ -z "${SPIKE_ACCOUNT:-}" ]]; then
  echo "Set SPIKE_ACCOUNT to a local public account id without Public/." >&2
  echo "Example:" >&2
  echo "  SPIKE_ACCOUNT=9WF... scripts/spike-00-run-direct-receipt-gate.sh" >&2
  exit 1
fi

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEZ_REPO/.wallet-local}"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  "$ROOT_DIR/scripts/spike-00-build-lez-program.sh"
fi

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Missing receipt_gate binary: $PROGRAM_BIN" >&2
  exit 1
fi

if [[ "${SKIP_DEPLOY:-0}" != "1" ]]; then
  echo "[1/2] Deploying receipt_gate program"
  wallet deploy-program "$PROGRAM_BIN"
else
  echo "[1/2] Skipping deploy"
fi

echo "[2/2] Running receipt_gate public transaction"
cd "$LEZ_REPO/examples/program_deployment"

set +e
cargo run --bin run_receipt_gate \
  "$PROGRAM_BIN" \
  "$SPIKE_ACCOUNT" \
  "$SPIKE_MARKER_VALUE"
status=$?
set -e

if [[ "$status" -ne 0 ]]; then
  cat >&2 <<'REPORT'

Direct receipt gate transaction failed.

This is an expected possible result for Spike 0A: public LEZ execution may not
currently expose a channel to provide the RISC Zero assumption/receipt required
by guest-side env::verify.

Next checks:
- confirm sequencer logs for the exact failure
- test recursive/native verifier support
- prototype the Logos-native private execution gate and ask evaluators whether
  it satisfies the LP-0005 on-chain path
REPORT
  exit "$status"
fi

echo "Direct receipt gate transaction submitted successfully."
echo "Waiting ${SEQUENCER_SETTLE_SECONDS}s for sequencer processing..."
sleep "$SEQUENCER_SETTLE_SECONDS"

account_output="$(wallet account get --account-id "Public/$SPIKE_ACCOUNT" 2>&1 || true)"
marker_hex="$(printf '%s' "$SPIKE_MARKER_VALUE" | od -An -tx1 | tr -d ' \n')"

if printf '%s\n' "$account_output" | grep -qi "$marker_hex"; then
  echo "Direct receipt gate marker found in account data."
  exit 0
fi

cat >&2 <<REPORT

Direct receipt gate marker was not found in account data.

This usually means the sequencer accepted the transaction submission but later
rejected execution, or the account was not updated by the verifier guest.

Expected marker hex:
$marker_hex

Account output:
$account_output

Check the sequencer logs for the exact execution error.
REPORT

exit 1
