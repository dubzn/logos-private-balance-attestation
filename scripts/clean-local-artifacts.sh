#!/usr/bin/env bash
# Clean local generated artifacts without touching source files.
#
# Default mode is a dry-run. Pass --yes to delete.
#
# By default this only targets build outputs:
#   target/
#   lez-verifier/program/target/
#   spikes/spike-08-program-chaining/lez/runner/target/
#
# Optional:
#   --include-runs     also remove .demo-runs/ and .spike-results/
#   --include-logos    also remove the local ./logos checkout
#   --include-wallets  also remove local .wallet-local dirs (dangerous)

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIRM=0
INCLUDE_RUNS=0
INCLUDE_LOGOS=0
INCLUDE_WALLETS=0

usage() {
  cat <<'EOF'
usage:
  scripts/clean-local-artifacts.sh [--yes] [options]

options:
  --yes              Delete candidates. Without this, only prints what would be removed.
  --include-runs     Include .demo-runs/ and .spike-results/.
  --include-logos    Include ./logos local checkout.
  --include-wallets  Include .wallet-local directories. Dangerous: wallet state may be lost.
  -h, --help         Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes)
      CONFIRM=1
      ;;
    --include-runs)
      INCLUDE_RUNS=1
      ;;
    --include-logos)
      INCLUDE_LOGOS=1
      ;;
    --include-wallets)
      INCLUDE_WALLETS=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

cd "$ROOT_DIR"

declare -a CANDIDATES=(
  "target"
  "lez-verifier/program/target"
  "spikes/spike-08-program-chaining/lez/runner/target"
)

if [[ "$INCLUDE_RUNS" == "1" ]]; then
  CANDIDATES+=(".demo-runs" ".spike-results")
fi
if [[ "$INCLUDE_LOGOS" == "1" ]]; then
  CANDIDATES+=("logos")
fi
if [[ "$INCLUDE_WALLETS" == "1" ]]; then
  while IFS= read -r wallet_dir; do
    CANDIDATES+=("$wallet_dir")
  done < <(find . -name .wallet-local -type d -prune | sed 's#^\./##' | sort)
fi

echo "Repository: $ROOT_DIR"
if [[ "$CONFIRM" == "1" ]]; then
  echo "Mode: delete"
else
  echo "Mode: dry-run (pass --yes to delete)"
fi
echo

FOUND=0
for path in "${CANDIDATES[@]}"; do
  if [[ ! -e "$path" ]]; then
    continue
  fi
  FOUND=1
  size="$(du -sh "$path" 2>/dev/null | awk '{print $1}')"
  printf '%-70s %8s\n' "$path" "$size"
  if [[ "$CONFIRM" == "1" ]]; then
    rm -rf -- "$path"
  fi
done

if [[ "$FOUND" == "0" ]]; then
  echo "No matching local artifacts found."
fi

if [[ "$CONFIRM" == "1" ]]; then
  echo
  echo "Clean complete."
else
  echo
  echo "Dry-run complete. Nothing was deleted."
fi
