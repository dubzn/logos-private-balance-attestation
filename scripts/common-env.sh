#!/usr/bin/env bash
# Shared local environment helpers for scripts that need a logos-execution-zone
# checkout. Resolution order:
#   1. LOGOS_LEZ_REPO
#   2. LEZ_REPO
#   3. ../logos-execution-zone next to this repository
#   4. $HOME/logos/src/logos-execution-zone legacy layout

resolve_logos_lez_repo() {
  local root_dir="${1:-}"
  local candidate=""

  if [[ -n "${LOGOS_LEZ_REPO:-}" ]]; then
    candidate="$LOGOS_LEZ_REPO"
  elif [[ -n "${LEZ_REPO:-}" ]]; then
    candidate="$LEZ_REPO"
  elif [[ -n "$root_dir" && -d "$root_dir/../logos-execution-zone" ]]; then
    candidate="$root_dir/../logos-execution-zone"
  elif [[ -d "$HOME/logos/src/logos-execution-zone" ]]; then
    candidate="$HOME/logos/src/logos-execution-zone"
  elif [[ -n "$root_dir" ]]; then
    candidate="$root_dir/../logos-execution-zone"
  else
    candidate="$HOME/logos/src/logos-execution-zone"
  fi

  if [[ -d "$candidate" ]]; then
    (cd "$candidate" && pwd)
  else
    printf '%s\n' "$candidate"
  fi
}

require_logos_lez_repo() {
  local root_dir="$1"
  shift

  LOGOS_LEZ_REPO="$(resolve_logos_lez_repo "$root_dir")"
  LEZ_REPO="$LOGOS_LEZ_REPO"
  export LOGOS_LEZ_REPO LEZ_REPO

  local missing=()
  local rel
  for rel in "$@"; do
    if [[ ! -e "$LOGOS_LEZ_REPO/$rel" ]]; then
      missing+=("$rel")
    fi
  done

  if [[ "${#missing[@]}" -gt 0 ]]; then
    {
      echo "LOGOS_LEZ_REPO does not point to a compatible logos-execution-zone checkout: $LOGOS_LEZ_REPO"
      echo "Missing paths:"
      for rel in "${missing[@]}"; do
        echo "  - $rel"
      done
      echo
      echo "Set LOGOS_LEZ_REPO to your fork checkout, for example:"
      echo "  export LOGOS_LEZ_REPO=/path/to/logos-execution-zone"
    } >&2
    return 2
  fi
}

export_default_wallet_home() {
  if [[ -z "${LOGOS_LEZ_REPO:-}" ]]; then
    echo "LOGOS_LEZ_REPO must be resolved before calling export_default_wallet_home" >&2
    return 2
  fi

  export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LOGOS_LEZ_REPO/.wallet-local}"

  local legacy_lez="$HOME/logos/src/logos-execution-zone"
  local legacy_wallet="$legacy_lez/.wallet-local"
  local resolved_lez
  resolved_lez="$(cd "$LOGOS_LEZ_REPO" && pwd -P)"

  if [[ "$NSSA_WALLET_HOME_DIR" == "$legacy_wallet" && "$resolved_lez" != "$legacy_lez" ]]; then
    {
      echo "NSSA_WALLET_HOME_DIR points at the legacy LEZ wallet home:"
      echo "  $NSSA_WALLET_HOME_DIR"
      echo
      echo "But LOGOS_LEZ_REPO points at:"
      echo "  $resolved_lez"
      echo
      echo "This usually means an old shell export is overriding the fork wallet home."
      echo "Use a wallet home for the same LEZ checkout, for example:"
      echo "  unset NSSA_WALLET_HOME_DIR"
      echo "  export LOGOS_LEZ_REPO='$resolved_lez'"
      echo "  export NSSA_WALLET_HOME_DIR=\"\$LOGOS_LEZ_REPO/.wallet-local\""
    } >&2
    return 2
  fi
}

