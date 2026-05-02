#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_MD="${SPIKE05_OUTPUT:-$ROOT_DIR/.spike-results/spike-05-devmode.md}"

"$ROOT_DIR/scripts/spike-05-run-baseline.sh" "devmode" "1" "$OUTPUT_MD"
