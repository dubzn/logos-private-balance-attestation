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

lez_core_crate_rel_path() {
  if [[ -z "${LOGOS_LEZ_REPO:-}" ]]; then
    echo "LOGOS_LEZ_REPO must be resolved before calling lez_core_crate_rel_path" >&2
    return 2
  fi

  if [[ -d "$LOGOS_LEZ_REPO/nssa/core" ]]; then
    printf '%s\n' "nssa/core"
    return 0
  fi

  if [[ -d "$LOGOS_LEZ_REPO/lee/state_machine/core" ]]; then
    printf '%s\n' "lee/state_machine/core"
    return 0
  fi

  {
    echo "Could not find the LEZ state-machine core crate in:"
    echo "  $LOGOS_LEZ_REPO"
    echo
    echo "Expected one of:"
    echo "  nssa/core"
    echo "  lee/state_machine/core"
  } >&2
  return 2
}

lez_core_crate_package_name() {
  local rel
  rel="$(lez_core_crate_rel_path)" || return $?

  case "$rel" in
    nssa/core) printf '%s\n' "nssa_core" ;;
    lee/state_machine/core) printf '%s\n' "lee_core" ;;
    *)
      echo "Unknown LEZ core crate path: $rel" >&2
      return 2
      ;;
  esac
}

lez_state_crate_rel_path() {
  if [[ -z "${LOGOS_LEZ_REPO:-}" ]]; then
    echo "LOGOS_LEZ_REPO must be resolved before calling lez_state_crate_rel_path" >&2
    return 2
  fi

  if [[ -d "$LOGOS_LEZ_REPO/nssa" ]]; then
    printf '%s\n' "nssa"
    return 0
  fi

  if [[ -d "$LOGOS_LEZ_REPO/lee/state_machine" ]]; then
    printf '%s\n' "lee/state_machine"
    return 0
  fi

  {
    echo "Could not find the LEZ state-machine crate in:"
    echo "  $LOGOS_LEZ_REPO"
    echo
    echo "Expected one of:"
    echo "  nssa"
    echo "  lee/state_machine"
  } >&2
  return 2
}

lez_state_crate_package_name() {
  local rel
  rel="$(lez_state_crate_rel_path)" || return $?

  case "$rel" in
    nssa) printf '%s\n' "nssa" ;;
    lee/state_machine) printf '%s\n' "lee" ;;
    *)
      echo "Unknown LEZ state-machine crate path: $rel" >&2
      return 2
      ;;
  esac
}

lez_wallet_crate_rel_path() {
  if [[ -z "${LOGOS_LEZ_REPO:-}" ]]; then
    echo "LOGOS_LEZ_REPO must be resolved before calling lez_wallet_crate_rel_path" >&2
    return 2
  fi

  if [[ -d "$LOGOS_LEZ_REPO/wallet" ]]; then
    printf '%s\n' "wallet"
    return 0
  fi

  if [[ -d "$LOGOS_LEZ_REPO/lez/wallet" ]]; then
    printf '%s\n' "lez/wallet"
    return 0
  fi

  {
    echo "Could not find the LEZ wallet crate in:"
    echo "  $LOGOS_LEZ_REPO"
    echo
    echo "Expected one of:"
    echo "  wallet"
    echo "  lez/wallet"
  } >&2
  return 2
}

export_default_wallet_home() {
  if [[ -z "${LOGOS_LEZ_REPO:-}" ]]; then
    echo "LOGOS_LEZ_REPO must be resolved before calling export_default_wallet_home" >&2
    return 2
  fi

  if [[ -z "${NSSA_WALLET_HOME_DIR:-}" && -n "${LEE_WALLET_HOME_DIR:-}" ]]; then
    export NSSA_WALLET_HOME_DIR="$LEE_WALLET_HOME_DIR"
  fi

  export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LOGOS_LEZ_REPO/.wallet-local}"
  export LEE_WALLET_HOME_DIR="${LEE_WALLET_HOME_DIR:-$NSSA_WALLET_HOME_DIR}"

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
      echo "  unset LEE_WALLET_HOME_DIR"
      echo "  export LOGOS_LEZ_REPO='$resolved_lez'"
      echo "  export NSSA_WALLET_HOME_DIR=\"\$LOGOS_LEZ_REPO/.wallet-local\""
      echo "  export LEE_WALLET_HOME_DIR=\"\$NSSA_WALLET_HOME_DIR\""
    } >&2
    return 2
  fi
}

