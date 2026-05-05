#!/usr/bin/env bash
# Full local LP-0005 demo:
#   private wallet account -> real membership proof -> RISC Zero envelope
#   -> host verifier -> deployable LEZ gate -> persisted nullifier.
#
# This composes:
#   scripts/demo-local-sequencer-e2e.sh
#   scripts/demo-local-gate-e2e.sh
#
# Required:
#   PRIVATE_ACCOUNT=Private/<id> or bare private account id
#
# Optional:
#   FULL_DEMO_DIR                    Defaults to .demo-runs/local-full/<timestamp>
#   RISC0_DEV_MODE                   Defaults to 1. Use 0 for real proving.
#   THRESHOLD                        Defaults to 1.
#   LOGOS_LEZ_REPO or LEZ_REPO       Defaults to $HOME/logos/src/logos-execution-zone.
#   NSSA_WALLET_HOME_DIR             Defaults to $LOGOS_LEZ_REPO/.wallet-local.
#   CHECK_DUPLICATE                  Defaults to 1 for the gate phase.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
FULL_DEMO_DIR="${FULL_DEMO_DIR:-$ROOT_DIR/.demo-runs/local-full/$TIMESTAMP}"
PROOF_DIR="$FULL_DEMO_DIR/proof"
GATE_DIR="$FULL_DEMO_DIR/gate"
LOG_DIR="$FULL_DEMO_DIR/logs"
REPORT="$FULL_DEMO_DIR/report.md"
RUN_JSON="$FULL_DEMO_DIR/run.json"
export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"

usage() {
  cat >&2 <<'EOF'
usage:
  PRIVATE_ACCOUNT=Private/<id> scripts/demo-local-full-e2e.sh

env:
  PRIVATE_ACCOUNT                   Required private wallet account.
  FULL_DEMO_DIR                     Output directory.
  RISC0_DEV_MODE                    Defaults to 1. Use 0 for real proving.
  THRESHOLD                         Defaults to 1.
  LOGOS_LEZ_REPO or LEZ_REPO        Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR              Wallet home.
  CHECK_DUPLICATE                   Defaults to 1 for the gate phase.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "${PRIVATE_ACCOUNT:-}" ]]; then
  echo "PRIVATE_ACCOUNT is required." >&2
  usage
  exit 2
fi

step() {
  printf '\n\033[1;36m== %s ==\033[0m\n' "$1"
}

duration() {
  local start="$1"
  local end
  end="$(date +%s)"
  printf "%02d:%02d:%02d" $(((end - start) / 3600)) $((((end - start) % 3600) / 60)) $(((end - start) % 60))
}

json_field() {
  local file="$1"
  local path="$2"
  python3 - "$file" "$path" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    value = json.load(fh)
for key in sys.argv[2].split("."):
    value = value[key]
print(value)
PY
}

mkdir -p "$LOG_DIR"
TOTAL_STARTED="$(date +%s)"

step "1/3 Build witness, prove, and verify from real wallet/sequencer"
proof_started="$(date +%s)"
DEMO_DIR="$PROOF_DIR" \
  "$ROOT_DIR/scripts/demo-local-sequencer-e2e.sh" \
  2>&1 | tee "$LOG_DIR/proof-phase.log"
proof_duration="$(duration "$proof_started")"

if [[ ! -f "$PROOF_DIR/envelope.json" || ! -f "$PROOF_DIR/gate.json" ]]; then
  echo "Proof phase did not produce envelope.json and gate.json under $PROOF_DIR" >&2
  exit 1
fi

step "2/3 Register presenter, initialize LEZ gate, and admit"
gate_started="$(date +%s)"
RUN_DIR="$PROOF_DIR" \
DEMO_DIR="$GATE_DIR" \
  "$ROOT_DIR/scripts/demo-local-gate-e2e.sh" \
  2>&1 | tee "$LOG_DIR/gate-phase.log"
gate_duration="$(duration "$gate_started")"

if [[ ! -f "$GATE_DIR/run.json" ]]; then
  echo "Gate phase did not produce run.json under $GATE_DIR" >&2
  exit 1
fi

step "3/3 Write full demo report"
total_duration="$(duration "$TOTAL_STARTED")"
verify_status="$(json_field "$PROOF_DIR/verify.json" status)"
context_nullifier="$(json_field "$GATE_DIR/run.json" context_nullifier)"
nullifier_count="$(json_field "$GATE_DIR/run.json" nullifier_count)"
duplicate_status="$(json_field "$GATE_DIR/run.json" duplicate_status)"
gate_account="$(json_field "$GATE_DIR/run.json" accounts.gate)"
presenter_account="$(json_field "$GATE_DIR/run.json" accounts.presenter)"

if [[ "$verify_status" != "ok" ]]; then
  echo "Expected proof verifier status ok, got: $verify_status" >&2
  exit 1
fi
if [[ "$nullifier_count" -lt 1 ]]; then
  echo "Expected nullifier count >= 1, got: $nullifier_count" >&2
  exit 1
fi

cat > "$RUN_JSON" <<EOF
{
  "status": "ok",
  "risc0_dev_mode": "$RISC0_DEV_MODE",
  "private_account": "Private/<redacted>",
  "full_demo_dir": "$FULL_DEMO_DIR",
  "proof_dir": "$PROOF_DIR",
  "gate_dir": "$GATE_DIR",
  "verify_status": "$verify_status",
  "context_nullifier": "$context_nullifier",
  "nullifier_count": "$nullifier_count",
  "duplicate_status": "$duplicate_status",
  "gate_account": "$gate_account",
  "presenter_account": "$presenter_account",
  "durations": {
    "proof_phase": "$proof_duration",
    "gate_phase": "$gate_duration",
    "total": "$total_duration"
  }
}
EOF

cat > "$REPORT" <<EOF
# Full Local E2E

| Phase | Script | Output | Duration |
| --- | --- | --- | --- |
| Proof | \`scripts/demo-local-sequencer-e2e.sh\` | \`$PROOF_DIR/report.md\` | $proof_duration |
| Gate | \`scripts/demo-local-gate-e2e.sh\` | \`$GATE_DIR/report.md\` | $gate_duration |
| Total | - | \`$RUN_JSON\` | $total_duration |

## Result

- Verify status: \`$verify_status\`
- Gate account: \`$gate_account\`
- Presenter account: \`$presenter_account\`
- Context nullifier: \`$context_nullifier\`
- Nullifier count in gate data: \`$nullifier_count\`
- Duplicate admit: \`$duplicate_status\`
- \`RISC0_DEV_MODE=$RISC0_DEV_MODE\`

## Privacy Note

\`$PROOF_DIR/witness.json\` is private and must not be published. The public
artifacts are the proof envelope, gate file, verifier output, gate report, and
this summary.
EOF

cat "$REPORT"

printf '\n\033[1;32mFull local E2E complete.\033[0m Artifacts in: %s\n' "$FULL_DEMO_DIR"
if [[ "$RISC0_DEV_MODE" != "0" ]]; then
  printf '\033[1;33mNote:\033[0m RISC0_DEV_MODE=%s — receipts are NOT cryptographically valid.\n' "$RISC0_DEV_MODE"
  printf '       Re-run with RISC0_DEV_MODE=0 for the real-prover variant.\n'
fi
