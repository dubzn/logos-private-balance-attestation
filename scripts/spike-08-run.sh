#!/usr/bin/env bash
#
# Spike 08 — runs the deployable LEZ balance-attestation program against a live
# local sequencer and probes whether the LEZ submission path binds the outer
# balance-attestation receipt. See spikes/spike-08-program-chaining/README.md
# for the question and decision matrix.
#
# Required env:
#   LOGOS_LEZ_REPO    path to the local logos-execution-zone checkout
#   NSSA_WALLET_HOME_DIR  wallet home (defaults to $LOGOS_LEZ_REPO/.wallet-local)
#
# Optional env:
#   PROGRAM_BIN       override path to the program ELF
#   THRESHOLD         u128 threshold to register in the gate (default 1)
#   REGISTER_ADMIN_ACCOUNT Public/<id> admin signer for presenter registration
#   INIT_ADMIN_ACCOUNT     Public/<id> fresh admin signer for gate init
#   GATE_ACCOUNT      Public/<id> uninitialized gate state (created if unset)
#   PRESENTER_ACCOUNT Public/<id> presenter (created if unset)
#   PRESENTER_PUBKEY_HEX  64-char hex (defaults to deterministic Spike test bytes)
#   CHAIN_ID_HEX, VERIFIER_ID_HEX, GATE_ID_HEX  optional 64-char hex overrides
#   FABRICATED_NULLIFIER_HEX optional 64-char hex for forged-journal probe
#   ADMIT_WAIT_SECONDS, DUPLICATE_SETTLE_SECONDS block-settle polling windows
#   RISC0_DEV_MODE    defaults to 1 (Spike 08 does not need real proving)
#
# Output: the runner prints the submitted tx hash for each step. The script
# leaves a SPIKE-08 RESULT line at the end summarising what was observed.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LOGOS_LEZ_REPO:-${LEZ_REPO:-}}"
if [[ -z "${LEZ_REPO}" ]]; then
  echo "Set LOGOS_LEZ_REPO (or LEZ_REPO) to the local logos-execution-zone checkout." >&2
  exit 1
fi

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"
export NSSA_WALLET_HOME_DIR="${NSSA_WALLET_HOME_DIR:-$LEZ_REPO/.wallet-local}"

PROGRAM_BIN="${PROGRAM_BIN:-$ROOT_DIR/lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin}"
THRESHOLD="${THRESHOLD:-1}"
PRESENTER_PUBKEY_HEX="${PRESENTER_PUBKEY_HEX:-5555555555555555555555555555555555555555555555555555555555555555}"
CHAIN_ID_HEX="${CHAIN_ID_HEX:-1111111111111111111111111111111111111111111111111111111111111111}"
VERIFIER_ID_HEX="${VERIFIER_ID_HEX:-2222222222222222222222222222222222222222222222222222222222222222}"
GATE_ID_HEX="${GATE_ID_HEX:-3333333333333333333333333333333333333333333333333333333333333333}"
INNER_IMAGE_ID_HEX="${INNER_IMAGE_ID_HEX:-4444444444444444444444444444444444444444444444444444444444444444}"
NULLIFIER_HEX="${NULLIFIER_HEX:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
FABRICATED_NULLIFIER_HEX="${FABRICATED_NULLIFIER_HEX:-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb}"
DEFAULT_PROGRAM_OWNER_B58='"program_owner":"11111111111111111111111111111111"'

extract_account_id() {
  local prefix="$1"
  sed -n "s/.*account_id ${prefix}\/\([^[:space:]]*\).*/\1/p" | tail -n 1
}

extract_gate_data_hex() {
  sed -n 's/.*"data":"\([^"]*\)".*/\1/p' | tail -n 1
}

new_public_account() {
  local label="$1"
  local out
  out="$(wallet account new public)"
  printf '%s\n' "$out" >&2
  local id
  id="$(printf '%s\n' "$out" | extract_account_id Public)"
  if [[ -z "$id" ]]; then
    echo "Could not parse public account id for $label" >&2
    exit 1
  fi
  printf '%s\n' "$id"
}

echo "[0/5] Sequencer + wallet health check"
wallet check-health

echo "[1/5] Building lez-verifier-program (release)"
cargo build --release --manifest-path "$ROOT_DIR/lez-verifier/program/Cargo.toml"

if [[ ! -f "$PROGRAM_BIN" ]]; then
  echo "Program binary not found: $PROGRAM_BIN" >&2
  exit 1
fi

echo "[2/5] Deploying program"
wallet deploy-program "$PROGRAM_BIN"
sleep "${DEPLOY_SETTLE_SECONDS:-15}"

GATE_ACCOUNT_ID="${GATE_ACCOUNT:-}"
if [[ -z "$GATE_ACCOUNT_ID" ]]; then
  echo "[2a/5] Creating gate state account"
  GATE_ACCOUNT_ID="$(new_public_account gate-state)"
