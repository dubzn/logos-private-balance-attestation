# Next Steps

This is the current ordered backlog. Keep this file aligned with
`IMPLEMENTATION_PLAN.md`, `MODULAR_TEST_PLAN.md`, and `PRIZE_CHECKLIST.md`.

## Current Position

Done locally:

- risk spikes for on-chain proof path, private execution fallback, membership
  proof retrieval, balance circuit, binding circuit, dev/prod baseline, and
  on-chain decision
- `attestation-core` with envelope/journal types, context hashing,
  presenter/nullifier helpers, LEZ commitment helpers, and Merkle root helpers
- `attestation-prover` initial sanitized private account inspection report
  helpers
- `attestation-prover` initial witness builder with redacted debug/summary
  output
- `attestation-cli inspect-private` for local-only and real
  `getProofForCommitment` inspection
- M2 compatibility scripts against local `nssa_core` and `WalletCore`
- real `getProofForCommitment` proof mode passed against a local sequencer
- `scripts/demo-local-sequencer-e2e.sh` harness that builds a private witness
  from real wallet state, proves, verifies, and writes a run report
- local-sequencer E2E passed with `RISC0_DEV_MODE=1` against a real wallet
  private account and real membership proof
- challenge-bound local-sequencer E2E passed with `RISC0_DEV_MODE=0`: total
  00:01:30, build witness 00:01:01, proving 00:00:23, verify 00:00:03,
  verify `status: ok`
- deployable LEZ gate program, Spike 08 live probe, and CLI
  `gate-register-presenter` / `gate-init` / `gate-admit`
- manual Workable gate flow passed locally: presenter account registered, gate
  account initialized, host-verified admit persisted the context nullifier
- `scripts/demo-local-gate-e2e.sh` passed locally with `RISC0_DEV_MODE=0`:
  total 00:02:11, nullifier count `1`, duplicate admit `not-applied`
- `scripts/demo-local-full-e2e.sh` added to compose proof + gate phases
- full local E2E passed with `RISC0_DEV_MODE=0`: proof phase 00:02:20, gate
  phase 00:02:49, total 00:05:09, verify `ok`, nullifier count `1`, duplicate
  admit `not-applied`
- `docs/BENCHMARKS.md` records the local proof benchmark, gate transaction
  timings, and the still-open devnet/testnet CU measurements
- `scripts/clean-local-artifacts.sh` added for dry-run-first cleanup of build
  outputs and optional run artifacts
- `scripts/common-env.sh` centralizes `LOGOS_LEZ_REPO` resolution with
  side-by-side fork fallback (`../logos-execution-zone`)
- `scripts/env.example` documents the public local environment shape
- `scripts/check-public-clean-room.sh` checks public hygiene, forbidden
  artifacts, shell syntax, and optional tests/LEZ compatibility
- `scripts/check-wallet-preflight.sh` catches missing wallet setup, incompatible
  wallet storage, and sequencer-checkout mismatches before heavy E2E runs
- `scripts/prepare-local-private-account.sh` initializes/funds a selected
  private account for local demos and confirms `getProofForCommitment`
- latest full local E2E against the synced LEZ fork passed with
  `RISC0_DEV_MODE=0`: proof phase 00:01:48, gate phase 00:01:42, total
  00:03:30, verify `ok`, nullifier count `1`, duplicate admit `not-applied`
- `apps/basecamp/` contains a backend-backed `ui_qml` MVP that wraps preflight,
  proof generation, envelope verification, and Workable gate admit
- the local `logos-execution-zone` fork was synced with upstream `main` at
  `4079b0c9` on 2026-05-18; the update includes wallet/private-account storage
  changes, so fresh wallet homes and private-account preparation may be needed
  after switching checkouts
- the Basecamp MVP now builds through `mkLogosQmlModule` as a real Qt plugin
  plus replica factory; direct `ui-host` smoke test reached `READY`
- `scripts/check-basecamp-package.sh` builds and inspects the Basecamp install
  tree; `scripts/run-basecamp-local.sh` installs and launches the local app;
  `docs/BASECAMP_QA.md` captures the manual QA and video checklist
- local/pluggable Messaging transport and token-gated admission flow:
  `attestation-messaging`, CLI `message-*` commands, `./demo.sh --messaging`,
  and `examples/chat-gate`
- Basecamp Logos Delivery adapter: `apps/basecamp` declares `delivery_module`,
  builds with the real module dependency, and wires create, start, subscribe,
  send, receive, and verify for the same V1 proof-message bytes
- third local reference integration: `examples/fee-tier-gate`
- Spike 09 PPE-native gate is available from the root demo entrypoint via
  `./demo.sh --ppe-gate --real-prover`; it writes a local benchmark report for
  wallet health, build, deploy, funding, positive admit, duplicate rejection,
  and insufficient-balance rejection
