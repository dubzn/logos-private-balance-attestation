# Local Setup

This document describes the intended local development flow. Some commands are
future-facing because the repo is still being built in layers. The LEZ and
wallet commands are based on the local `logos-execution-zone` checkout.

## Expected Local Paths

```sh
export LOGOS_LEZ_REPO="$HOME/logos/src/logos-execution-zone"
export BALANCE_ATTEST_REPO="/Users/dub/Desktop/logos/logos-private-balance-attestation"
export NSSA_WALLET_HOME_DIR="$LOGOS_LEZ_REPO/.wallet-local"
```

Use one wallet home per local demo. Mixing wallet homes makes private account
state confusing because private account data is local-only.

## Prerequisites

- Rust toolchain.
- Docker, for guest builds that use the LEZ/RISC Zero build flow.
- RISC Zero toolchain compatible with the local LEZ checkout.
- Local `logos-execution-zone` checkout at `~/logos/src/logos-execution-zone`.
- `wallet` installed from the local LEZ checkout.

Install or refresh wallet:

```sh
cd "$LOGOS_LEZ_REPO"
cargo install --path wallet --force
```

The inspected LEZ checkout currently pins:

```text
risc0-zkvm = 3.0.5
risc0-build = 3.0.5
```

The attestation workspace should use the same RISC Zero major version unless
the LEZ checkout changes.

The implementation should add `scripts/check-risc0-version.sh` before the first
Rust milestone. That script must compare the attestation workspace RISC Zero
version against the local LEZ checkout and fail CI on mismatch.

## Run The Core Unit Tests

The first reusable crate does not need a sequencer, wallet, Docker, or RISC
Zero proving. From the repo root:

```sh
cd "$BALANCE_ATTEST_REPO"
cargo test -p attestation-core
```

Expected result:

```text
11 passed
```

These tests cover context id derivation, presenter id derivation, context
nullifier behavior, proof envelope shape, stable JSON encoding, and
deterministic error-code mappings.

## Check LEZ Commitment Compatibility

This command creates a temporary Cargo project that depends on this repo's
`attestation-core` and the local LEZ `nssa_core`, then compares the commitment
and Merkle helpers against the real LEZ implementation:

```sh
cd "$BALANCE_ATTEST_REPO"
scripts/m2-check-lez-commitment-compat.sh
```

Expected result:

```text
commitment_match: true
leaf_hash_match: true
membership_root_match: true
```

The Markdown and JSON outputs are written under:

```text
.spike-results/m2-commitment-compat/
```

## Inspect A Local Private Account Safely

To validate the wallet-side adapter without a running sequencer:

```sh
cd "$BALANCE_ATTEST_REPO"
export NSSA_WALLET_HOME_DIR="$LOGOS_LEZ_REPO/.wallet-local"
cargo run -p attestation-cli -- inspect-private \
  --account Private/<private-account-id> \
  --local-only
```

Expected output:

```text
private_state_found: true
local_commitment_matches_wallet: true
membership_proof_found: false
```

The script intentionally redacts the account id and does not print `npk`,
balance, nonce, account data, private keys, the raw commitment, or Merkle
siblings.

With the sequencer running, request the real proof through the wallet path that
calls `getProofForCommitment`:

```sh
cargo run -p attestation-cli -- inspect-private \
  --account Private/<private-account-id> \
  --require-proof
```

Expected proof-mode output:

```text
membership_proof_found: true
proof_depth: <number>
commitment_root_hex: <hex-32>
core_root_matches_wallet_root: true
```

If `wallet check-health` fails with `Connection refused` on `127.0.0.1:3040`,
start the local sequencer first.

Observed successful local proof-mode result:

```text
private_state_found: true
local_commitment_matches_wallet: true
membership_proof_found: true
core_root_matches_wallet_root: true
```

The older script remains useful as an M2 harness and Markdown report generator:

```sh
PRIVATE_ACCOUNT="<private-account-id-without-Private>" \
  scripts/m2-inspect-private-account.sh --require-proof
```

## Start Local Sequencer

In terminal 1:

```sh
cd "$LOGOS_LEZ_REPO"
RUST_LOG=info cargo run --features standalone -p sequencer_service \
  sequencer/service/configs/debug/sequencer_config.json
```

Healthy signs:

- the JSON-RPC server listens on port `3040`
- logs mention that the sequencer is running

In terminal 2:

```sh
export LOGOS_LEZ_REPO="$HOME/logos/src/logos-execution-zone"
export NSSA_WALLET_HOME_DIR="$LOGOS_LEZ_REPO/.wallet-local"
wallet check-health
```