fi
REGISTER_ADMIN_ACCOUNT_ID="${REGISTER_ADMIN_ACCOUNT:-${ADMIN_ACCOUNT:-}}"
if [[ -z "$REGISTER_ADMIN_ACCOUNT_ID" ]]; then
  echo "[2b/5] Creating register admin account"
  REGISTER_ADMIN_ACCOUNT_ID="$(new_public_account register-admin)"
fi
INIT_ADMIN_ACCOUNT_ID="${INIT_ADMIN_ACCOUNT:-}"
if [[ -z "$INIT_ADMIN_ACCOUNT_ID" ]]; then
  echo "[2c/5] Creating init admin account"
  INIT_ADMIN_ACCOUNT_ID="$(new_public_account init-admin)"
fi
PRESENTER_ACCOUNT_ID="${PRESENTER_ACCOUNT:-}"
if [[ -z "$PRESENTER_ACCOUNT_ID" ]]; then
  echo "[2d/5] Creating presenter account"
  PRESENTER_ACCOUNT_ID="$(new_public_account presenter)"
fi

PRESENTER_ID_HEX="$(python3 -c "import hashlib; d=b'logos-balance-attestation/v1/presenter'; pk=bytes.fromhex('$PRESENTER_PUBKEY_HEX'); h=hashlib.sha256(); h.update(len(d).to_bytes(8,'little')); h.update(d); h.update(len(pk).to_bytes(8,'little')); h.update(pk); print(h.hexdigest())")"

echo "[3/5] Building Spike 08 runner"
cargo build --release --manifest-path "$ROOT_DIR/spikes/spike-08-program-chaining/lez/runner/Cargo.toml"
RUNNER_BIN="$ROOT_DIR/spikes/spike-08-program-chaining/lez/runner/target/release/spike_08_run"

run_runner() {
  local mode="$1"
  local nullifier_hex="${2:-$NULLIFIER_HEX}"
  local admin_account_id="${3:-$INIT_ADMIN_ACCOUNT_ID}"
  "$RUNNER_BIN" \
    --program-bin "$PROGRAM_BIN" \
    --gate-account "Public/$GATE_ACCOUNT_ID" \
    --admin-account "Public/$admin_account_id" \
    --presenter-account "Public/$PRESENTER_ACCOUNT_ID" \
    --presenter-pubkey-hex "$PRESENTER_PUBKEY_HEX" \
    --chain-id-hex "$CHAIN_ID_HEX" \
    --verifier-id-hex "$VERIFIER_ID_HEX" \
    --gate-id-hex "$GATE_ID_HEX" \
    --threshold "$THRESHOLD" \
    --inner-image-id-hex "$INNER_IMAGE_ID_HEX" \
    --nullifier-hex "$nullifier_hex" \
    --presenter-id-hex "$PRESENTER_ID_HEX" \
    --mode "$mode"
}

get_gate_info() {
  wallet account get --account-id "Public/$GATE_ACCOUNT_ID" 2>/dev/null || true
}

get_presenter_info() {
  wallet account get --account-id "Public/$PRESENTER_ACCOUNT_ID" 2>/dev/null || true
}

count_hex_occurrences() {
  local haystack="$1"
  local needle="$2"
  awk -v haystack="$haystack" -v needle="$needle" '
    BEGIN {
      count = 0
      while ((idx = index(haystack, needle)) > 0) {
        count += 1
        haystack = substr(haystack, idx + length(needle))
      }
      print count
    }
  '
}

gate_nullifier_count() {
  local nullifier_hex="$1"
  local info data
  info="$(get_gate_info)"
  data="$(printf '%s\n' "$info" | extract_gate_data_hex)"
  count_hex_occurrences "$data" "$nullifier_hex"
}

wait_for_nullifier_count_at_least() {
  local nullifier_hex="$1"
  local min_count="$2"
  local deadline=$((SECONDS + ${ADMIT_WAIT_SECONDS:-90}))
  local count=0
  while (( SECONDS < deadline )); do
    count="$(gate_nullifier_count "$nullifier_hex")"
    if (( count >= min_count )); then
      echo "Observed nullifier $nullifier_hex count=$count"
      return 0
    fi
    sleep 5
  done
  echo "Timed out waiting for nullifier $nullifier_hex count >= $min_count (last count=$count)" >&2
  return 1
}

wait_for_duplicate_to_settle() {
  local nullifier_hex="$1"
  local deadline=$((SECONDS + ${DUPLICATE_SETTLE_SECONDS:-45}))
  local count=0
  while (( SECONDS < deadline )); do
    count="$(gate_nullifier_count "$nullifier_hex")"
    if (( count > 1 )); then
      echo "Duplicate nullifier was applied: $nullifier_hex count=$count" >&2
      return 1
    fi
    sleep 5
  done
  echo "Duplicate nullifier did not apply after settle window: $nullifier_hex count=$count"
  return 0
}

echo "[4a/6] register_presenter"
run_runner register-presenter "$NULLIFIER_HEX" "$REGISTER_ADMIN_ACCOUNT_ID" || { echo "register_presenter failed"; exit 1; }

