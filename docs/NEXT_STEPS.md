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
- local/pluggable Messaging transport and token-gated admission flow:
  `attestation-messaging`, CLI `message-*` commands, `./demo.sh --messaging`,
  and `examples/chat-gate`
- third local reference integration: `examples/fee-tier-gate`
- Spike 09 PPE-native gate is available from the root demo entrypoint via
  `./demo.sh --ppe-gate --real-prover`; it writes a local benchmark report for
  wallet health, build, deploy, funding, positive admit, duplicate rejection,
  and insufficient-balance rejection
- clean-room evaluator runner: `scripts/demo-clean-room.sh` runs public
  hygiene, quick proof/verify, local Messaging, and optional live Workable/PPE
  paths with one consolidated report
- evaluator-visible root artifacts: `demo.sh`,
  `balance-attestation-verifier.idl.json`, and `apps/basecamp/module.json`

Current command set:

```sh
source scripts/env.example
cargo test
scripts/check-public-clean-room.sh
scripts/demo-clean-room.sh --real-prover
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

## Ordered Backlog

1. Turn the successful `RISC0_DEV_MODE=0` run into final demo evidence.
   - Record a clean-room narrated run.
   - Run `scripts/check-wallet-preflight.sh` before recording.
   - Run `scripts/demo-clean-room.sh --real-prover --with-tests --with-lez`
     before publishing demo artifacts.
   - Keep `witness.json` private and publish only envelope/report artifacts.

2. Resolve the evaluator-approved live on-chain path.
   - Wait for evaluator/Discord answer if possible.
   - Keep direct public receipt verification marked unsupported for this LEZ
     version.
   - Keep the host-preverified Workable path documented unless evaluators
     request a different native LEZ pattern.
   - Spike 09 now validates the Logos-native private execution/PPE candidate
     locally with `RISC0_DEV_MODE=0`: private balance check, public
     gate/nullifier update, duplicate rejection, and insufficient-balance
     rejection all work against the local sequencer.
   - Ask evaluators whether this PPE-native path satisfies LP-0005's on-chain
     verifier requirement, or whether the final submission must still use a
     public LEZ program that verifies an externally supplied receipt.

3. Replace local Messaging transport with the accepted real adapter if required.
   - Current local adapter: `attestation-messaging::LocalFileTransport`.
   - Current CLI flow: `message-export`, `message-receive`, `message-verify`,
     `message-admit`.
   - Keep the proof message bytes stable when wiring the real Logos Messaging
     transport.

4. Harden Basecamp GUI.
   - Run a final manual end-to-end UX pass from inside Basecamp for recording.
   - Keep the `nix build .#install` packaging path aligned with the active
     Basecamp build.
   - Keep the UI limited to public/sanitized proof state.

5. Add final submission support.
   - External partner/integrator for at least one reference integration.
   - Live LEZ testnet deployment and program id docs.
   - Devnet/testnet CU measurements for the operations already listed in
     `docs/BENCHMARKS.md`.
   - Narrated demo video with `RISC0_DEV_MODE=0`.

6. Submission hardening.
   - Keep the Lambda Prize PR title exactly
     `Solution: LP-0005 — Private Token Balance Attestation`.
   - Do not reopen as a draft submission; use Discord or a separate discussion
     for the on-chain-path question.
   - Keep root `demo.sh`, root `*.idl.json`, and Basecamp `module.json`
     visible for automated checks.
   - Final submission should include testnet deployment details, CU docs,
     clean local E2E evidence, and narrated video.
