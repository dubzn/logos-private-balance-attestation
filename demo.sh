#!/usr/bin/env bash
# Lambda Prize entrypoint for the reproducible local LP-0005 demo.
#
# This thin wrapper exists so external validation tooling can find a root
# demo.sh while the implementation keeps the real operator script under
# scripts/.
#
# Required:
#   PRIVATE_ACCOUNT=Private/<id>
#
# Recommended for final demo evidence:
#   RISC0_DEV_MODE=0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT_DIR/scripts/demo-local-full-e2e.sh" "$@"
