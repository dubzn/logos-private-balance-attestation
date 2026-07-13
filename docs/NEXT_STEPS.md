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
- `scripts/benchmark-lez-cycles.sh` follows upstream LEZ `tools/cycle_bench`
  and records deterministic RISC Zero user cycles for register, init, and
  admit; `docs/BENCHMARKS.md` keeps those distinct from unavailable network CU
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
- latest full local E2E against a clean latest LEZ checkout passed with
  `RISC0_DEV_MODE=0`: proof phase 00:01:49, gate phase 00:01:47, total
  00:03:36, verify `ok`, nullifier count `1`, duplicate admit `not-applied`
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
- two-instance Basecamp Logos Delivery passed with a 1,323,577-byte real-prover
  message split into 17 chunks, out-of-order reassembly, matching SHA-256, and
  local proof verification with `status: ok`
- `.github/workflows/ci.yml` now includes a pinned standalone-sequencer E2E;
  `scripts/ci-live-sequencer-e2e.sh` passed against a disposable LEZ clone with
  an ephemeral wallet, real `getProofForCommitment`, and sanitized outputs
- Spike 10 proved that explicit in-guest receipt verification works without
  assumptions and rejects tampering, but measured 313,056,015 cycles for a
  succinct receipt and 162,362,189 cycles for Groth16; both exceed latest LEZ's
  33,554,432-cycle public execution limit
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
- latest LEZ static compatibility refresh: a clean
  `/Users/dub/Desktop/logos/logos-execution-zone-latest` checkout tracks
  `logos-blockchain/logos-execution-zone` `upstream/dev` at `1b4d8fbc`
  (2026-07-09); RISC Zero versions match, M2 commitment compatibility passes
  byte-for-byte against official `lee_core`, and the deployable LEZ program
  builds/tests against `lee_core` (`cargo test --manifest-path
  lez-verifier/program/Cargo.toml`, 6/6)

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
| On-chain LEZ proof verification is missing. | Not closed. Spike 10 now proves direct portable receipt verification is cryptographically possible without assumptions, but succinct costs 313,056,015 cycles and Groth16 costs 162,362,189 against LEZ's 33,554,432-cycle public limit. The Workable and PPE-native paths remain available. | Ask Logos for a native verifier, assumption channel, higher verifier budget, or explicit confirmation that the PPE-native gate satisfies LP-0005. |
| Off-chain transmission over Logos Messaging does not seem to work. | Closed locally. A two-instance Basecamp run transferred a 1,323,577-byte real-prover envelope over real Logos Delivery in 17 out-of-order chunks, reassembled the expected SHA-256, and verified with `status: ok`. | Capture the same flow in the narrated submission video and link the related SDK issue/PR. |
| CU cost documentation is missing. | Partially closed with the official LEZ metric. `scripts/benchmark-lez-cycles.sh` now reports deterministic RISC Zero user cycles for register, init, and admit. The current RPC does not expose network CU, and failed execution does not expose `SessionInfo`. | Ask evaluators whether this upstream-compatible cycle report satisfies the requirement; instrument the final accepted on-chain path if it changes. |
| E2E-vs-sequencer in CI is missing. | Implemented. The new CI job uses a pinned official LEZ checkout, ephemeral wallet/private account, real `getProofForCommitment`, dev-mode proving, verification, cleanup, and sanitized artifacts. A disposable-clone local run passed. | Push and confirm the first GitHub Actions run; keep `RISC0_DEV_MODE=0` for manual/video evidence. |
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
- `logos-execution-zone upstream/dev`: do not blindly pull into the older
  working checkout. It is dirty with wallet/RocksDB state and local spike
  sources, and upstream has breaking layout/naming changes, including
  `nssa` -> `lee`. A clean latest clone now lives at
  `/Users/dub/Desktop/logos/logos-execution-zone-latest` and should be used for
  migration work. The current commitment formula still uses the
  `/LEE/v0.3/Commitment/` domain separator and private `account_id`, which
  matches this repo's production circuit. Static compatibility against
  `lee_core` is green, and the live wallet/sequencer full E2E now passes
  against that latest checkout. No direct public LEZ external receipt
  verification support was found in the fetched upstream tree. Upstream includes
  `tools/cycle_bench` and `docs/benchmarks/cycle_bench.md`, which should be the
  model for LP-0005 CU/cycle reporting.

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
   - Include Spike 10's measured blocker in that question: direct Groth16
     verification works but uses 162,362,189 cycles under a 33,554,432-cycle
     public limit.

2. Confirm the new evidence in public CI.
   - Push the live-sequencer job and verify its first GitHub Actions run.
   - Keep uploads limited to the sanitized report, verification output, and
     redacted witness summary.
   - Ask evaluators whether upstream-compatible RISC Zero user cycles satisfy
     the LP's CU wording; do not relabel them as network CU.

3. Record the validated Logos Delivery path.
   - Re-run the two-instance Basecamp flow for the narrated video.
   - Show out-of-order chunk reception, matching reassembled SHA-256, local
     proof verification, and token-gated admission semantics.
   - Link the SDK event-payload issue/PR in the submission notes.

4. Turn the successful `RISC0_DEV_MODE=0` run into final demo evidence.
   - Record a clean-room narrated run.
   - Run `scripts/check-wallet-preflight.sh` before recording.
   - Run `./demo.sh --clean-room --real-prover --with-tests --with-lez`
     before publishing demo artifacts.
   - Keep `witness.json` private and publish only envelope/report artifacts.

5. Harden Basecamp GUI.
   - Run a final manual end-to-end UX pass from inside Basecamp for recording.
   - Run `scripts/check-basecamp-package.sh` before manual QA.
   - Keep the `nix build .#install` packaging path aligned with the active
   Basecamp build.
   - Keep the UI limited to public/sanitized proof state.

6. Submission hardening.
   - Keep the Lambda Prize PR title exactly
     `Solution: LP-0005 — Private Token Balance Attestation`.
   - Do not reopen as a draft submission; use Discord or a separate discussion
     for the on-chain-path question.
   - Keep root `demo.sh`, root `*.idl.json`, and Basecamp `module.json`
     visible for automated checks.
   - Final submission should include testnet deployment details, CU docs,
     clean local E2E evidence, and narrated video.
