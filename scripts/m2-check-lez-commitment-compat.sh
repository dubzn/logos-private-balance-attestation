#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOGOS_LEZ_REPO="${LOGOS_LEZ_REPO:-$HOME/logos/src/logos-execution-zone}"
RESULT_DIR="${RESULT_DIR:-$REPO_ROOT/.spike-results/m2-commitment-compat}"
TIMESTAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
REPORT="$RESULT_DIR/$TIMESTAMP.md"
OUTPUT_JSON="$RESULT_DIR/$TIMESTAMP.json"

if [[ ! -d "$LOGOS_LEZ_REPO/nssa/core" ]]; then
  echo "LOGOS_LEZ_REPO does not point to a logos-execution-zone checkout: $LOGOS_LEZ_REPO" >&2
  exit 2
fi

mkdir -p "$RESULT_DIR"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/balance-attest-m2.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

duration() {
  local start="$1"
  local end
  end="$(date +%s)"
  printf "%02d:%02d:%02d" $(((end - start) / 3600)) $((((end - start) % 3600) / 60)) $(((end - start) % 60))
}

status="ok"
started="$(date +%s)"

mkdir -p "$TMP_DIR/src"
cat > "$TMP_DIR/Cargo.toml" <<EOF
[package]
name = "balance_attest_m2_commitment_compat"
version = "0.1.0"
edition = "2021"

[dependencies]
attestation-core = { path = "$REPO_ROOT/crates/attestation-core" }
nssa_core = { path = "$LOGOS_LEZ_REPO/nssa/core", features = ["host"] }
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
EOF

cat > "$TMP_DIR/src/main.rs" <<'EOF'
use attestation_core::{
    compute_lez_membership_root, derive_lez_private_account_commitment,
    hash_lez_commitment_leaf, Digest32, HexBytes, LezMembershipProof,
    LezPrivateAccountCommitmentInput,
};
use nssa_core::{
    account::{Account, Data, Nonce},
    compute_digest_for_path, Commitment, MembershipProof, NullifierPublicKey,
};
use serde::Serialize;

#[derive(Serialize)]
struct CaseReport {
    name: &'static str,
    commitment_match: bool,
    leaf_hash_match: bool,
    membership_root_match: bool,
    commitment_hex: String,
    leaf_hash_hex: String,
    membership_root_hex: String,
}

fn main() {
    let cases = vec![
        compare_case(
            "dummy-default",
            [0; 32],
            [0; 8],
            0,
            0,
            Vec::new(),
            0,
            vec![],
        ),
        compare_case(
            "documented-fixture",
            [0x07; 32],
            [1, 2, 3, 4, 5, 6, 7, 8],
            42,
            123_456,
            b"compat fixture".to_vec(),
            5,
            vec![[0x11; 32], [0x22; 32], [0x33; 32], [0x44; 32]],
        ),
        compare_case(
            "wide-values",
            [0xfe; 32],
            [
                0,
                1,
                0x0102_0304,
                0x1122_3344,
                0x5566_7788,
                0x99aa_bbcc,
                0xddee_ff00,
                u32::MAX,
            ],
            u128::MAX - 7,
            u128::MAX - 11,
            (0_u8..=63).collect(),
            2,
            vec![[0xaa; 32], [0xbb; 32], [0xcc; 32]],
        ),
    ];

    assert!(cases.iter().all(|case| {
        case.commitment_match && case.leaf_hash_match && case.membership_root_match
    }));

    println!(
        "{}",
        serde_json::to_string_pretty(&cases).expect("report should serialize")
    );
}

#[allow(clippy::too_many_arguments)]
fn compare_case(
    name: &'static str,
    npk_bytes: [u8; 32],
    program_owner: [u32; 8],
    balance: u128,
    nonce: u128,
    data: Vec<u8>,
    proof_index: usize,
    siblings: Vec<[u8; 32]>,
) -> CaseReport {
    let ours = derive_lez_private_account_commitment(&LezPrivateAccountCommitmentInput {
        npk: Digest32(npk_bytes),
        program_owner,
        balance,
        nonce,
        data: HexBytes::new(data.clone()),
    });

    let account = Account {
        program_owner,
        balance,
        data: Data::try_from(data).expect("fixture data should fit"),
        nonce: Nonce(nonce),
    };
    let theirs = Commitment::new(&NullifierPublicKey(npk_bytes), &account).to_byte_array();

    let ours_leaf_hash = hash_lez_commitment_leaf(&ours);
    let theirs_leaf_hash =
        compute_digest_for_path(&Commitment::from_byte_array(theirs), &(0, Vec::new()));

    let ours_root = compute_lez_membership_root(
        &ours,
        &LezMembershipProof {
            index: proof_index as u64,
            siblings: siblings.iter().copied().map(Digest32).collect(),
        },
    );
    let nssa_proof: MembershipProof = (proof_index, siblings);
    let theirs_root = compute_digest_for_path(&Commitment::from_byte_array(theirs), &nssa_proof);

    CaseReport {
        name,
        commitment_match: ours.as_bytes() == &theirs,
        leaf_hash_match: ours_leaf_hash.as_bytes() == &theirs_leaf_hash,
        membership_root_match: ours_root.as_bytes() == &theirs_root,
        commitment_hex: ours.to_hex(),
        leaf_hash_hex: ours_leaf_hash.to_hex(),
        membership_root_hex: ours_root.to_hex(),
    }
}
EOF

run_started="$(date +%s)"
if cargo run --manifest-path "$TMP_DIR/Cargo.toml" --quiet > "$OUTPUT_JSON"; then
  run_status="ok"
else
  run_status="fail"
  status="fail"
fi
run_duration="$(duration "$run_started")"
total_duration="$(duration "$started")"

{
  echo "# M2 LEZ Commitment Compatibility"
  echo
  echo "| Step | Command | Status | Output | Duration |"
  echo "| --- | --- | --- | --- | --- |"
  echo "| prepare | temp Cargo project using local nssa_core | ok | $TMP_DIR | 00:00:00 |"
  echo "| compare | cargo run --manifest-path <temp>/Cargo.toml --quiet | $run_status | $OUTPUT_JSON | $run_duration |"
  echo "| total | - | $status | $REPORT | $total_duration |"
  echo
  echo "## JSON Output"
  echo
  echo '```json'
  if [[ -s "$OUTPUT_JSON" ]]; then
    cat "$OUTPUT_JSON"
  else
    echo "[]"
  fi
  echo '```'
} > "$REPORT"

cat "$REPORT"

if [[ "$status" != "ok" ]]; then
  exit 1
fi