- public LEZ testnet evidence was recorded on 2026-06-01 for both the Workable
  host-preverified gate and the Spike 09 PPE-native candidate; see
  `docs/TESTNET_DEPLOYMENT.md`
- clean-room evaluator runner: `scripts/demo-clean-room.sh` runs public
  hygiene, quick proof/verify, local Messaging, and optional live Workable/PPE
  paths with one consolidated report
- submission-readiness docs: `docs/SUBMISSION_STATUS.md`,
  `docs/PR_REOPEN_MESSAGE.md`, and `docs/DEMO_VIDEO_SCRIPT.md`
- evaluator-visible root artifacts: `demo.sh`,
  `balance-attestation-verifier.idl.json`, and `apps/basecamp/module.json`
- Basecamp guided-flow polish: stepper-only workflow, contextual Delivery
  actions, clearer `Real proving` / `Dev proving` labeling, visible in-progress
  node pulse, and local RISC Zero recursion artifact cache support for proof
  generation from the UI

Current command set:

```sh
source scripts/env.example
cargo test
scripts/check-public-clean-room.sh
./demo.sh --clean-room --real-prover
scripts/check-basecamp-package.sh
scripts/run-basecamp-local.sh --reset --real-prover
./demo.sh --ppe-gate --real-prover
scripts/m2-check-lez-commitment-compat.sh
cargo run -p attestation-cli -- inspect-private --account Private/<id> --local-only
cargo run -p attestation-cli -- inspect-private --account Private/<id> --require-proof
PRIVATE_ACCOUNT=Private/<id> scripts/prepare-local-private-account.sh
PRIVATE_ACCOUNT=Private/<id> scripts/demo-local-sequencer-e2e.sh
RUN_DIR=.demo-runs/local-sequencer/<run> scripts/demo-local-gate-e2e.sh
./demo.sh --quick
./demo.sh --messaging
PRIVATE_ACCOUNT=Private/<id> ./demo.sh --full
scripts/clean-local-artifacts.sh
cd apps/basecamp && nix build .#install
```

## Latest PR Feedback Triage

The Lambda Prize PR feedback called out five required gaps before resubmission.
Track them here as the submission checklist of record:

| Feedback item | Current state | Next action |
| --- | --- | --- |
| On-chain LEZ proof verification is missing. | Not closed. The repo has two candidate paths: Workable host-preverified gate and Spike 09 PPE-native gate. Public external receipt verification remains unsupported in the tested LEZ runtime. | Get evaluator confirmation that the PPE-native path satisfies LP-0005, or implement the supported public receipt-verifier path if Logos provides one. Keep the Workable path framed as evidence, not final proof verification. |
| Off-chain transmission over Logos Messaging does not seem to work. | Improved. CLI local JSON transport works; Basecamp now uses the real `delivery_module` dependency and manual single-instance send/receive/verify has passed locally. | Record a two-instance Basecamp Delivery walkthrough with sender/receiver evidence and keep `docs/LOGOS_DELIVERY.md` updated with exact commands and screenshots/log notes. |
| CU cost documentation is missing. | Still open. `docs/BENCHMARKS.md` has wall-clock local/testnet timings, not chain CU. | Use upstream LEZ `tools/cycle_bench` or any exposed testnet/devnet CU source to measure the accepted path. Document `register_presenter`, `init_gate`, `admit`, duplicate rejection, and PPE-native positive/rejection costs if applicable. |
| E2E-vs-sequencer in CI is missing. | Partial. Live sequencer E2E scripts pass locally, but CI only runs workspace/deployable checks. | Add a CI job that starts standalone LEZ, initializes a fresh wallet/private account, runs a bounded `RISC0_DEV_MODE=1` local sequencer E2E, and uploads sanitized reports. Keep `RISC0_DEV_MODE=0` for manual/video evidence if CI runtime is too expensive. |
| YouTube video is missing. | Open. `docs/DEMO_VIDEO_SCRIPT.md` exists. | Record the final narrated video after on-chain-path wording is settled enough to avoid overstating the solution. Must show `RISC0_DEV_MODE=0`, CLI proof generation, Basecamp Delivery, and accepted on-chain path evidence. |

## Latest Upstream Refresh

Checked on 2026-07-09 with non-destructive `git fetch` only:

- `lambda-prize upstream/master`: LP-0005 is still open and still requires
  on-chain LEZ proof verification, off-chain proof transmission over Logos
  Messaging, CU documentation, sequencer E2E in CI, and a narrated video. The
  integration requirement is now easier for this repo: it asks for a
  standalone consumer integration demo, not three integrations with an external
  party. The repo already has `examples/governance-gate`, `examples/chat-gate`,
  and `examples/fee-tier-gate`; feature one clearly in the video.
