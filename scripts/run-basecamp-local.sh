#!/usr/bin/env bash
# Build/install the Basecamp ui_qml plugin and launch a local Basecamp dev app.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

BASECAMP_REPO="${LOGOS_BASECAMP_REPO:-}"
BASECAMP_USER_DIR="${BASECAMP_USER_DIR:-/Users/dub/Desktop/logos/basecamp-balance-attestation-user}"
SKIP_BUILD=0
SKIP_DELIVERY=0
RESET=0
RISC0_MODE="${RISC0_DEV_MODE:-0}"
DELIVERY_MODULE_FLAKE="${DELIVERY_MODULE_FLAKE:-github:logos-co/logos-delivery-module/v0.1.3#install}"

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/run-basecamp-local.sh [--skip-build] [--reset] [--dev-mode|--real-prover]
                                [--basecamp-repo <path>] [--user-dir <path>]

options:
  --skip-build           Reuse apps/basecamp/result instead of running nix build.
  --skip-delivery        Do not install the delivery_module dependency into
                         the local Basecamp user dir.
  --reset                Kill existing local Basecamp/ui-host/logos_host_qt processes first.
  --dev-mode             Launch with RISC0_DEV_MODE=1.
  --real-prover          Launch with RISC0_DEV_MODE=0. Default.
  --basecamp-repo PATH   Basecamp checkout. Defaults to LOGOS_BASECAMP_REPO,
                         sibling ../logos-basecamp, then /Users/dub/Desktop/logos/logos-basecamp.
  --user-dir PATH        Basecamp user data dir. Defaults to
                         /Users/dub/Desktop/logos/basecamp-balance-attestation-user.

notes:
  The app runs in the foreground. Keep this terminal open while using Basecamp.
  The balance_attestation plugin depends on delivery_module for the Logos
  Delivery panel. By default this script builds and installs that dependency
  into <user-dir>/modules/delivery_module before launching Basecamp.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --skip-delivery)
      SKIP_DELIVERY=1
      shift
      ;;
    --reset)
      RESET=1
      shift
      ;;
    --dev-mode)
      RISC0_MODE=1
      shift
      ;;
    --real-prover)
      RISC0_MODE=0
      shift
      ;;
    --basecamp-repo)
      BASECAMP_REPO="${2:-}"
      shift 2
      ;;
    --user-dir)
      BASECAMP_USER_DIR="${2:-}"
      shift 2
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

if [[ -z "$BASECAMP_REPO" ]]; then
  if [[ -d "$ROOT_DIR/../logos-basecamp" ]]; then
    BASECAMP_REPO="$(cd "$ROOT_DIR/../logos-basecamp" && pwd)"
  elif [[ -d "/Users/dub/Desktop/logos/logos-basecamp" ]]; then
    BASECAMP_REPO="/Users/dub/Desktop/logos/logos-basecamp"
  else
    BASECAMP_REPO="$ROOT_DIR/../logos-basecamp"
  fi
fi

require_logos_lez_repo "$ROOT_DIR" Cargo.toml wallet nssa/core
export_default_wallet_home
export_default_risc0_recursion_cache "$ROOT_DIR"

if [[ ! -f "$BASECAMP_REPO/run-dev.sh" ]]; then
  {
    echo "Basecamp run-dev.sh not found:"
    echo "  $BASECAMP_REPO/run-dev.sh"
    echo
    echo "Set LOGOS_BASECAMP_REPO or pass --basecamp-repo <path>."
  } >&2
  exit 2
fi

if [[ "$RESET" == "1" ]]; then
  set +e
  pkill -f 'LogosBasecamp|ui-host|logos_host_qt'
  set -e
  sleep 1
fi

if [[ "$SKIP_BUILD" == "1" ]]; then
  "$ROOT_DIR/scripts/check-basecamp-package.sh" --skip-build
else
  "$ROOT_DIR/scripts/check-basecamp-package.sh"
fi

PLUGIN_SRC="$ROOT_DIR/apps/basecamp/result/plugins/balance_attestation"
PLUGIN_DEST="$BASECAMP_USER_DIR/plugins/balance_attestation"
DELIVERY_DEST="$BASECAMP_USER_DIR/modules/delivery_module"

rm -rf "$PLUGIN_DEST"
mkdir -p "$BASECAMP_USER_DIR/plugins"
cp -R "$PLUGIN_SRC" "$BASECAMP_USER_DIR/plugins/"
chmod -R u+w "$PLUGIN_DEST"

if [[ "$SKIP_DELIVERY" != "1" ]]; then
  command -v nix >/dev/null || {
    echo "nix is required to install delivery_module; pass --skip-delivery to skip it" >&2
    exit 2
  }
  DELIVERY_INSTALL="$(nix build --print-out-paths "$DELIVERY_MODULE_FLAKE")"
  DELIVERY_SRC="$DELIVERY_INSTALL/modules/delivery_module"
  if [[ ! -d "$DELIVERY_SRC" ]]; then
    echo "delivery_module install tree not found at $DELIVERY_SRC" >&2
    exit 1
  fi
  rm -rf "$DELIVERY_DEST"
  mkdir -p "$BASECAMP_USER_DIR/modules"
  cp -R "$DELIVERY_SRC" "$BASECAMP_USER_DIR/modules/"
  chmod -R u+w "$DELIVERY_DEST"
fi

cat <<EOF

Basecamp local launch
  BALANCE_ATTEST_REPO=$ROOT_DIR
  LOGOS_LEZ_REPO=$LOGOS_LEZ_REPO
  NSSA_WALLET_HOME_DIR=$NSSA_WALLET_HOME_DIR
  LOGOS_BASECAMP_REPO=$BASECAMP_REPO
  BASECAMP_USER_DIR=$BASECAMP_USER_DIR
  RISC0_DEV_MODE=$RISC0_MODE
  DELIVERY_MODULE=$([[ "$SKIP_DELIVERY" == "1" ]] && echo "skipped" || echo "$DELIVERY_DEST")

Keep this terminal open while using Basecamp.
EOF

cd "$BASECAMP_REPO"
BALANCE_ATTEST_REPO="$ROOT_DIR" \
LOGOS_BALANCE_ATTESTATION_ROOT="$ROOT_DIR" \
LOGOS_LEZ_REPO="$LOGOS_LEZ_REPO" \
LEZ_REPO="$LOGOS_LEZ_REPO" \
NSSA_WALLET_HOME_DIR="$NSSA_WALLET_HOME_DIR" \
RISC0_DEV_MODE="$RISC0_MODE" \
exec bash ./run-dev.sh --user-dir "$BASECAMP_USER_DIR"
