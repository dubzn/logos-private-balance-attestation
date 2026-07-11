# Evaluator Guide

This repository is a technical LP-0005 implementation. It is not an award-ready
submission yet because the evaluator-approved on-chain verification model,
CU measurements, final Delivery walkthrough, and final video are still pending.
Public testnet evidence now exists for both current on-chain candidate paths.
The local proof, off-chain verification, Messaging-style CLI transport,
Basecamp Delivery wiring, standalone consumer integrations, and Workable
host-preverified LEZ gate path are implemented.
Spike 09 also demonstrates a PPE-native LEZ gate candidate where private
execution checks `balance >= threshold` and writes public gate/nullifier state.

## What To Review First

1. Read [README.md](../README.md) for status and limitations.
2. Read [SUBMISSION_STATUS.md](SUBMISSION_STATUS.md) for the current
   review-ready surfaces, exact commands, and pending final-submission items.
3. Read [ARCHITECTURE.md](ARCHITECTURE.md) for the proof envelope, context
   binding, presenter binding, off-chain path, Workable public gate, and
   Spike 09 PPE-native candidate.
4. Read [ONCHAIN_PATH_DECISION.md](ONCHAIN_PATH_DECISION.md) for why the current
   public LEZ path is host-preverified and why Spike 09 is the current
   PPE-native candidate.
5. Read [SECURITY_MODEL.md](SECURITY_MODEL.md) for privacy, replay, and
   forwarding assumptions.
6. Check [PRIZE_CHECKLIST.md](PRIZE_CHECKLIST.md) for the exact requirement
   mapping.
7. Check [TESTNET_DEPLOYMENT.md](TESTNET_DEPLOYMENT.md) for public testnet
   program ids, transaction hashes, and readback evidence.

## Quick Smoke Path

This path does not need a local LEZ sequencer or wallet. It proves a synthetic
fixture and verifies the public proof envelope.

```sh
./demo.sh --quick
```

For a real RISC Zero receipt instead of dev-mode receipts:

```sh
./demo.sh --quick --real-prover
```

This confirms the circuit/prover/verifier stack, but it does not exercise live
wallet private state or `getProofForCommitment`.

## Off-Chain Messaging Path

This path demonstrates the LP-0005 off-chain proof-message shape with a local
JSON transport:

```sh
./demo.sh --messaging --real-prover
```

It performs:

```text
proof envelope
  -> proof message
  -> receive/import
  -> local verify
  -> token-gated group admission
  -> duplicate nullifier rejection
```

The CLI transport is intentionally local and replaceable. It is useful for CI
and deterministic smoke tests.

For the real Logos Messaging/Delivery path, use the Basecamp module. It declares
`delivery_module`, creates and starts a Delivery node, subscribes to the LP-0005
proof-message topic, sends the same proof message bytes, stores received
messages, and verifies them locally with `message-verify`.

```sh
scripts/check-basecamp-package.sh
scripts/run-basecamp-local.sh --reset --real-prover
```

See [LOGOS_DELIVERY.md](LOGOS_DELIVERY.md) for the two-instance Delivery QA
flow.

## Real Local Sequencer Path

Use this path to exercise live wallet private state and the real sequencer
`getProofForCommitment` route.

Recommended layout:

```text
workdir/
  logos-private-balance-attestation/
  logos-execution-zone/
```

Set the environment:

```sh
source scripts/env.example
export LOGOS_LEZ_REPO=/absolute/path/to/logos-execution-zone
export NSSA_WALLET_HOME_DIR="$LOGOS_LEZ_REPO/.wallet-local"
export LEE_WALLET_HOME_DIR="$NSSA_WALLET_HOME_DIR"
```

Start the sequencer from the same LEZ checkout:

```sh
cd "$LOGOS_LEZ_REPO"
if [ -d lez/sequencer/service/configs/debug ]; then
  rm -rf lez/sequencer/service/data/debug
  RISC0_DEV_MODE=0 RUST_LOG=info cargo run --features standalone -p sequencer_service \
    lez/sequencer/service/configs/debug/sequencer_config.json
else
  rm -rf sequencer/service/data/debug
  RISC0_DEV_MODE=0 RUST_LOG=info cargo run --features standalone -p sequencer_service \
    sequencer/service/configs/debug/sequencer_config.json
fi
```

In another terminal, initialize wallet state if needed:

```sh
cd "$LOGOS_LEZ_REPO"
if [ -d lez/wallet ]; then
  cargo install --path lez/wallet --force
else
  cargo install --path wallet --force
fi
wallet account new public --label presenter
wallet check-health
wallet account new private --label private-balance
```