## Create Demo Accounts

Create a public presenter account:

```sh
wallet account new public --label presenter
```

Save the account id without adding an extra `Public/` prefix. If the command
prints:

```text
Generated new account with account_id Public/ABC...
```

then the raw id is:

```text
ABC...
```

Create a private balance account:

```sh
wallet account new private --label private-balance
```

This prints the private account id plus local `npk` and `vpk` values. Do not
paste those keys into public logs.

## Fund A Private Account For Testing

Initialize and fund the public presenter account:

```sh
wallet auth-transfer init --account-id Public/<presenter-id>
wallet pinata claim --to Public/<presenter-id>
```

Send native tokens from the public account to the private account:

```sh
wallet auth-transfer send \
  --from Public/<presenter-id> \
  --to Private/<private-account-id> \
  --amount 42
```

Sync local private state:

```sh
wallet account sync-private
wallet account get --account-id Private/<private-account-id>
```

The final `account get` reads local wallet storage. Other users cannot query the
private account balance from the network.

## Future CLI Flow

After the first implementation milestones, the intended CLI flow is:

```sh
cargo run -p attestation-cli -- inspect-private \
  --account Private/<private-account-id>

cargo run -p attestation-cli -- prove \
  --account Private/<private-account-id> \
  --threshold 25 \
  --context "local/governance/demo-vote-1" \
  --presenter Public/<presenter-id> \
  --out proof.json

cargo run -p attestation-cli -- verify-offchain \
  --proof proof.json \
  --context "local/governance/demo-vote-1" \
  --threshold 25
```

The current smoke demo can run with real proving over deterministic fixtures:

```sh
RISC0_DEV_MODE=0 scripts/demo-end-to-end.sh
```

The final submission demo still needs the same real-prover mode against a
clean local sequencer, wallet state, and `getProofForCommitment`.

The current local-sequencer harness is:

```sh
PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
RISC0_DEV_MODE=1 \
  scripts/demo-local-sequencer-e2e.sh
```

It writes artifacts under `.demo-runs/local-sequencer/<timestamp>/`. Treat
`witness.json` as private; it contains private account witness material and the
presenter secret. The public artifacts are `envelope.json`, `verify.json`, and
`report.md`.

For fast development, early local tests may use:

```sh
RISC0_DEV_MODE=1 cargo test
```

The distinction must be visible in the final docs and video.

For Spike 01 private execution tests, start the sequencer in dev mode too:

```sh
cd "$LOGOS_LEZ_REPO"
RISC0_DEV_MODE=1 RUST_LOG=info cargo run --features standalone -p sequencer_service \
  sequencer/service/configs/debug/sequencer_config.json
```

Then run the automated private balance gate fixture from this repo:

```sh
cd "$BALANCE_ATTEST_REPO"
RISC0_DEV_MODE=1 scripts/spike-01-demo-private-gate.sh
```

If the wallet generates dev-mode private proofs but the sequencer is not also
running with `RISC0_DEV_MODE=1`, private transactions are rejected as
`InvalidPrivacyPreservingProof`.

The local standalone sequencer commonly includes transactions on a roughly
15-second cadence. Scripts that check account state should poll with a timeout
instead of relying on a single short sleep.

## Inspect A Private Membership Proof

After a private account has been funded and synced, check that the sequencer can
return a Merkle proof for its current commitment:

```sh
cd "$BALANCE_ATTEST_REPO"
export PRIVATE_ACCOUNT="<private-account-id-without-Private>"
scripts/spike-02-inspect-membership-proof.sh
```

Expected output includes:

```json
{
  "private_state_found": true,
  "membership_proof_found": true,
  "proof_depth": 4,
  "commitment_root_hex": "<hex-32>"
}
```

## Run The Balance Circuit Spike

Build the standalone RISC Zero guest that checks the balance threshold,
reconstructs the LEZ private account commitment, and verifies the Merkle path:

```sh
cd "$BALANCE_ATTEST_REPO"
scripts/spike-03-build-balance-circuit.sh
```

Run fast fixture checks without wallet/sequencer state:

```sh
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh
```

Run against a real initialized private account. Keep the local sequencer running
with `RISC0_DEV_MODE=1` in another terminal:

```sh
export PRIVATE_ACCOUNT="<private-account-id-without-Private>"
export THRESHOLD=25
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live
```

Run the live negative check:

```sh
export THRESHOLD=999999
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live-below-threshold
```

Expected live success output includes:

```json
{
  "mode": "live",
  "proved": true,
  "verified": true,
  "journal": {
    "threshold": "25",
    "commitment_root_hex": "<hex-32>",
    "proof_depth": 4
  }
}
```

