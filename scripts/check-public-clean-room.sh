#!/usr/bin/env bash
# Public-repo hygiene and clean-room checks.
#
# Default mode is static and fast. Add --with-tests for workspace tests,
# --with-lez for checks that need a local logos-execution-zone checkout, and
# --with-basecamp for the Nix-built Basecamp ui_qml package.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/common-env.sh"

WITH_TESTS=0
WITH_LEZ=0
WITH_BASECAMP=0
ALLOW_DIRTY="${ALLOW_DIRTY:-0}"

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/check-public-clean-room.sh [--with-tests] [--with-lez] [--with-basecamp]

env:
  ALLOW_DIRTY=1       Do not fail when the worktree has local changes.
  LOGOS_LEZ_REPO      Required by --with-lez unless ../logos-execution-zone exists.

checks:
  default             Git hygiene, forbidden files, secret-like strings, shell syntax.
  --with-tests        cargo fmt --check + cargo test --workspace.
  --with-lez          Resolve LEZ checkout, sync ignored repo-local path dependency,
                      check RISC Zero version, and run LEZ compatibility tests.
  --with-basecamp     Build and inspect the Basecamp ui_qml package.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --with-tests)
      WITH_TESTS=1
      shift
      ;;
    --with-lez)
      WITH_LEZ=1
      shift
      ;;
    --with-basecamp)
      WITH_BASECAMP=1
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

step() {
  printf '\n\033[1;36m== %s ==\033[0m\n' "$1"
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

grep_pipe() {
  local pattern="$1"
  grep -E "$pattern" || true
}

require_empty() {
  local label="$1"
  local value="$2"
  if [[ -n "$value" ]]; then
    printf '%s\n%s\n' "$label" "$value" >&2
    exit 1
  fi
}

cd "$ROOT_DIR"

step "1/5 Git state"
git rev-parse --is-inside-work-tree >/dev/null
if [[ "$ALLOW_DIRTY" != "1" ]]; then
  dirty="$(git status --porcelain --untracked-files=all)"
  require_empty "Worktree should be clean for a public clean-room check:" "$dirty"
else
  echo "ALLOW_DIRTY=1 set; skipping clean worktree assertion."
fi

step "2/5 Forbidden tracked/untracked artifacts"
tracked_forbidden="$(
  git ls-files | grep_pipe '(^|/)(witness\.json|storage\.json|wallet_config\.json|accounts\.env|\.env|\.wallet-local)(/|$)|(^|/)(\.demo-runs|\.spike-results|target)/'
)"
require_empty "Forbidden tracked artifacts found:" "$tracked_forbidden"

untracked_forbidden="$(
  git status --porcelain --untracked-files=all \
    | grep_pipe '(^|[[:space:]])(witness\.json|storage\.json|wallet_config\.json|accounts\.env|\.env|\.wallet-local)(/|$)|(^|[[:space:]])(\.demo-runs|\.spike-results|target)/'
)"
require_empty "Forbidden untracked artifacts found:" "$untracked_forbidden"

secret_hits="$(
  git grep -n -E 'Private/[1-9A-HJ-NP-Za-km-z]{20,}|BEGIN [A-Z ]*PRIVATE KEY|seed phrase|mnemonic phrase|NSSA_PRIVATE' -- . ':!scripts/check-public-clean-room.sh' || true
)"
require_empty "Secret-like tracked strings found:" "$secret_hits"

step "3/5 Public docs smoke"
[[ -f scripts/env.example ]] || fail "missing scripts/env.example"
grep -q 'Workable / host-preverified' README.md docs/ONCHAIN_PATH_DECISION.md \
  || fail "README/docs should explicitly state the Workable / host-preverified on-chain path"
grep -q 'RISC0_DEV_MODE=0' README.md docs/BENCHMARKS.md docs/PRIZE_CHECKLIST.md \
  || fail "README/docs should mention RISC0_DEV_MODE=0 final-demo mode"

step "4/5 Shell syntax"
while IFS= read -r script; do
  bash -n "$script"
done < <(find scripts -maxdepth 1 -type f -name '*.sh' | sort)
echo "Shell syntax ok."

if [[ "$WITH_TESTS" == "1" ]]; then
  step "5/5 Workspace tests"
  cargo fmt --all --check
  cargo test --workspace
else
  step "5/5 Workspace tests"
  echo "Skipped. Re-run with --with-tests to include cargo fmt/test."
fi

if [[ "$WITH_LEZ" == "1" ]]; then
  step "LEZ checks"
  require_logos_lez_repo "$ROOT_DIR" Cargo.toml
  lez_wallet_crate_rel_path >/dev/null
  lez_core_crate_rel_path >/dev/null
  export_default_wallet_home
  ensure_repo_local_lez_link "$ROOT_DIR"
  echo "LOGOS_LEZ_REPO=$LOGOS_LEZ_REPO"
  echo "NSSA_WALLET_HOME_DIR=$NSSA_WALLET_HOME_DIR"
  echo "LEE_WALLET_HOME_DIR=$LEE_WALLET_HOME_DIR"
  scripts/check-risc0-version.sh
  scripts/m2-check-lez-commitment-compat.sh
  cargo test --manifest-path lez-verifier/program/Cargo.toml
else
  echo
  echo "LEZ checks skipped. Re-run with --with-lez after setting LOGOS_LEZ_REPO."
fi

if [[ "$WITH_BASECAMP" == "1" ]]; then
  step "Basecamp package"
  scripts/check-basecamp-package.sh
else
  echo
  echo "Basecamp package check skipped. Re-run with --with-basecamp to include nix build."
fi

echo
echo "Public clean-room checks passed."
