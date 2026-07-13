#!/usr/bin/env bash
# Prepare two local Basecamp user directories for a Logos Delivery send/receive QA pass.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

BASECAMP_REPO="${LOGOS_BASECAMP_REPO:-}"
SENDER_USER_DIR="${SENDER_USER_DIR:-/Users/dub/Desktop/logos/basecamp-balance-attestation-sender}"
RECEIVER_USER_DIR="${RECEIVER_USER_DIR:-/Users/dub/Desktop/logos/basecamp-balance-attestation-receiver}"
DELIVERY_MODULE_FLAKE="${DELIVERY_MODULE_FLAKE:-$ROOT_DIR/apps/basecamp#delivery-install}"
OUT_DIR="${OUT_DIR:-$ROOT_DIR/.demo-runs/basecamp-delivery-pair/$(date -u +%Y%m%dT%H%M%SZ)}"
RISC0_MODE="${RISC0_DEV_MODE:-0}"
SKIP_BUILD=0
RESET=0

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/prepare-basecamp-delivery-pair.sh [--skip-build] [--reset]
                                             [--dev-mode|--real-prover]
                                             [--basecamp-repo <path>]
                                             [--sender-dir <path>]
                                             [--receiver-dir <path>]

options:
  --skip-build          Reuse apps/basecamp/result instead of running nix build.
  --reset               Remove the selected sender/receiver user dirs first.
  --dev-mode            Write launchers with RISC0_DEV_MODE=1.
  --real-prover         Write launchers with RISC0_DEV_MODE=0. Default.
  --basecamp-repo PATH  Basecamp checkout. Defaults to LOGOS_BASECAMP_REPO,
                        sibling ../logos-basecamp, then /Users/dub/Desktop/logos/logos-basecamp.
  --sender-dir PATH     Basecamp user dir for the sender instance.
  --receiver-dir PATH   Basecamp user dir for the receiver instance.

This script does not launch Basecamp. It prepares both user dirs with:
  - plugins/balance_attestation
  - modules/delivery_module
and writes launch-sender.sh / launch-receiver.sh under OUT_DIR.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
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
    --sender-dir)
      SENDER_USER_DIR="${2:-}"
      shift 2
      ;;
    --receiver-dir)
      RECEIVER_USER_DIR="${2:-}"
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

