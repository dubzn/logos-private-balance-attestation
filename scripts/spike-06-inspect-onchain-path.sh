#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"
require_logos_lez_repo "$ROOT_DIR" nssa/src/program.rs nssa/src/privacy_preserving_transaction/circuit.rs program_methods/guest/src/bin/privacy_preserving_circuit.rs
LEZ_REPO="$LOGOS_LEZ_REPO"
OUTPUT="${SPIKE06_OUTPUT:-$ROOT_DIR/.spike-results/spike-06-onchain-path.md}"

mkdir -p "$(dirname "$OUTPUT")"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "Missing expected file: $path" >&2
    exit 1
  fi
}

PUBLIC_PROGRAM="$LEZ_REPO/nssa/src/program.rs"
PRIVATE_CIRCUIT_HOST="$LEZ_REPO/nssa/src/privacy_preserving_transaction/circuit.rs"
PRIVATE_CIRCUIT_GUEST="$LEZ_REPO/program_methods/guest/src/bin/privacy_preserving_circuit.rs"
REFERENCE_NOTES="$ROOT_DIR/docs/REFERENCE_NOTES.md"

require_file "$PUBLIC_PROGRAM"
require_file "$PRIVATE_CIRCUIT_HOST"
require_file "$PRIVATE_CIRCUIT_GUEST"
require_file "$REFERENCE_NOTES"

grep_count() {
  local pattern="$1"
  local file="$2"
  (grep -nE "$pattern" "$file" 2>/dev/null || true) | wc -l | tr -d ' '
}

grep_has() {
  local pattern="$1"
  local file="$2"
  grep -qE "$pattern" "$file"
}

public_add_assumption_count="$(grep_count "add_assumption" "$PUBLIC_PROGRAM")"
private_add_assumption_count="$(grep_count "add_assumption" "$PRIVATE_CIRCUIT_HOST")"
private_env_verify_count="$(grep_count "env::verify" "$PRIVATE_CIRCUIT_GUEST")"
public_execute_uses_executor="$(
  if grep_has "default_executor\\(\\)" "$PUBLIC_PROGRAM"; then
    printf yes
  else
    printf no
  fi
)"
private_uses_succinct="$(
  if grep_has "ProverOpts::succinct" "$PRIVATE_CIRCUIT_HOST"; then
    printf yes
  else
    printf no
  fi
)"
spike_00_error="$(
  if grep_has "no receipt found to resolve assumption" "$REFERENCE_NOTES"; then
    printf observed
  else
    printf missing
  fi
)"

cat > "$OUTPUT" <<EOF
# Spike 06 On-Chain Path Inspection

- Date: \`$(date -u +%Y-%m-%dT%H:%M:%SZ)\`
- LEZ_REPO: \`$LEZ_REPO\`

| Check | Result | Evidence |
| --- | --- | --- |
| Public LEZ execution uses default executor | $public_execute_uses_executor | \`$PUBLIC_PROGRAM\` |
| Public LEZ execution adds receipt assumptions | $public_add_assumption_count | \`grep add_assumption $PUBLIC_PROGRAM\` |
| Private LEZ proof host adds assumptions | $private_add_assumption_count | \`$PRIVATE_CIRCUIT_HOST\` |
| Private LEZ guest calls \`env::verify\` | $private_env_verify_count | \`$PRIVATE_CIRCUIT_GUEST\` |
| Private LEZ proof uses succinct receipts | $private_uses_succinct | \`grep ProverOpts::succinct $PRIVATE_CIRCUIT_HOST\` |
| Spike 0A missing assumption error recorded | $spike_00_error | \`$REFERENCE_NOTES\` |

## Interpretation

- RISC Zero assumptions are used by the Logos-native private execution path.
- The local public LEZ execution path does not expose an assumption injection
  point in \`nssa/src/program.rs\`.
- Spike 0A already showed that a public LEZ program calling \`env::verify\`
  fails at runtime with \`no receipt found to resolve assumption\`.
- No local deployable public LEZ verifier path was found for an external
  standalone RISC Zero receipt.

## Decision

For the next implementation milestone, do not build the on-chain path around a
public LEZ program that directly verifies external RISC Zero receipts.

Keep a two-track architecture:

1. Off-chain path: standalone RISC Zero proof envelope verified locally.
2. On-chain path: host-verified proof envelope followed by a deployable LEZ
   gate-ledger/nullifier transaction, pending evaluator confirmation for
   LP-0005.

EOF

cat "$OUTPUT"
