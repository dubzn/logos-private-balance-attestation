#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_MD="${SPIKE05_OUTPUT:-$ROOT_DIR/.spike-results/spike-05-prod.md}"

"$ROOT_DIR/scripts/spike-05-run-baseline.sh" "prod" "0" "$OUTPUT_MD"