wallet_setup_instructions() {
  cat >&2 <<EOF
How to fix this:

  cd '$LOGOS_LEZ_REPO'
  export LOGOS_LEZ_REPO='$LOGOS_LEZ_REPO'
  export NSSA_WALLET_HOME_DIR='$NSSA_WALLET_HOME_DIR'
  mkdir -p "\$NSSA_WALLET_HOME_DIR"

  # Make sure the wallet binary matches this LEZ checkout.
  cargo install --path "$LOGOS_LEZ_REPO/wallet" --force

  # Run this one interactively. It may ask for the wallet password/setup.
  wallet account new public --label presenter
  wallet check-health

  # For the local-sequencer E2E you also need an initialized private account.
  wallet account new private --label private-balance

If you already have a compatible wallet home, set NSSA_WALLET_HOME_DIR to that
directory before running the demo. Avoid reusing a wallet home from a different
logos-execution-zone checkout; wallet storage formats can drift between forks.
EOF
}

check_wallet_storage_schema() {
  local storage_path="$NSSA_WALLET_HOME_DIR/storage.json"

  if ! command -v python3 >/dev/null 2>&1; then
    echo "warning: python3 not found; skipping wallet storage schema preflight" >&2
    return 0
  fi

  python3 - "$storage_path" <<'PY'
import json
import sys

path = sys.argv[1]
try:
    with open(path, "r", encoding="utf-8") as fh:
        storage = json.load(fh)
except Exception as exc:
    print(f"Wallet storage exists but is not readable JSON: {exc}", file=sys.stderr)
    sys.exit(2)

accounts = storage.get("accounts")
errors = []
if not isinstance(accounts, list):
    errors.append("top-level 'accounts' should be a JSON array")
else:
    for index, entry in enumerate(accounts):
        if not isinstance(entry, dict):
            errors.append(f"accounts[{index}] should be an object")
            continue

        private = entry.get("Private")
        if isinstance(private, dict):
            if "account_id" in private or "identifiers" not in private:
                errors.append(
                    f"accounts[{index}].Private uses an older private-account storage shape "
                    "(expected 'identifiers', found old-style fields)"
                )

        preconfigured = entry.get("Preconfigured")
        if isinstance(preconfigured, dict):
            preconfigured_private = preconfigured.get("Private")
            if isinstance(preconfigured_private, dict):
                if "account_id" in preconfigured_private or "identifier" not in preconfigured_private:
                    errors.append(
                        f"accounts[{index}].Preconfigured.Private uses an older initial private-account storage shape "
                        "(expected 'identifier', found old-style fields)"
                    )

if errors:
    print("Wallet storage schema is incompatible with the selected logos-execution-zone checkout:", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    sys.exit(2)
PY
}

print_sequencer_listener_info() {
  if ! command -v lsof >/dev/null 2>&1; then
    echo "Sequencer listener check skipped: lsof not found."
    return 0
  fi

  local pids
  pids="$(lsof -nP -iTCP:3040 -sTCP:LISTEN -t 2>/dev/null | sort -u || true)"
  if [[ -z "$pids" ]]; then
    echo "No process appears to be listening on TCP port 3040."
    return 0
  fi

  local expected_lez=""
  if [[ -n "${LOGOS_LEZ_REPO:-}" && -d "$LOGOS_LEZ_REPO" ]]; then
    expected_lez="$(cd "$LOGOS_LEZ_REPO" && pwd -P)"
  fi

  local pid cwd cmd
  while IFS= read -r pid; do
    [[ -n "$pid" ]] || continue
    cwd="$(lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | head -n 1 || true)"
    cmd="$(ps -p "$pid" -o command= 2>/dev/null || true)"
    echo "Sequencer listener on 3040:"
    echo "  pid=$pid"
    echo "  cwd=${cwd:-unknown}"
    echo "  command=${cmd:-unknown}"
    if [[ -n "$expected_lez" && -n "$cwd" && "$cwd" != "$expected_lez" ]]; then
      echo "  mismatch: listener cwd does not match LOGOS_LEZ_REPO ($expected_lez)"
    fi
  done <<< "$pids"
}

require_wallet_storage() {
  if [[ -z "${NSSA_WALLET_HOME_DIR:-}" ]]; then
    echo "NSSA_WALLET_HOME_DIR must be set before calling require_wallet_storage" >&2
    return 2
  fi

  if [[ ! -f "$NSSA_WALLET_HOME_DIR/storage.json" ]]; then
    {
      echo "Wallet storage is not initialized at:"
      echo "  $NSSA_WALLET_HOME_DIR/storage.json"
      echo
      echo "The wallet CLI may prompt for setup/password. Non-interactive demos refuse to continue here because that prompt would otherwise be hidden in a log file."
      echo
      echo "Current environment:"
      echo "  LOGOS_LEZ_REPO=$LOGOS_LEZ_REPO"
      echo "  NSSA_WALLET_HOME_DIR=$NSSA_WALLET_HOME_DIR"
      echo
      wallet_setup_instructions
    } >&2
    return 2
  fi

  if ! check_wallet_storage_schema; then
    {
      echo
      echo "This usually means storage.json was created by a wallet binary from another LEZ version."
      echo "Use a wallet home created by the same checkout, or move the incompatible one aside:"
      echo
      echo "  mv '$NSSA_WALLET_HOME_DIR' '$NSSA_WALLET_HOME_DIR.incompatible.$(date -u +%Y%m%dT%H%M%SZ)'"
      echo
      wallet_setup_instructions
    } >&2
    return 2
  fi
}

require_wallet_health() {
  local health_log="$1"

  require_wallet_storage
  if ! wallet check-health < /dev/null > "$health_log" 2>&1; then
    {
      echo "Wallet/sequencer health check failed in non-interactive mode."
      echo "Log file:"
      echo "  $health_log"
      echo
      sed -n '1,80p' "$health_log"
      echo
      echo "Common causes:"
      echo "  - the local sequencer is not running on the expected wallet endpoint"
      echo "  - the sequencer on port 3040 was started from another LEZ checkout"
      echo "  - the wallet home belongs to another logos-execution-zone checkout"
      echo "  - the wallet needs interactive setup/password input"
      echo
      print_sequencer_listener_info
      echo
      wallet_setup_instructions
    } >&2
    return 1
  fi
}

ensure_repo_local_lez_link() {
  local root_dir="$1"
  local local_path="$root_dir/logos"
  local resolved

  if [[ -z "${LOGOS_LEZ_REPO:-}" ]]; then
    echo "LOGOS_LEZ_REPO must be resolved before calling ensure_repo_local_lez_link" >&2
    return 2
  fi

  if [[ ! -d "$LOGOS_LEZ_REPO" ]]; then
    echo "Cannot link missing LOGOS_LEZ_REPO: $LOGOS_LEZ_REPO" >&2
    return 2
  fi

  resolved="$(cd "$LOGOS_LEZ_REPO" && pwd -P)"

  if [[ -L "$local_path" ]]; then
    local current
    current="$(cd "$local_path" && pwd -P)"
    if [[ "$current" != "$resolved" ]]; then
      rm "$local_path"
      ln -s "$resolved" "$local_path"
    fi
    return 0
  fi

  if [[ -e "$local_path" ]]; then
    local current
    current="$(cd "$local_path" && pwd -P)"
    if [[ "$current" == "$resolved" ]]; then
      return 0
    fi

    {
      echo "Repo-local LEZ path dependency exists but does not match LOGOS_LEZ_REPO."
      echo "  repo-local:     $local_path -> $current"
      echo "  LOGOS_LEZ_REPO: $resolved"
      echo
      echo "Move the ignored repo-local 'logos' directory aside, or point LOGOS_LEZ_REPO at it."
      echo "Recommended:"
      echo "  mv '$local_path' '$local_path.legacy'"
      echo "  ln -s '$resolved' '$local_path'"
    } >&2
    return 2
  fi

  ln -s "$resolved" "$local_path"
}
