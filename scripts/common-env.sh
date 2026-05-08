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
