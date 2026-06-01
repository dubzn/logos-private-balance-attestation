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

This is a technical implementation for review, not an award-ready submission
yet. The remaining blocker is the evaluator-approved live on-chain proof
verification model.

Implemented:

- RISC Zero circuit proving `balance >= threshold` over the LEZ private account
  commitment format.
- Real wallet/sequencer witness path using local wallet private state and
  `getProofForCommitment`.
- Off-chain verifier crate for the public proof envelope.
- Local/pluggable Messaging adapter for transporting proof envelopes and
  simulating token-gated group admission.
- CLI for inspect/prove/verify and LEZ gate commands.
- Deployable LEZ gate-state program for the current Workable path.
- PPE-native LEZ balance-gate spike that checks private `balance >= threshold`
  inside a privacy-preserving transaction and writes public gate/nullifier
  state.
- Backend-backed Basecamp GUI MVP that loads as a `ui_qml` plugin locally.
- Three reference integrations: governance gate, chat gate, and fee-tier gate.
- IDL artifact, deterministic error codes, CI, local E2E scripts, and local
  benchmark documentation.
- Public LEZ testnet evidence for both current on-chain candidate paths.

Still pending for final LP-0005 submission:

- Evaluator-approved on-chain verification model.
- Logos Messaging-specific network adapter, if evaluators require the real
  transport instead of the current local JSON adapter.
- Final consumer-integration evidence, using the updated LP-0005 requirement
  for a standalone runnable integration demo.
- CU measurements for the accepted on-chain path.
- Narrated demo video showing the CLI and Basecamp flows with
  `RISC0_DEV_MODE=0`.

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

There is also a stronger candidate in `spikes/spike-09-ppe-gate/`: a
Logos privacy-preserving transaction checks the private balance condition and
writes public `BAP1` gate/nullifier state. It passed locally and on public LEZ
testnet with
`RISC0_DEV_MODE=0`, including duplicate and insufficient-balance rejection.
Evaluator confirmation is still required because this PPE-native path is not
the same portable proof envelope used by the off-chain verifier.

## Repository Layout

```text
crates/attestation-core      shared types, hashes, envelope, error codes
crates/attestation-prover    witness/proof construction helpers
crates/attestation-verifier  off-chain proof envelope verifier
crates/attestation-messaging local/pluggable proof message transport
crates/attestation-cli       CLI for inspect/prove/verify/gate commands
crates/attestation-sdk       umbrella crate for integrations
methods/                     production RISC Zero balance-attestation circuit
lez-verifier/                in-memory recursive/on-chain-path rehearsal
lez-verifier/program/        deployable LEZ gate-state program
apps/basecamp/               backend-backed ui_qml Basecamp MVP
examples/governance-gate/    reference governance integration
examples/chat-gate/          reference chat/admission integration
examples/fee-tier-gate/      reference fee/discount tier integration
spikes/                      risk spikes and local LEZ probes
scripts/                     reproducible local flows and setup helpers
idl/                         LEZ verifier IDL artifact
docs/                        architecture, setup, security, benchmarks, checklist
```

## Evaluator-Visible Artifacts

These files are intentionally easy to find from the repository root:

| Artifact | Path |
| --- | --- |
| Demo entrypoint | `demo.sh` |
| SPEL-style IDL | `balance-attestation-verifier.idl.json` |
| Basecamp module metadata | `apps/basecamp/module.json` |
| Submission status | `docs/SUBMISSION_STATUS.md` |
| Requirement map | `docs/PRIZE_CHECKLIST.md` |
| Clean-room guide | `docs/EVALUATOR_GUIDE.md` |
| Public testnet evidence | `docs/TESTNET_DEPLOYMENT.md` |
| Benchmarks | `docs/BENCHMARKS.md` |
| Basecamp QA | `docs/BASECAMP_QA.md` |

## LP-0005 Requirement Snapshot

| Area | Current status |
| --- | --- |
| Circuit and proof generation | Implemented and tested locally. |
| Real wallet + `getProofForCommitment` witness path | Implemented in the full local E2E. |
| Off-chain verification | Implemented via `attestation-verifier`. |
| Presenter binding and nullifier | Implemented with challenge-bound BIP-340 presentation signatures and context nullifiers. |
| Off-chain Messaging path | Implemented with a local/pluggable transport; real Logos Messaging network adapter pending if required. |
| LEZ gate path | Workable/host-preverified public gate plus Spike 09 PPE-native private execution candidate. Both have public testnet evidence; evaluator-approved in-LEZ proof verification model is still pending. |
| Basecamp app | Backend-backed `ui_qml` MVP builds and loads locally. |
| Consumer integration demo | Governance gate, chat gate, and fee-tier gate are implemented locally; updated LP-0005 now asks for a standalone runnable consumer integration demo rather than an outside-team integration. |
| CU metrics | Pending; local and public testnet wall-clock evidence is documented. |
| Narrated video | Pending. |

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

Run the default clean-room demo. This does not require a wallet or sequencer:

```sh
./demo.sh --real-prover
```

Run only the fast smoke demo with deterministic fixtures:

```sh
./demo.sh --quick
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
  ./demo.sh --full --real-prover
```

`demo.sh` is the Lambda Prize root entrypoint. The default mode is
`--clean-room`, which runs public hygiene, quick proof/verify, and local
Messaging. `--quick` runs only the synthetic fixture smoke demo, `--messaging`
runs only the local off-chain transport demo, and `--full` delegates to
`scripts/demo-local-full-e2e.sh`. The full mode composes:

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

