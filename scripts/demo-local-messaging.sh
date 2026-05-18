#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
DEMO_DIR="${DEMO_DIR:-$ROOT_DIR/.demo-runs/local-messaging/$(date -u +%Y%m%dT%H%M%SZ)}"
GROUP_ID="${GROUP_ID:-demo-chat}"
SENDER_ID="${SENDER_ID:-presenter-local}"
RECIPIENT_ID="${RECIPIENT_ID:-chat-host-local}"

mkdir -p "$DEMO_DIR"

step() {
  printf "\n== %s ==\n" "$1"
}

run_logged() {
  local label="$1"
  shift
  printf '+ %s\n' "$*"
  "$@" >"$DEMO_DIR/${label}.stdout" 2>"$DEMO_DIR/${label}.stderr"
}

step "1/5 Build deterministic fixture"
cargo run -p demo-runner --bin build-demo-fixtures -- "$DEMO_DIR/fixture"

step "2/5 Prove and verify public envelope"
RISC0_DEV_MODE="$RISC0_DEV_MODE" cargo run -p attestation-cli -- prove \
  --witness "$DEMO_DIR/fixture/witness.json" \
  --out "$DEMO_DIR/envelope.json"

RISC0_DEV_MODE="$RISC0_DEV_MODE" cargo run -p attestation-cli -- verify \
  --envelope "$DEMO_DIR/envelope.json" \
  --gate "$DEMO_DIR/fixture/gate.json" \
  > "$DEMO_DIR/verify.json"

step "3/5 Export and receive local proof message"
cargo run -p attestation-cli -- message-export \
  --envelope "$DEMO_DIR/envelope.json" \
  --out "$DEMO_DIR/message.json" \
  --group "$GROUP_ID" \
  --sender "$SENDER_ID" \
  --recipient "$RECIPIENT_ID" \
  > "$DEMO_DIR/message-export.json"

cargo run -p attestation-cli -- message-receive \
  --message "$DEMO_DIR/message.json" \
  --out "$DEMO_DIR/received-envelope.json" \
  > "$DEMO_DIR/message-receive.json"

step "4/5 Verify message and admit once"
cargo run -p attestation-cli -- message-verify \
  --message "$DEMO_DIR/message.json" \
  --gate "$DEMO_DIR/fixture/gate.json" \
  > "$DEMO_DIR/message-verify.json"

cargo run -p attestation-cli -- message-admit \
  --message "$DEMO_DIR/message.json" \
  --gate "$DEMO_DIR/fixture/gate.json" \
  --state "$DEMO_DIR/admissions.json" \
  --group "$GROUP_ID" \
  > "$DEMO_DIR/message-admit.json"

step "5/5 Confirm duplicate admission is rejected"
set +e
cargo run -p attestation-cli -- message-admit \
  --message "$DEMO_DIR/message.json" \
  --gate "$DEMO_DIR/fixture/gate.json" \
  --state "$DEMO_DIR/admissions.json" \
  --group "$GROUP_ID" \
  > "$DEMO_DIR/message-admit-duplicate.stdout" \
  2> "$DEMO_DIR/message-admit-duplicate.stderr"
duplicate_status=$?
set -e

if [[ "$duplicate_status" -eq 0 ]]; then
  echo "duplicate admission unexpectedly succeeded" >&2
  exit 1
fi

cat > "$DEMO_DIR/report.md" <<EOF
# Local Messaging Demo

| Artifact | Path |
| --- | --- |
| Envelope | \`$DEMO_DIR/envelope.json\` |
| Message | \`$DEMO_DIR/message.json\` |
| Received envelope | \`$DEMO_DIR/received-envelope.json\` |
| Admission state | \`$DEMO_DIR/admissions.json\` |

## Result

- Verify status: \`$(jq -r .status "$DEMO_DIR/message-verify.json")\`
- Admit status: \`$(jq -r .status "$DEMO_DIR/message-admit.json")\`
- Group: \`$GROUP_ID\`
- Member count: \`$(jq -r .member_count "$DEMO_DIR/message-admit.json")\`
- Duplicate admission: \`rejected\`
- RISC0_DEV_MODE: \`$RISC0_DEV_MODE\`

## Notes

This is a local JSON transport adapter for the off-chain path. It preserves the
same public proof envelope and admission semantics expected from a future Logos
Messaging transport.
EOF

cat "$DEMO_DIR/report.md"
echo
echo "Local messaging demo complete. Artifacts in: $DEMO_DIR"