step() {
  printf '\n\033[1;36m== %s ==\033[0m\n' "$1"
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

resolve_basecamp_repo() {
  if [[ -n "$BASECAMP_REPO" ]]; then
    return 0
  fi

  if [[ -d "$ROOT_DIR/../logos-basecamp" ]]; then
    BASECAMP_REPO="$(cd "$ROOT_DIR/../logos-basecamp" && pwd)"
  elif [[ -d "/Users/dub/Desktop/logos/logos-basecamp" ]]; then
    BASECAMP_REPO="/Users/dub/Desktop/logos/logos-basecamp"
  else
    BASECAMP_REPO="$ROOT_DIR/../logos-basecamp"
  fi
}

install_tree() {
  local user_dir="$1"
  local role="$2"
  local plugin_dest="$user_dir/plugins/balance_attestation"
  local delivery_dest="$user_dir/modules/delivery_module"

  if [[ "$RESET" == "1" ]]; then
    rm -rf "$user_dir"
  fi

  rm -rf "$plugin_dest" "$delivery_dest"
  mkdir -p "$user_dir/plugins" "$user_dir/modules"
  cp -R "$PLUGIN_SRC" "$user_dir/plugins/"
  cp -R "$DELIVERY_SRC" "$user_dir/modules/"
  chmod -R u+w "$plugin_dest" "$delivery_dest"

  printf '%s user dir prepared: %s\n' "$role" "$user_dir"
}

write_launcher() {
  local path="$1"
  local user_dir="$2"

  cat > "$path" <<EOF
#!/usr/bin/env bash
set -euo pipefail

cd "$BASECAMP_REPO"
export BALANCE_ATTEST_REPO="$ROOT_DIR"
export LOGOS_BALANCE_ATTESTATION_ROOT="$ROOT_DIR"
export LOGOS_LEZ_REPO="$LOGOS_LEZ_REPO"
export LEZ_REPO="$LOGOS_LEZ_REPO"
export NSSA_WALLET_HOME_DIR="$NSSA_WALLET_HOME_DIR"
export LEE_WALLET_HOME_DIR="$LEE_WALLET_HOME_DIR"
export RISC0_DEV_MODE="$RISC0_MODE"

if [[ -x ./result/bin/LogosBasecamp ]]; then
  exec ./result/bin/LogosBasecamp --user-dir "$user_dir"
else
  exec bash ./run-dev.sh --user-dir "$user_dir"
fi
EOF
  chmod +x "$path"
}

resolve_basecamp_repo

require_logos_lez_repo "$ROOT_DIR" Cargo.toml
lez_wallet_crate_rel_path >/dev/null
lez_core_crate_rel_path >/dev/null
export_default_wallet_home
export_default_risc0_recursion_cache "$ROOT_DIR"

if [[ ! -x "$BASECAMP_REPO/result/bin/LogosBasecamp" && ! -f "$BASECAMP_REPO/run-dev.sh" ]]; then
  fail "Basecamp launcher not found; run 'nix build' in $BASECAMP_REPO"
fi
command -v nix >/dev/null || fail "nix is required to build/install delivery_module"

step "1/4 Build and inspect balance_attestation package"
if [[ "$SKIP_BUILD" == "1" ]]; then
  "$ROOT_DIR/scripts/check-basecamp-package.sh" --skip-build
else
  "$ROOT_DIR/scripts/check-basecamp-package.sh"
fi

PLUGIN_SRC="$ROOT_DIR/apps/basecamp/result/plugins/balance_attestation"
[[ -d "$PLUGIN_SRC" ]] || fail "missing plugin install tree: $PLUGIN_SRC"

step "2/4 Build delivery_module install tree"
DELIVERY_INSTALL="$(nix build --print-out-paths "$DELIVERY_MODULE_FLAKE")"
DELIVERY_SRC="$DELIVERY_INSTALL/modules/delivery_module"
[[ -d "$DELIVERY_SRC" ]] || fail "delivery_module install tree not found: $DELIVERY_SRC"

step "3/4 Prepare sender and receiver user dirs"
install_tree "$SENDER_USER_DIR" "sender"
install_tree "$RECEIVER_USER_DIR" "receiver"

step "4/4 Write launchers and QA notes"
mkdir -p "$OUT_DIR"
SENDER_LAUNCH="$OUT_DIR/launch-sender.sh"
RECEIVER_LAUNCH="$OUT_DIR/launch-receiver.sh"
REPORT="$OUT_DIR/report.md"
write_launcher "$SENDER_LAUNCH" "$SENDER_USER_DIR"
write_launcher "$RECEIVER_LAUNCH" "$RECEIVER_USER_DIR"

cat > "$REPORT" <<EOF
# Basecamp Delivery Pair

Prepared two Basecamp user directories for LP-0005 Logos Delivery QA.

| Role | User dir | Launcher |
| --- | --- | --- |
| Sender | \`$SENDER_USER_DIR\` | \`$SENDER_LAUNCH\` |
| Receiver | \`$RECEIVER_USER_DIR\` | \`$RECEIVER_LAUNCH\` |

## Environment

- Repository: \`$ROOT_DIR\`
- Basecamp: \`$BASECAMP_REPO\`
- LEZ checkout: \`$LOGOS_LEZ_REPO\`
- Wallet home: \`$NSSA_WALLET_HOME_DIR\`
- Delivery module: \`$DELIVERY_SRC\`
- RISC0_DEV_MODE: \`$RISC0_MODE\`

## Manual QA

1. Terminal A:

   \`\`\`sh
   "$SENDER_LAUNCH"
   \`\`\`

2. Terminal B:

   \`\`\`sh
   "$RECEIVER_LAUNCH"
   \`\`\`

3. In both Basecamp windows, load \`balance_attestation\`.
4. Receiver: **Create node**, then **Subscribe**.
5. Sender: generate/verify proof, then **Create node**, **Subscribe**, **Send proof**.
6. Receiver: wait for **Delivery Msg**, then press **Verify received**.

Expected receiver verification:

\`\`\`json
{"status":"ok"}
\`\`\`

Do not publish \`witness.json\`. Delivery messages carry the public proof envelope.
EOF

cat "$REPORT"
