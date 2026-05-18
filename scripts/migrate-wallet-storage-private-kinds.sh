#!/usr/bin/env bash
# Migrate local LEZ wallet storage from the pre-PrivateAccountKind JSON shape
# to the current shape used by newer logos-execution-zone checkouts.
#
# The migration is intentionally narrow:
#   Private.identifiers -> Private.kinds = [{"Regular": identifier}, ...]
#   Private.data.value[1] entries [identifier, account]
#     -> [{"Regular": identifier}, account]
#
# A timestamped backup is always written before modification.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

APPLY=0

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/migrate-wallet-storage-private-kinds.sh [--yes]

env:
  LOGOS_LEZ_REPO or LEZ_REPO       Path to logos-execution-zone checkout.
  NSSA_WALLET_HOME_DIR            Wallet home. Defaults to $LOGOS_LEZ_REPO/.wallet-local.

By default this is a dry run. Pass --yes to write storage.json after creating
a timestamped backup next to it.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes)
      APPLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

require_logos_lez_repo "$ROOT_DIR" wallet nssa/core
export_default_wallet_home

STORAGE="$NSSA_WALLET_HOME_DIR/storage.json"
if [[ ! -f "$STORAGE" ]]; then
  echo "wallet storage not found: $STORAGE" >&2
  exit 2
fi

python3 - "$STORAGE" "$APPLY" <<'PY'
import json
import os
import shutil
import sys
from datetime import datetime, timezone

path = sys.argv[1]
apply = sys.argv[2] == "1"

with open(path, "r", encoding="utf-8") as fh:
    storage = json.load(fh)

changed = 0
accounts = storage.get("accounts")
if not isinstance(accounts, list):
    print("storage.json has no top-level accounts array", file=sys.stderr)
    sys.exit(2)

for entry in accounts:
    if not isinstance(entry, dict):
        continue

    private = entry.get("Private")
    if not isinstance(private, dict):
        continue

    identifiers = private.pop("identifiers", None)
    if "kinds" not in private:
        if identifiers is None:
            private["kinds"] = []
        else:
            private["kinds"] = [{"Regular": identifier} for identifier in identifiers]
        changed += 1
    elif identifiers is not None:
        changed += 1

    value = private.get("data", {}).get("value")
    if isinstance(value, list) and len(value) > 1 and isinstance(value[1], list):
        for pair in value[1]:
            if isinstance(pair, list) and pair and isinstance(pair[0], int):
                pair[0] = {"Regular": pair[0]}
                changed += 1

print(f"wallet storage path: {path}")
print(f"entries changed: {changed}")

if changed == 0:
    print("no migration needed")
    sys.exit(0)

if not apply:
    print("dry run only; re-run with --yes to write changes")
    sys.exit(0)

stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
backup = f"{path}.backup-before-private-kinds-{stamp}"
shutil.copy2(path, backup)
tmp = f"{path}.tmp-{os.getpid()}"
with open(tmp, "w", encoding="utf-8") as fh:
    json.dump(storage, fh, indent=2)
    fh.write("\n")
os.replace(tmp, path)
print(f"backup: {backup}")
print("migration written")
PY