Then from this repository:

```sh
scripts/check-wallet-preflight.sh

PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  scripts/prepare-local-private-account.sh

PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  ./demo.sh --real-prover
```

`witness.json` is private and must not be published. The demo writes public
envelope, verifier output, gate report, and run summary artifacts under
`.demo-runs/`.

## PPE-Native Gate Candidate

This path exercises the Spike 09 Logos privacy-preserving execution candidate.
It does not use the portable off-chain proof envelope; instead, the LEZ private
execution proof checks the balance condition and writes public gate/nullifier
state.

```sh
RISC0_DEV_MODE=0 ./demo.sh --ppe-gate --real-prover
```

The script creates fresh local accounts, funds a private holder, admits once,
checks duplicate rejection, checks insufficient-balance rejection, and writes a
local benchmark report under `.demo-runs/spike-09-ppe-gate/`. Treat this as
local evidence for the on-chain-path discussion, not as devnet/testnet CU data.

## Basecamp MVP

Automated package check:

```sh
scripts/check-basecamp-package.sh
```

Build, install, and launch helper:

```sh
scripts/run-basecamp-local.sh --reset --real-prover
```

Build the Basecamp plugin:

```sh
cd apps/basecamp
nix build .#install
```

Install the generated plugin tree into a local Nix-built Basecamp user
directory:

```sh
export BASECAMP_USER_DIR=/absolute/path/to/basecamp-user

rm -rf "$BASECAMP_USER_DIR/plugins/balance_attestation"
mkdir -p "$BASECAMP_USER_DIR/plugins"
cp -R result/plugins/balance_attestation "$BASECAMP_USER_DIR/plugins/"
chmod -R u+w "$BASECAMP_USER_DIR/plugins/balance_attestation"
```

Launch Basecamp with the same environment used by the CLI:

```sh
BALANCE_ATTEST_REPO=/absolute/path/to/logos-private-balance-attestation \
LOGOS_LEZ_REPO=/absolute/path/to/logos-execution-zone \
NSSA_WALLET_HOME_DIR=/absolute/path/to/logos-execution-zone/.wallet-local \
RISC0_DEV_MODE=0 \
  /absolute/path/to/logos-basecamp/run-dev.sh --user-dir "$BASECAMP_USER_DIR"
```

The app appears as `balance_attestation` under UI Modules.

The Delivery panel in the app is backed by the real `delivery_module`
dependency. See [LOGOS_DELIVERY.md](LOGOS_DELIVERY.md) and
[BASECAMP_QA.md](BASECAMP_QA.md) for the full manual UI checklist and video
capture notes.

## Reference Integrations

The repository includes three local integrations:

```sh
cargo test -p governance-gate-example
cargo test -p chat-gate-example
cargo test -p fee-tier-gate-example
```

Their proof-generating tests are ignored by default. To run those in dev mode:

```sh
RISC0_DEV_MODE=1 cargo test -p governance-gate-example -- --include-ignored
RISC0_DEV_MODE=1 cargo test -p chat-gate-example -- --include-ignored
RISC0_DEV_MODE=1 cargo test -p fee-tier-gate-example -- --include-ignored
```

## Public Hygiene

Before reviewing public artifacts:

```sh
scripts/check-public-clean-room.sh
scripts/check-public-clean-room.sh --with-tests
```

Use `--with-lez` only when `LOGOS_LEZ_REPO` points at the intended local
`logos-execution-zone` checkout.

For a single evaluator-style dry run that writes a consolidated report:

```sh
./demo.sh --clean-room --real-prover
```

This default dry run does not require a wallet or sequencer. It runs public
hygiene checks, the quick proof/verify demo, and the local Messaging demo. To
include live LEZ paths:

```sh
PRIVATE_ACCOUNT=Private/<private-account-id> \
  ./demo.sh --clean-room --real-prover --with-lez --with-live --with-ppe
```

The live run writes private witness artifacts under `.demo-runs/clean-room/`;
do not publish those artifacts without removing `witness.json`.

## Known Limitations

- The deployable LEZ program currently records gate/nullifier state after
  host-side verification. It does not cryptographically verify the RISC Zero
  receipt inside public LEZ execution.
- Spike 09 demonstrates a stronger PPE-native LEZ gate candidate, but it is
  still a spike and does not consume the same portable off-chain proof
  envelope.
- The CLI Messaging path is local JSON by design, but the Basecamp MVP now wires
  the real `delivery_module`. Final submission still needs a recorded
  two-instance Delivery walkthrough.
- Public testnet deployment evidence exists, but CU measurements and narrated
  video artifacts are still pending.
