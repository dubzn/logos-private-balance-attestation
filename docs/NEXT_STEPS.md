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
- latest full local E2E against the synced LEZ fork passed with
  `RISC0_DEV_MODE=0`: proof phase 00:01:48, gate phase 00:01:42, total
  00:03:30, verify `ok`, nullifier count `1`, duplicate admit `not-applied`
- `apps/basecamp/` contains a backend-backed `ui_qml` MVP that wraps preflight,
  proof generation, envelope verification, and Workable gate admit
- the local `logos-execution-zone` fork was synced with upstream `main` at
  `41fa494e` on 2026-05-14; the update includes wallet/private-account changes
  and Nix pin fixes, so fresh wallet homes may be needed after switching
  checkouts
- the Basecamp MVP now builds through `mkLogosQmlModule` as a real Qt plugin
  plus replica factory; direct `ui-host` smoke test reached `READY`

Current command set:

```sh
source scripts/env.example
cargo test
scripts/check-public-clean-room.sh
scripts/m2-check-lez-commitment-compat.sh
cargo run -p attestation-cli -- inspect-private --account Private/<id> --local-only
cargo run -p attestation-cli -- inspect-private --account Private/<id> --require-proof
PRIVATE_ACCOUNT=Private/<id> scripts/demo-local-sequencer-e2e.sh
RUN_DIR=.demo-runs/local-sequencer/<run> scripts/demo-local-gate-e2e.sh
PRIVATE_ACCOUNT=Private/<id> ./demo.sh
scripts/clean-local-artifacts.sh
cd apps/basecamp && nix build .#install
```

## Ordered Backlog

1. Turn the successful `RISC0_DEV_MODE=0` run into final demo evidence.
   - Record a clean-room narrated run.
   - Run `scripts/check-wallet-preflight.sh` before recording.
   - Run `scripts/check-public-clean-room.sh --with-tests --with-lez` before
     publishing demo artifacts.
   - Keep `witness.json` private and publish only envelope/report artifacts.

2. Resolve the evaluator-approved live on-chain path.
   - Wait for evaluator/Discord answer if possible.
   - Keep direct public receipt verification marked unsupported for this LEZ
     version.
   - Keep the host-preverified Workable path documented unless evaluators
     request a different native LEZ pattern.

3. Add Messaging transport.
   - Start behind a local adapter.
   - Replace with the accepted Logos Messaging path once confirmed.

4. Harden Basecamp GUI.
   - Run a manual end-to-end UX pass from inside Basecamp.
   - Keep the `nix build .#install` packaging path aligned with the active
     Basecamp build.
   - Keep the UI limited to public/sanitized proof state.

5. Add final submission support.
   - Third reference integration and external partner.
   - Live LEZ testnet deployment and program id docs.
   - Devnet/testnet CU measurements for the operations already listed in
     `docs/BENCHMARKS.md`.
   - Narrated demo video with `RISC0_DEV_MODE=0`.

6. Build the remaining reference integration.
   - Governance/access gate.
   - Messaging group gate.
   - Third integration with external validation.

7. Submission hardening.
    - CI.
    - Clean local E2E.
    - `RISC0_DEV_MODE=0` final demo.
    - Benchmarks and CU docs.
    - SPEL/IDL artifact.
    - Testnet deployment details.
    - Narrated video.