- `logos-docs origin/main`: official docs now frame Messaging transport through
  Logos Delivery. The documented C++ module path uses `delivery_module`
  pinned to `github:logos-co/logos-delivery-module/v0.1.3`, declares the
  dependency in metadata, registers events before `start()`, subscribes before
  sending, and uses a LIP-23 content topic. This matches the current Basecamp
  adapter shape; the missing evidence is a two-instance Delivery recording.
- `logos-basecamp origin/master` / release `0.2.1`: Basecamp changed
  substantially, but the important `ui_qml` contract remains compatible with
  this repo: `view` is the QML entry point and core dependencies are loaded
  before the UI plugin. Before final video, re-test the module against the
  latest local Basecamp checkout rather than relying only on the older dev app.
- `logos-execution-zone upstream/main`: do not blindly pull into the current
  working checkout. It is dirty with wallet/RocksDB state and local spike
  sources, and upstream has breaking layout/naming changes, including
  `nssa` -> `lee`. The current commitment formula still uses the
  `/LEE/v0.3/Commitment/` domain separator and private `account_id`, which
  matches this repo's production circuit. No direct public LEZ external receipt
  verification support was found in the fetched upstream tree. Upstream does
  include `tools/cycle_bench` and `docs/benchmarks/cycle_bench.md`, which
  should be the model for LP-0005 CU/cycle reporting.

## Ordered Backlog

1. Resolve the evaluator-approved live on-chain path.
   - Wait for evaluator/Discord answer if possible, but do not block unrelated
     polish or evidence gathering.
   - Keep direct public receipt verification marked unsupported for this LEZ
     version.
   - Keep the host-preverified Workable path documented unless evaluators
     request a different native LEZ pattern.
   - Spike 09 now validates the Logos-native private execution/PPE candidate
     locally and on public testnet with `RISC0_DEV_MODE=0`: private balance
     check, public gate/nullifier update, duplicate rejection, and
     insufficient-balance rejection all work.
   - Ask evaluators whether this PPE-native path satisfies LP-0005's on-chain
     verifier requirement, or whether the final submission must still use a
     public LEZ program that verifies an externally supplied receipt.

2. Record and harden the Logos Delivery path.
   - Current deterministic adapter: `attestation-messaging::LocalFileTransport`.
   - Current Basecamp adapter: `delivery_module` create, start, subscribe,
     send, receive, and verify.
   - Run a two-instance Basecamp Delivery pass and capture the received
     `proof-message.json`, `message-verify` output, and UI notes.
   - Keep the proof message bytes stable across local JSON and Delivery.

3. Add CU / cycle measurements for the accepted path.
   - CU measurements for the operations already listed in
     `docs/BENCHMARKS.md`.
   - Use upstream LEZ `tools/cycle_bench` as the model for cycle/CU-style
     reporting where possible.
   - If true chain CU is not exposed, document the exact limitation and include
     the closest available cycle/executor metrics without relabeling them as CU.

4. Add live local-sequencer E2E to CI if practical.
   - Prefer a fast `RISC0_DEV_MODE=1` CI job for wallet/sequencer/API drift.
   - Upload sanitized `.demo-runs/.../report.md` artifacts.
   - Keep real-prover `RISC0_DEV_MODE=0` in manual evidence and video unless
     CI runtime is acceptable.

5. Turn the successful `RISC0_DEV_MODE=0` run into final demo evidence.
   - Record a clean-room narrated run.
   - Run `scripts/check-wallet-preflight.sh` before recording.
   - Run `./demo.sh --clean-room --real-prover --with-tests --with-lez`
     before publishing demo artifacts.
   - Keep `witness.json` private and publish only envelope/report artifacts.

6. Harden Basecamp GUI.
   - Run a final manual end-to-end UX pass from inside Basecamp for recording.
   - Run `scripts/check-basecamp-package.sh` before manual QA.
   - Keep the `nix build .#install` packaging path aligned with the active
   Basecamp build.
   - Keep the UI limited to public/sanitized proof state.

7. Submission hardening.
   - Keep the Lambda Prize PR title exactly
     `Solution: LP-0005 — Private Token Balance Attestation`.
   - Do not reopen as a draft submission; use Discord or a separate discussion
     for the on-chain-path question.
   - Keep root `demo.sh`, root `*.idl.json`, and Basecamp `module.json`
     visible for automated checks.
   - Final submission should include testnet deployment details, CU docs,
     clean local E2E evidence, and narrated video.