echo "[4b/6] waiting for register_presenter block inclusion"
WAIT_DEADLINE=$((SECONDS + ${REGISTER_WAIT_SECONDS:-120}))
PRESENTER_REGISTERED=0
while (( SECONDS < WAIT_DEADLINE )); do
  PRESENTER_INFO="$(get_presenter_info)"
  PRESENTER_DATA_HEX="$(printf '%s\n' "$PRESENTER_INFO" | extract_gate_data_hex)"
  if [[ "$PRESENTER_DATA_HEX" == "$PRESENTER_PUBKEY_HEX"* ]] \
     && ! printf '%s' "$PRESENTER_INFO" | grep -q "$DEFAULT_PROGRAM_OWNER_B58"; then
    PRESENTER_REGISTERED=1
    break
  fi
  sleep 5
done
if (( PRESENTER_REGISTERED == 0 )); then
  echo "Presenter account did not register pubkey within deadline." >&2
  echo "Last wallet account get output:" >&2
  printf '%s\n' "$PRESENTER_INFO" >&2
  exit 1
fi
printf '%s\n' "$PRESENTER_INFO"

echo "[4c/6] init_gate"
run_runner init-gate || { echo "init_gate failed"; exit 1; }

# Poll the sequencer for the gate-state account until claim resolution applies
# (program_owner flips from default 0...0 to the deployed program id). LEZ
# typically takes 20-45s to include a tx in a block; we cap at 90s.
echo "[4d/6] waiting for init_gate block inclusion"
# After init_gate, claim resolution flips the gate-state account's
# program_owner from [0;32] to BALANCE_ATTESTATION_PROGRAM_ID. The wallet
# JSON encodes program_owner in base58; the all-zero default is the literal
# `11111111111111111111111111111111`, and our program also stamps the magic
# `BAT1` (0x42415431) at the start of `data`. Either signal confirms the
# block landed and the claim resolved.
GATE_STATE_MAGIC_HEX='42415431'
WAIT_DEADLINE=$((SECONDS + ${INIT_WAIT_SECONDS:-120}))
GATE_OWNED=0
while (( SECONDS < WAIT_DEADLINE )); do
  GATE_INFO="$(get_gate_info)"
  if printf '%s' "$GATE_INFO" | grep -q "\"data\":\"$GATE_STATE_MAGIC_HEX" \
     && ! printf '%s' "$GATE_INFO" | grep -q "$DEFAULT_PROGRAM_OWNER_B58"; then
    GATE_OWNED=1
    break
  fi
  sleep 5
done
if (( GATE_OWNED == 0 )); then
  echo "Gate state account did not transition to program-owned within deadline." >&2
  echo "Last wallet account get output:" >&2
  printf '%s\n' "$GATE_INFO" >&2
  exit 1
fi
printf '%s\n' "$GATE_INFO"

echo "[5a/6] admit (real-shape journal)"
if run_runner admit "$NULLIFIER_HEX" && wait_for_nullifier_count_at_least "$NULLIFIER_HEX" 1; then
  ADMIT_REAL=ok
else
  ADMIT_REAL=fail
fi

echo "[5b/6] duplicate admit after block inclusion"
if [[ "$ADMIT_REAL" == "ok" ]]; then
  if run_runner admit "$NULLIFIER_HEX"; then
    if wait_for_duplicate_to_settle "$NULLIFIER_HEX"; then
      ADMIT_DUPLICATE=rejected-or-not-applied
    else
      ADMIT_DUPLICATE=accepted
    fi
  else
    ADMIT_DUPLICATE=rejected-or-not-applied
  fi
else
  ADMIT_DUPLICATE=not-run
fi

echo "[5c/6] admit-fabricated (no real outer receipt; arbitrary journal)"
if run_runner admit-fabricated "$FABRICATED_NULLIFIER_HEX" && wait_for_nullifier_count_at_least "$FABRICATED_NULLIFIER_HEX" 1; then
  ADMIT_FAB=ok
else
  ADMIT_FAB=fail
fi

echo
echo "------------------------------------------------------------"
echo "ADMIT real-shape:       $ADMIT_REAL"
echo "ADMIT duplicate settled:$ADMIT_DUPLICATE"
echo "ADMIT fabricated:       $ADMIT_FAB"
if [[ "$ADMIT_REAL" == "ok" && "$ADMIT_DUPLICATE" == "rejected-or-not-applied" && "$ADMIT_FAB" == "fail" ]]; then
  echo "SPIKE-08 RESULT: Best (sequencer rejected fabricated journal — receipt-binding works)"
elif [[ "$ADMIT_REAL" == "ok" && "$ADMIT_DUPLICATE" == "rejected-or-not-applied" && "$ADMIT_FAB" == "ok" ]]; then
  echo "SPIKE-08 RESULT: Workable (sequencer admits any journal; host pre-verification is the trust seat)"
elif [[ "$ADMIT_DUPLICATE" == "accepted" ]]; then
  echo "SPIKE-08 RESULT: Blocked (duplicate nullifier applied after block inclusion)"
else
  echo "SPIKE-08 RESULT: Blocked (real-shape admit failed; debug from sequencer logs)"
fi
echo "------------------------------------------------------------"
