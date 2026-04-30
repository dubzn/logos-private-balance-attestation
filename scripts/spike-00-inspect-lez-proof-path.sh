#!/usr/bin/env bash
set -euo pipefail

LEZ_REPO="${LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"

privacy_circuit="$LEZ_REPO/nssa/src/privacy_preserving_transaction/circuit.rs"
public_program="$LEZ_REPO/nssa/src/program.rs"
guest_circuit="$LEZ_REPO/program_methods/guest/src/bin/privacy_preserving_circuit.rs"

for file in "$privacy_circuit" "$public_program" "$guest_circuit"; do
  if [[ ! -f "$file" ]]; then
    echo "Missing expected LEZ source file: $file" >&2
    exit 1
  fi
done

echo "[1/4] Checking RISC Zero version"
"$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-risc0-version.sh"

echo "[2/4] Checking guest-side verification API"
if ! grep -q "env::verify" "$guest_circuit"; then
  echo "Expected env::verify in privacy preserving circuit guest." >&2
  exit 1
fi
echo "Found env::verify in privacy preserving circuit guest."

echo "[3/4] Checking private execution assumption wiring"
if ! grep -q "env_builder.add_assumption(inner_receipt)" "$privacy_circuit"; then
  echo "Expected add_assumption in private execution proof builder." >&2
  exit 1
fi
echo "Found add_assumption in private execution proof builder."

echo "[4/4] Checking public execution path"
if grep -q "add_assumption" "$public_program"; then
  echo "Public Program::execute appears to add assumptions. Inspect manually." >&2
  exit 1
fi

if ! grep -q "default_executor" "$public_program"; then
  echo "Expected public Program::execute to use default_executor." >&2
  exit 1
fi

cat <<'REPORT'

Static finding:
- LEZ private execution already wires child program receipts as RISC Zero
  assumptions.
- Public program execution appears to use default_executor without an
  assumption channel.

Interpretation:
- Direct receipt verification from a public LEZ transaction is the first runtime
  spike to test.
- If it fails, prioritize the Logos-native private execution gate or an
  evaluator-confirmed native verifier path.
REPORT