To exercise the Spike 09 PPE-native candidate, use the same local sequencer and
wallet environment:

```sh
RISC0_DEV_MODE=0 ./demo.sh --ppe-gate --real-prover
```

This creates fresh local accounts, funds a private holder, proves
`balance >= threshold` inside LEZ privacy-preserving execution, writes public
`BAP1` gate/nullifier state, checks duplicate rejection, checks
insufficient-balance rejection, and writes a benchmark report under
`.demo-runs/spike-09-ppe-gate/`.

For setup details, see [docs/LOCAL_SETUP.md](docs/LOCAL_SETUP.md).

Before publishing or recording a run, check the public repo hygiene:

```sh
scripts/check-public-clean-room.sh
```

Use `--with-tests` for workspace tests and `--with-lez` for checks that need
the local LEZ fork.

For an evaluator-style dry run, use the root entrypoint or the underlying
clean-room runner directly:

```sh
./demo.sh --clean-room --real-prover
# or:
scripts/demo-clean-room.sh --real-prover
```

Add live paths when a local sequencer, wallet home, and private account are
available:

```sh
PRIVATE_ACCOUNT="Private/REPLACE_WITH_PRIVATE_ACCOUNT_ID" \
  ./demo.sh --clean-room --real-prover --with-lez --with-live --with-ppe
```

To include the Basecamp package build/inspection in the clean-room check:

```sh
./demo.sh --clean-room --real-prover --with-basecamp
```

To build, install, and launch the local Basecamp MVP:

```sh
scripts/run-basecamp-local.sh --reset --real-prover
```

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

Simulate the off-chain Messaging path with local JSON transport:

```sh
./demo.sh --messaging --dev-mode
```

Or use the CLI pieces directly:

```sh
cargo run -p attestation-cli -- message-export \
  --envelope ./demo/envelope.json \
  --out ./demo/message.json \
  --group demo-chat \
  --sender presenter-local \
  --recipient chat-host-local

cargo run -p attestation-cli -- message-receive \
  --message ./demo/message.json \
  --out ./demo/received-envelope.json

cargo run -p attestation-cli -- message-verify \
  --message ./demo/message.json \
  --gate ./demo/gate.json

cargo run -p attestation-cli -- message-admit \
  --message ./demo/message.json \
  --gate ./demo/gate.json \
  --state ./demo/admissions.json \
  --group demo-chat
```

`message-admit` verifies the proof envelope locally, stores the context
nullifier in `admissions.json`, and rejects duplicate admission attempts. This
is the same admission model expected from a future Logos Messaging transport;
only the transport is local JSON for now.

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

Latest Spike 09 PPE-native local run with `RISC0_DEV_MODE=0`:

| Phase | Duration |
| --- | ---: |
| Fund private holder | 00:02:21 |
| Positive PPE admit | 00:01:49 |
| Duplicate rejection | 00:00:23 |
| Insufficient-balance rejection | 00:00:02 |
| Total | 00:04:56 |

Public testnet deploy/admit evidence is documented in
[docs/TESTNET_DEPLOYMENT.md](docs/TESTNET_DEPLOYMENT.md). These are not CU
metrics. CU measurement is still pending.
Spike 09 PPE-native runs now write their own local benchmark reports via
`./demo.sh --ppe-gate --real-prover`.

## Documentation

- [Architecture](docs/ARCHITECTURE.md): proof format, flow, and component boundaries.
- [Submission Status](docs/SUBMISSION_STATUS.md): what is review-ready, what is pending, and exact evidence commands.
- [Testnet Deployment](docs/TESTNET_DEPLOYMENT.md): public testnet program ids, transaction hashes, readback, and remaining CU gap.
- [Local Setup](docs/LOCAL_SETUP.md): sequencer, wallet, private account, and demo commands.
- [Evaluator Guide](docs/EVALUATOR_GUIDE.md): clean review path and known limitations.
- [Basecamp QA](docs/BASECAMP_QA.md): package inspection, manual UI checklist, and video notes.
- [Security Model](docs/SECURITY_MODEL.md): privacy guarantees, replay, forwarding, nullifiers, and limits.
- [On-Chain Path Decision](docs/ONCHAIN_PATH_DECISION.md): why the current on-chain path is Workable / host-preverified.
- [Prize Checklist](docs/PRIZE_CHECKLIST.md): LP-0005 requirements mapped to current artifacts.
- [Benchmarks](docs/BENCHMARKS.md): local proof/gate timings and remaining CU work.
- [Error Codes](docs/ERROR_CODES.md): deterministic BAxxx errors.
- [IDL Reference](docs/IDL_DRAFT.md): LEZ verifier interface.
- [Basecamp MVP](apps/basecamp/README.md): local `ui_qml` proof and gate workflow.
- [Next Steps](docs/NEXT_STEPS.md): ordered backlog to final submission.
- [PR Reopen Message](docs/PR_REOPEN_MESSAGE.md): prepared text for the eventual Lambda Prize PR.
- [Demo Video Script](docs/DEMO_VIDEO_SCRIPT.md): recording plan for the final narrated demo.
- [Risk Spikes](docs/RISK_SPIKES.md): high-risk assumptions and validation plan.
- [Reference Notes](docs/REFERENCE_NOTES.md): public research notes and compatibility findings.

## License

MIT. See [LICENSE](LICENSE).
