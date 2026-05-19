# Evaluator Guide

This repository is a technical LP-0005 implementation. It is not an award-ready
submission yet because the evaluator-approved on-chain verification model,
testnet deployment, CU measurements, external integration evidence, and final
video are still pending. The local proof, off-chain verification,
Messaging-style transport, Basecamp MVP, three local reference integrations,
and Workable host-preverified LEZ gate path are implemented.
Spike 09 also demonstrates a PPE-native LEZ gate candidate where private
execution checks `balance >= threshold` and writes public gate/nullifier state.

## What To Review First

1. Read [README.md](../README.md) for status and limitations.
2. Read [ARCHITECTURE.md](ARCHITECTURE.md) for the proof envelope, context
   binding, presenter binding, off-chain path, Workable public gate, and
   Spike 09 PPE-native candidate.
3. Read [ONCHAIN_PATH_DECISION.md](ONCHAIN_PATH_DECISION.md) for why the current
   public LEZ path is host-preverified and why Spike 09 is the current
   PPE-native candidate.
4. Read [SECURITY_MODEL.md](SECURITY_MODEL.md) for privacy, replay, and
   forwarding assumptions.
5. Check [PRIZE_CHECKLIST.md](PRIZE_CHECKLIST.md) for the exact requirement
   mapping.

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

This path demonstrates the LP-0005 off-chain shape with a local JSON transport:

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

The transport is intentionally local and replaceable. A future Logos Messaging
adapter should implement the same `ProofMessageTransport` trait and carry the
same message bytes.

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
```

Start the sequencer from the same LEZ checkout:

```sh
cd "$LOGOS_LEZ_REPO"
rm -rf sequencer/service/data/debug
RISC0_DEV_MODE=0 RUST_LOG=info cargo run --features standalone -p sequencer_service \
  sequencer/service/configs/debug/sequencer_config.json
```

In another terminal, initialize wallet state if needed:

```sh
cd "$LOGOS_LEZ_REPO"
cargo install --path wallet --force
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

## Basecamp MVP

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

## Known Limitations

- The deployable LEZ program currently records gate/nullifier state after
  host-side verification. It does not cryptographically verify the RISC Zero
  receipt inside public LEZ execution.
- Spike 09 demonstrates a stronger PPE-native LEZ gate candidate, but it is
  still a spike and does not consume the same portable off-chain proof
  envelope.
- Real Logos Messaging network transport is not wired yet. The current adapter
  is local JSON and intentionally pluggable.
- Testnet deployment, CU measurements, external integrator validation, and
  narrated video artifacts are still pending.