export_default_risc0_recursion_cache() {
  local root_dir="$1"
  local expected_sha="744b999f0a35b3c86753311c7efb2a0054be21727095cf105af6ee7d3f4d8849"
  local url="https://risc0-artifacts.s3.us-west-2.amazonaws.com/zkr/${expected_sha}.zip"
  local cache_dir="$root_dir/.risc0-cache"
  local recursion_zkr="$root_dir/.risc0-cache/recursion_zkr.zip"

  if [[ -z "${RECURSION_SRC_PATH:-}" && -f "$recursion_zkr" ]]; then
    export RECURSION_SRC_PATH="$recursion_zkr"
    return 0
  fi

  if [[ -n "${RECURSION_SRC_PATH:-}" || -f "$recursion_zkr" ]]; then
    return 0
  fi

  if ! command -v curl >/dev/null 2>&1; then
    return 0
  fi

  mkdir -p "$cache_dir"
  local tmp="$recursion_zkr.tmp"
  echo "Caching RISC Zero recursion artifact at $recursion_zkr"
  if ! curl -L --fail --retry 3 --output "$tmp" "$url"; then
    rm -f "$tmp"
    return 0
  fi

  local actual_sha=""
  if command -v shasum >/dev/null 2>&1; then
    actual_sha="$(shasum -a 256 "$tmp" | awk '{print $1}')"
  elif command -v sha256sum >/dev/null 2>&1; then
    actual_sha="$(sha256sum "$tmp" | awk '{print $1}')"
  fi

  if [[ "$actual_sha" != "$expected_sha" ]]; then
    rm -f "$tmp"
    {
      echo "warning: downloaded RISC Zero recursion artifact failed SHA-256 check"
      echo "  expected: $expected_sha"
      echo "  actual:   ${actual_sha:-unknown}"
    } >&2
    return 0
  fi

  mv "$tmp" "$recursion_zkr"
  export RECURSION_SRC_PATH="$recursion_zkr"
}

export_macos_python_framework_rustflags() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 0
  fi

  local framework_dir="/Library/Developer/CommandLineTools/Library/Frameworks"
  if [[ ! -d "$framework_dir/Python3.framework" ]]; then
    return 0
  fi

  local rpath_arg="-C link-arg=-Wl,-rpath,$framework_dir"
  if [[ " ${RUSTFLAGS:-} " != *" $rpath_arg "* ]]; then
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }$rpath_arg"
  fi
}

wallet_setup_instructions() {
  local wallet_rel="wallet"
  if [[ -n "${LOGOS_LEZ_REPO:-}" && -d "$LOGOS_LEZ_REPO/lez/wallet" ]]; then
    wallet_rel="lez/wallet"
  fi

  cat >&2 <<EOF
How to fix this:

  cd '$LOGOS_LEZ_REPO'
  export LOGOS_LEZ_REPO='$LOGOS_LEZ_REPO'
  export NSSA_WALLET_HOME_DIR='$NSSA_WALLET_HOME_DIR'
  export LEE_WALLET_HOME_DIR='$LEE_WALLET_HOME_DIR'
  mkdir -p "\$NSSA_WALLET_HOME_DIR"

  # Make sure the wallet binary matches this LEZ checkout.
  cargo install --path "$LOGOS_LEZ_REPO/$wallet_rel" --force

  # Run this one interactively. It may ask for the wallet password/setup.
  wallet account new public --label presenter
  wallet check-health

  # For the local-sequencer E2E you also need an initialized private account.
  wallet account new private --label private-balance

If you already have a compatible wallet home, set NSSA_WALLET_HOME_DIR to that
directory before running the demo. For latest LEZ checkouts, LEE_WALLET_HOME_DIR
is exported to the same path. Avoid reusing a wallet home from a different
logos-execution-zone checkout; wallet storage formats can drift between forks.
EOF
}

wallet_macos_python_rpath_instructions() {
  cat >&2 <<'EOF'

macOS Python framework fix:

  The wallet binary was linked against @rpath/Python3.framework, but macOS did
  not know where to resolve that framework from. If the framework exists under
  CommandLineTools, add that rpath to the installed wallet binary:

    install_name_tool -add_rpath /Library/Developer/CommandLineTools/Library/Frameworks "$(command -v wallet)"

  Then re-run:

    wallet --help
    wallet check-health

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

errors = []
if isinstance(storage.get("key_chain"), dict):
    # Current LEZ wallet storage shape:
    # {
    #   "key_chain": { "accounts": [...] },
    #   "labels": { ... },
    #   "last_synced_block": N
    # }
    key_chain = storage["key_chain"]
    accounts = key_chain.get("accounts")
    if not isinstance(accounts, list):
        errors.append("top-level 'key_chain.accounts' should be a JSON array")
    if "last_synced_block" not in storage:
        errors.append("top-level 'last_synced_block' is missing")
elif isinstance(storage.get("accounts"), list):
    # Older wallet storage shape used by earlier local LEZ checkouts.
    # Keep accepting it so the preflight can produce targeted migration errors
    # for the specific private-account fields that drifted.
    accounts = storage["accounts"]
else:
    errors.append(
        "wallet storage has an unknown shape; expected current 'key_chain.accounts' "
        "or legacy top-level 'accounts'"
    )
    accounts = []

for index, entry in enumerate(accounts):
    if not isinstance(entry, dict):
        errors.append(f"accounts[{index}] should be an object")
        continue

    private = entry.get("Private")
    if isinstance(private, dict):
        if "identifiers" in private:
            errors.append(
                f"accounts[{index}].Private uses the pre-PrivateAccountKind storage shape "
                "(found 'identifiers', expected current key-chain account data)"
            )

        value = private.get("data", {}).get("value")
        if isinstance(value, list) and len(value) > 1 and isinstance(value[1], list):
            for account_index, pair in enumerate(value[1]):
                if (
                    isinstance(pair, list)
                    and pair
                    and isinstance(pair[0], int)
                ):
                    errors.append(
                        f"accounts[{index}].Private.data.value[1][{account_index}] "
                        "uses an integer identifier; expected a PrivateAccountKind object"
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
      if grep -q "@rpath/Python3.framework" "$health_log" 2>/dev/null; then
        wallet_macos_python_rpath_instructions
      fi
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
