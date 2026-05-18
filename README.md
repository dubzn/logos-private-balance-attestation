# Logos Private Balance Attestation

Implementation workspace for [LP-0005: Private Token Balance Attestation](LP-0005.md).

This repository builds a reusable Logos primitive that lets a holder of a private
LEZ account prove:

```text
private_balance >= public_threshold
```

without revealing the private account id, nullifier public key, exact balance,
nonce, or account data.

## Current Status

This is a draft-stage implementation for technical review, not an award-ready
submission yet.

Implemented locally:

- RISC Zero circuit proving `balance >= threshold` over the LEZ private account
  commitment format.
- Real wallet/sequencer witness path using local wallet private state and
  `getProofForCommitment`.
- Off-chain verifier crate for the public proof envelope.
- CLI for inspect/prove/verify and LEZ gate commands.
- Deployable LEZ gate-state program for the current Workable path.
- Backend-backed Basecamp GUI MVP that loads as a `ui_qml` plugin locally.
- Two reference integrations: governance gate and chat gate.
- IDL artifact, deterministic error codes, CI, local E2E scripts, and local
  benchmark documentation.

Still pending for final LP-0005 submission:

- Evaluator-approved on-chain verification model.
- Basecamp UX polish and final package/install validation.
- Logos Messaging-specific transport adapter or accepted equivalent.
- Third reference integration, ideally with an external integrator.
- LEZ devnet/testnet deployment and CU measurements.
- Narrated demo video showing `RISC0_DEV_MODE=0`.

## Important Limitation

The local LEZ public execution path tested here does not currently expose a
working assumptions/receipt channel for verifying an external RISC Zero receipt
inside a public LEZ guest.

Because of that, the current live LEZ path is **Workable / host-preverified**:

1. The host verifies the public proof envelope with `attestation-verifier`.
2. The host submits a LEZ transaction only after verification succeeds.
3. The deployable LEZ program records gate admission state and deduplicates the
   context nullifier in `account.data`.

The deployable LEZ program does not yet cryptographically verify the RISC Zero
receipt inside public LEZ execution. This is tracked explicitly in
[docs/ONCHAIN_PATH_DECISION.md](docs/ONCHAIN_PATH_DECISION.md) and
[docs/PRIZE_CHECKLIST.md](docs/PRIZE_CHECKLIST.md).

## Repository Layout

```text
crates/attestation-core      shared types, hashes, envelope, error codes
crates/attestation-prover    witness/proof construction helpers
crates/attestation-verifier  off-chain proof envelope verifier
crates/attestation-cli       CLI for inspect/prove/verify/gate commands
crates/attestation-sdk       umbrella crate for integrations
methods/                     production RISC Zero balance-attestation circuit
lez-verifier/                in-memory recursive/on-chain-path rehearsal
lez-verifier/program/        deployable LEZ gate-state program
apps/basecamp/               backend-backed ui_qml Basecamp MVP
examples/governance-gate/    reference governance integration
examples/chat-gate/          reference chat/admission integration
spikes/                      risk spikes and local LEZ probes
scripts/                     reproducible local flows and setup helpers
idl/                         LEZ verifier IDL artifact
docs/                        architecture, setup, security, benchmarks, checklist
```

## Quick Start

Recommended local layout:

```text
workdir/
  logos-private-balance-attestation/
  logos-execution-zone/
```

The scripts resolve the LEZ checkout in this order: `LOGOS_LEZ_REPO`,
`LEZ_REPO`, sibling `../logos-execution-zone`, then the legacy
`$HOME/logos/src/logos-execution-zone` path. For a local fork, source:

```sh
source scripts/env.example
```

The current local fork baseline is synced with
`logos-blockchain/logos-execution-zone` `main` at `4079b0c9` (2026-05-18). If
you update LEZ again and hit wallet storage errors, create a fresh wallet home
for that checkout and rerun `scripts/check-wallet-preflight.sh`.

Run the fast smoke demo with deterministic fixtures:

```sh
scripts/demo-end-to-end.sh
```

This proves and verifies a synthetic witness. It is useful for checking the
workspace quickly, but it does not use a wallet account or live sequencer
membership proof.

For the real local wallet + sequencer path, start a local LEZ sequencer, set a
private account, and run the non-interactive preflight first:

```sh
scripts/check-wallet-preflight.sh
```

If the wallet home is missing or the wallet wants setup/password input, the
preflight prints the exact commands to initialize it instead of hanging inside
an E2E script. It also detects wallet storage written by an older
`logos-execution-zone` checkout, which otherwise fails later while building the
real witness.

A newly generated private account is only local wallet state until it is
initialized and funded on the local sequencer. Prepare it before proving:

```sh
PRIVATE_ACCOUNT="Private/REPLACE_WITH_PRIVATE_ACCOUNT_ID" \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  scripts/prepare-local-private-account.sh
```

Once that passes, run:

```sh
PRIVATE_ACCOUNT="Private/REPLACE_WITH_PRIVATE_ACCOUNT_ID" \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  ./demo.sh
```

`demo.sh` is the Lambda Prize root entrypoint and delegates to
`scripts/demo-local-full-e2e.sh`. That script composes:

```text
real local wallet state
  -> getProofForCommitment
  -> witness.json
  -> RISC Zero proof envelope
  -> off-chain verification
  -> host-preverified LEZ gate admit
  -> persisted context nullifier
```

`witness.json` is private. Do not publish it. Public artifacts include the proof
envelope, verifier output, gate report, and run summary.

For setup details, see [docs/LOCAL_SETUP.md](docs/LOCAL_SETUP.md).

Before publishing or recording a run, check the public repo hygiene:

```sh
scripts/check-public-clean-room.sh
```

Use `--with-tests` for workspace tests and `--with-lez` for checks that need
the local LEZ fork.

## CLI Examples

Generate and verify a deterministic fixture:

```sh
cargo run -p demo-runner --bin build-demo-fixtures -- ./demo

RISC0_DEV_MODE=1 cargo run -p attestation-cli -- prove \
  --witness ./demo/witness.json \
  --out ./demo/envelope.json

RISC0_DEV_MODE=1 cargo run -p attestation-cli -- verify \
  --envelope ./demo/envelope.json \
  --gate ./demo/gate.json
```

Inspect a local private account without printing witness data:

```sh
cargo run -p attestation-cli -- inspect-private \
  --account "Private/REPLACE_WITH_PRIVATE_ACCOUNT_ID" \
  --local-only
```

With a local sequencer running, request the real membership proof:

```sh
cargo run -p attestation-cli -- inspect-private \
  --account "Private/REPLACE_WITH_PRIVATE_ACCOUNT_ID" \
  --require-proof
```

Prepare the current Workable LEZ gate path:

```sh
cargo run -p attestation-cli -- gate-register-presenter \
  --presenter-account "Public/REPLACE_WITH_PRESENTER_ACCOUNT" \
  --admin-account "Public/REPLACE_WITH_FRESH_REGISTER_ADMIN" \
  --presenter-pubkey-hex "$(jq -r .presenter_pubkey .demo-runs/local-sequencer/REPLACE_WITH_RUN/envelope.json)"

cargo run -p attestation-cli -- gate-init \
  --gate .demo-runs/local-sequencer/REPLACE_WITH_RUN/gate.json \
  --gate-account "Public/REPLACE_WITH_GATE_STATE" \
  --admin-account "Public/REPLACE_WITH_FRESH_INIT_ADMIN"

cargo run -p attestation-cli -- gate-admit \
  --envelope .demo-runs/local-sequencer/REPLACE_WITH_RUN/envelope.json \
  --gate .demo-runs/local-sequencer/REPLACE_WITH_RUN/gate.json \
  --gate-account "Public/REPLACE_WITH_GATE_STATE" \
  --presenter-account "Public/REPLACE_WITH_REGISTERED_PRESENTER"
```

Add `--execute` to submit transactions. Use fresh public accounts for setup
commands; local LEZ nonce/freshness behavior can reject reused setup signers in
one flow.

## Testing

Core workspace tests:

```sh
cargo test --workspace
```

Ignored E2E-style suites in dev mode:

```sh
RISC0_DEV_MODE=1 cargo test --workspace -- --include-ignored
```

Deployable LEZ program checks:

```sh
cargo test --manifest-path lez-verifier/program/Cargo.toml
cargo check --manifest-path spikes/spike-08-program-chaining/lez/runner/Cargo.toml
```

The runner check depends on the Logos Blockchain circuits bundle. See the CI
workflow and [docs/LOCAL_SETUP.md](docs/LOCAL_SETUP.md) for setup details.

## Benchmarks

Local wall-clock timings from the latest full local E2E are documented in
[docs/BENCHMARKS.md](docs/BENCHMARKS.md).

Latest recorded local run with `RISC0_DEV_MODE=0`:

| Phase | Duration |
| --- | ---: |
| Proof phase | 00:01:48 |
| Gate phase | 00:01:42 |
| Total | 00:03:30 |

These are not devnet/testnet CU metrics. CU measurement is still pending.

## Documentation

- [Architecture](docs/ARCHITECTURE.md): proof format, flow, and component boundaries.
- [Local Setup](docs/LOCAL_SETUP.md): sequencer, wallet, private account, and demo commands.
- [Security Model](docs/SECURITY_MODEL.md): privacy guarantees, replay, forwarding, nullifiers, and limits.
- [On-Chain Path Decision](docs/ONCHAIN_PATH_DECISION.md): why the current on-chain path is Workable / host-preverified.
- [Prize Checklist](docs/PRIZE_CHECKLIST.md): LP-0005 requirements mapped to current artifacts.
- [Benchmarks](docs/BENCHMARKS.md): local proof/gate timings and remaining CU work.
- [Error Codes](docs/ERROR_CODES.md): deterministic BAxxx errors.
- [IDL Draft](docs/IDL_DRAFT.md): LEZ verifier interface.
- [Basecamp MVP](apps/basecamp/README.md): local `ui_qml` proof and gate workflow.
- [Next Steps](docs/NEXT_STEPS.md): ordered backlog to final submission.
- [Risk Spikes](docs/RISK_SPIKES.md): high-risk assumptions and validation plan.
- [Reference Notes](docs/REFERENCE_NOTES.md): public research notes and compatibility findings.

## License

MIT. See [LICENSE](LICENSE).