The journal is public-only. It must not include the exact balance, `npk`,
private keys, account nonce, account data, or Merkle proof siblings.

## Run The Binding Circuit Spike

Spike 04 adds context binding, presenter binding, and a context nullifier. It
also removes the commitment leaf from the public journal.

Build:

```sh
cd "$BALANCE_ATTEST_REPO"
scripts/spike-04-build-binding-circuit.sh
```

Run fixture checks:

```sh
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh
```

Run against a real initialized private account. Keep the local sequencer running
with `RISC0_DEV_MODE=1` in another terminal:

```sh
export PRIVATE_ACCOUNT="<private-account-id-without-Private>"
export THRESHOLD=25
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live
```

Run the live negative check:

```sh
export THRESHOLD=999999
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live-below-threshold
```

Expected live success output includes:

```json
{
  "mode": "live",
  "proved": true,
  "verified": true,
  "journal": {
    "threshold": "25",
    "commitment_root_hex": "<hex-32>",
    "context_id_hex": "<hex-32>",
    "context_nullifier_hex": "<hex-32>",
    "presenter_id_hex": "<hex-32>"
  }
}
```

## Dev/Prod Baseline

Spike 05 keeps two separate benchmark outputs:

```sh
scripts/spike-05-run-devmode-baseline.sh  # writes .spike-results/spike-05-devmode.md
scripts/spike-05-run-prod-baseline.sh     # writes .spike-results/spike-05-prod.md
```

Each output must be a Markdown table:

```text
| Step | Command | Status | Output | Duration |
| --- | --- | --- | --- | --- |
```

The final row must report total duration so `RISC0_DEV_MODE=1` and
`RISC0_DEV_MODE=0` are easy to compare.

Default baseline steps:

```text
check-risc0-version
build-binding-circuit
prove-fixture-valid
total
```

Optional extra steps:

```sh
SPIKE05_INCLUDE_CONTEXT_VARIANT=1 scripts/spike-05-run-devmode-baseline.sh

SPIKE05_SKIP_BUILD=1 scripts/spike-05-run-devmode-baseline.sh

SPIKE05_RUN_LIVE=1 \
PRIVATE_ACCOUNT="<private-account-id-without-Private>" \
THRESHOLD=25 \
  scripts/spike-05-run-devmode-baseline.sh
```

Use the same env vars with `scripts/spike-05-run-prod-baseline.sh` when you are
ready to pay the real proving cost.

When `RISC0_DEV_MODE=0`, proof generation can take several minutes. Demo
scripts should print progress markers to stderr, for example:

```text
[1/3] building witness...
[2/3] proving...
[3/3] verifying...
```

## Future On-Chain Flow

The on-chain path should follow the same pattern we used in
`logos-document-guardian`:

```sh
scripts/install-lez-sources.sh
scripts/build-program.sh

cd "$LOGOS_LEZ_REPO/examples/program_deployment"
export EXAMPLE_PROGRAMS_BUILD_DIR="$LOGOS_LEZ_REPO/target/riscv32im-risc0-zkvm-elf/docker"

wallet deploy-program "$EXAMPLE_PROGRAMS_BUILD_DIR/balance_attestation_verifier.bin"

cargo run -p attestation-cli -- claim-access \
  --proof "$BALANCE_ATTEST_REPO/proof.json" \
  --presenter Public/<presenter-id> \
  --context "local/governance/demo-vote-1" \
  --execute
```

The exact command names may change once the LEZ verifier program is
implemented, but the flow should stay stable.

## Troubleshooting Notes

### Invalid Base58 From Double Prefix

Use either:

```sh
wallet account get --account-id Public/<id>
```

or store the raw id:

```sh
PRESENTER_ACCOUNT="<id>"
wallet account get --account-id "Public/$PRESENTER_ACCOUNT"
```

Do not store `Public/<id>` and then call `Public/$PRESENTER_ACCOUNT`.

### Private Account Looks Uninitialized After A Transaction

Run:

```sh
wallet account sync-private
```

Private state is local wallet data refreshed from encrypted transaction outputs.

### Nonce Mismatch

Local runners can submit a transaction and return before the sequencer has
processed the account nonce update. Wait a few seconds, refresh account state,
and rerun the next transaction. Demo scripts should include short waits or
polling.

The implementation should centralize this in `scripts/wait-for-tx.sh` instead
of duplicating sleeps in every demo script.

### Program Already Exists

If a demo deploys the same program twice, the sequencer may log
`ProgramAlreadyExists`. Reuse the deployed program or add `SKIP_DEPLOY=1` to the
script once that option exists.
