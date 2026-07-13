# LP-0005 Prize Checklist

This checklist maps LP-0005 requirements to planned repository artifacts.

Status legend:

- `planned`: not implemented yet
- `in-progress`: partially implemented
- `done`: implemented and tested
- `risk`: needs technical validation
- `blocker`: must be resolved before lower-risk implementation

## Functionality

| Requirement | Status | Artifact |
| --- | --- | --- |
| Generate client-side proof for `balance >= N` from a shielded token account. | done | `attestation-prover::prove_attestation` over `methods/` production circuit. |
| Verify without revealing `npk`, exact balance, or private account identity. | done | Journal commits only public fields; `attestation-verifier::verify_envelope`. |
| Bind proof to a context to prevent cross-gate replay. | done | `attestation_core::derive_context_id` over `(chain_id, circuit_image_id, verifier_id, gate_id, threshold)`; circuit asserts. |
| Bind proof to presenter identity to reduce forwarding. | done (V1) | BIP-340 Schnorr: `presenter_id = H(pubkey)` in circuit; envelope signature covers `presentation_digest(journal.digest(), presentation_challenge)`. Verifiers must generate fresh challenges per session. |
| Target existing LEZ private account commitment format. | done | `attestation_core::derive_lez_private_account_commitment` mirrors `nssa_core` byte-for-byte (compat script + tests). |
| On-chain LEZ verifier gates an action. | partial / evaluator decision | Two paths are implemented and have public testnet evidence. Path A is the Workable public gate ledger: `lez-verifier/program/` registers presenters, admits against gate state, and dedups nullifiers after mandatory host-side `attestation_verifier::verify_envelope` precheck; Spike 08 showed fabricated journals can be applied if the host submits them, so this is not an in-guest receipt verifier. Path B is Spike 09 PPE-native gate: a LEZ privacy-preserving transaction checks private `balance >= threshold` and writes public `BAP1` gate/nullifier state; local and public testnet `RISC0_DEV_MODE=0` runs passed positive admit, `BA206` duplicate rejection, and `BA201` insufficient-balance rejection. Evaluator confirmation is still required because Path B does not verify the same portable off-chain proof envelope. |
| Off-chain path over Logos Messaging. | done locally / live Delivery validated | `attestation-verifier` + `attestation-messaging` + `examples/chat-gate`: envelope JSON is wrapped as a proof message, received/imported, verified locally, and admitted into a local token-gated group state. `apps/basecamp` uses the real `delivery_module` for create/start/subscribe/chunked-send/receive/reassemble/verify. A two-instance Basecamp run transferred a 1,323,577-byte real-prover message in 17 chunks, reassembled the same SHA-256 despite out-of-order arrival, and passed `message-verify` with `status: ok`. The temporary SDK event-payload fix is tracked in `logos-cpp-sdk#99`; final narrated recording is still pending. |
| Standalone consumer integration demo. | done locally | Updated LP-0005 now asks for a standalone consumer integration demo. The repo ships three runnable local integrations: `examples/governance-gate`, `examples/chat-gate`, and `examples/fee-tier-gate`; final submission should feature the clearest one in the narrated demo. |
| Full docs and clean public repo. | in-progress | README + `docs/`, `docs/SUBMISSION_STATUS.md`, root `demo.sh`, root `balance-attestation-verifier.idl.json`, Basecamp `module.json`, CI, Basecamp MVP docs, clean-room checks, `scripts/demo-clean-room.sh`, and `docs/TESTNET_DEPLOYMENT.md`; CU metrics and video artifacts pending. |

## Usability

| Requirement | Status | Artifact |
| --- | --- | --- |
| SDK/module for Logos modules. | done | `crates/attestation-sdk/` umbrella; off-chain default + `on-chain` feature. |
| Basecamp GUI with local build instructions. | done (MVP) | `apps/basecamp/`: backend-backed `ui_qml` MVP that wraps preflight, proof generation, envelope verification, Workable gate admit flow, and Logos Delivery send/receive. The Nix install target builds a Qt plugin plus replica factory; `scripts/check-basecamp-package.sh` inspects the package tree and asserts the `delivery_module` dependency, `scripts/run-basecamp-local.sh` launches the local app, and `docs/BASECAMP_QA.md` documents manual QA. Final recorded walkthrough still pending. |
| SPEL IDL for LEZ program. | done (SPEL-style JSON) | `balance-attestation-verifier.idl.json` at the repository root points to `idl/balance-attestation-verifier.json`; `docs/IDL_DRAFT.md` is the prose companion. The local LEZ deployment path does not consume SPEL directly, so this remains a public contract artifact unless Logos requires regeneration with specific tooling. |

## Reliability

| Requirement | Status | Artifact |
| --- | --- | --- |
| Proof generation failures surface clear errors. | done | `ProveError`, `LezGateError` carry structured detail; CLI maps to non-zero exit + message. |
| Messaging verification failures do not expose private data. | done | `MessagingError` maps transport/decode failures to BA400/BA401 and wraps `VerifyError::*` without exposing witness internals; messages carry only the public envelope. |
| Verifier returns deterministic documented error codes. | done | `AttestationErrorCode` BAxxx codes exposed via `VerifyError::code()` and `LezGateProgramError`. |

## Performance

| Requirement | Status | Artifact |
| --- | --- | --- |
| Document CU cost of on-chain operations. | partial / official cycle metric documented | `scripts/benchmark-lez-cycles.sh` measures deterministic RISC Zero `SessionInfo::cycles()` for the deployable gate: register `113,667`, init `173,308`, first admit `234,496`, and admit after 10 nullifiers `385,290` user cycles. `docs/BENCHMARKS.md` distinguishes these from unavailable per-transaction network CU and records the `BA206` rejection limitation. Final accepted-path/evaluator confirmation remains open. |
| Proof generation benchmark. | in-progress | `docs/BENCHMARKS.md`: latest full local E2E against current `lee_core` / `lez/wallet` layout with `RISC0_DEV_MODE=0`: proof phase 00:01:49, gate phase 00:01:47, total 00:03:36; proof substeps include build witness 00:01:19, prove 00:00:24, verify 00:00:03. Spike 09 PPE-native benchmark also records local and public testnet positive admit, duplicate rejection, and insufficient-balance rejection timings. |

## Supportability

| Requirement | Status | Artifact |
| --- | --- | --- |
| Program deployed and tested on devnet/testnet. | partial / testnet evidence recorded | Public LEZ testnet runs on 2026-06-01 exercised both current candidate paths. Workable path: program id `[132217456, 1022839988, 453626531, 1132126476, 3831540952, 652159567, 3230477833, 332845675]`, register/init/admit tx hashes, account readback, and duplicate state check. PPE-native path: program id `[411233179, 1364156298, 540691977, 2827788750, 2267394079, 1887386629, 231760863, 2207022502]`, positive admit tx hash, and `BA206`/`BA201` rejection evidence. See `docs/TESTNET_DEPLOYMENT.md`. Final accepted path still requires evaluator confirmation. |
| E2E tests against standalone LEZ sequencer in CI. | implemented / first GitHub run pending | `.github/workflows/ci.yml` now starts a pinned official standalone LEZ checkout, creates an ephemeral wallet/private account, funds it, calls the real `getProofForCommitment` path, proves in `RISC0_DEV_MODE=1`, verifies, and uploads only sanitized reports. `scripts/ci-live-sequencer-e2e.sh` passed locally against a disposable clone on port 3041. |
| CI green on default branch. | implemented / first GitHub run pending | `.github/workflows/ci.yml`: fmt, clippy, workspace tests, isolated deployable-program checks, and live standalone-sequencer E2E. Local disposable-clone validation passed; GitHub status must be confirmed after push. |
| README covers CLI and Basecamp for both paths. | done for current flows | `README.md`, `docs/EVALUATOR_GUIDE.md`, `apps/basecamp/README.md`, and `docs/TESTNET_DEPLOYMENT.md` cover quick, messaging, full local sequencer, Workable gate, Basecamp, and public testnet evidence. |
| Reproducible demo script with `RISC0_DEV_MODE=0`. | partial | Root `demo.sh` defaults to `--clean-room` and also supports `--quick`, `--messaging`, `--full`, and `--ppe-gate`; `scripts/demo-clean-room.sh` composes evaluator-style quick/messaging and optional live paths into one report. `scripts/demo-local-full-e2e.sh` passed locally on 2026-07-10 against a clean latest LEZ checkout using current `lee_core` / `lez/wallet`, wallet state + real `getProofForCommitment`, proof verify `status: ok`, LEZ gate admit, nullifier persisted, and duplicate admit `not-applied`. `./demo.sh --ppe-gate --real-prover` passed locally on 2026-05-19 with `RISC0_DEV_MODE=0`, positive PPE admit, `BA206`/no-second-nullifier duplicate behavior, and `BA201` insufficient-balance rejection. Clean-room video still pending. |
| Narrated demo video showing proof generation and dev mode off. | planned | Submission artifact. |

## Submission Blockers To Clear

Before submitting, the repo must prove:

0. Evaluators confirm whether the Spike 09 native LEZ private execution proof
   path satisfies the on-chain gate requirement, or provide the supported path
   for a public LEZ program to verify an externally submitted receipt.
1. Tests pass from a clean checkout.
2. No invented sequencer endpoints are used.
3. The verifier program is deployable to LEZ.
4. The on-chain path verifies the proof, or evaluators explicitly accept the
   host-preverified Workable model with its limitations.
5. The demo runs against a live local sequencer.
6. The final demo uses `RISC0_DEV_MODE=0`.

## Current Spike Evidence

| Evidence | Status | Notes |
| --- | --- | --- |
| Direct public LEZ receipt verification | failed/currently unsupported | Runtime lacks receipt assumption channel for `env::verify`. |
| Logos-native private execution gate | passed locally and on public testnet | Useful fallback/candidate, evaluator acceptance pending. |
| Real `getProofForCommitment` path | passed locally | Spike 02 fetched live membership proof. |
| Standalone balance circuit | passed locally | Spike 03 proved threshold + commitment + Merkle path. |
| Binding/nullifier circuit | passed locally | Spike 04 adds presenter binding, context binding, and nullifier. |
| Dev/prod proving baseline | passed locally | Spike 05 proved fixture with `RISC0_DEV_MODE=0`. |
| On-chain path decision | passed locally | Spike 06 documents no local public external receipt verifier path; evaluator confirmation still needed. |
| PPE-native balance gate | passed locally and on public testnet | Spike 09 runs a LEZ privacy-preserving transaction that checks private `balance >= threshold`, writes public `BAP1` gate/nullifier state, rejects duplicate admit with `BA206`, and rejects insufficient balance with `BA201`. Latest public testnet run used `RISC0_DEV_MODE=0`; evaluator acceptance pending because this is not the same portable envelope used off-chain. |
| `attestation-core` workspace | passed locally | M1 started with context hashing, nullifier/presenter helpers, envelope/journal types, and error-code tests. |
| LEZ commitment helper compatibility | passed locally | M2 script compares commitment, leaf hash, and Merkle root helpers against local LEZ core helpers (`nssa_core` on older checkouts, `lee_core` on current checkouts). |
| Sanitized private account inspect | passed local-only | M2 script reads wallet private state and checks commitment reconstruction without printing witness data. |
| Real proof-mode private inspect | passed locally | M2 script calls `WalletCore::check_private_account_initialized` and verifies core root matches wallet root. |
| `attestation-prover` initial crate | passed locally | Reusable sanitized report/redaction logic has unit coverage. |
| Initial witness builder | passed locally | `attestation-prover` builds witness fields from private account, membership proof, context, and presenter inputs with redacted debug/summary output. |
| `attestation-cli inspect-private` | passed locally | CLI wraps the M2 adapter and returns sanitized JSON for local-only and require-proof modes. |
| Local sequencer E2E harness | passed locally (dev + real proving) | `scripts/demo-local-sequencer-e2e.sh` produced envelope + verify `status: ok` from real wallet state and `getProofForCommitment` with `RISC0_DEV_MODE=1` and `RISC0_DEV_MODE=0`. Latest real-prover run against current `lee_core` / `lez/wallet` layout: 00:01:48 total, 00:00:24 proving, 1.3 MB public envelope. |
| Local gate E2E harness | passed locally | `scripts/demo-local-gate-e2e.sh` consumes a local-sequencer `envelope.json` + `gate.json`, creates fresh public accounts, registers presenter, initializes gate, submits host-verified admit, and asserts the nullifier is present in gate `account.data`. Latest run: 2026-07-10 against current `lee` layout, `RISC0_DEV_MODE=0`, total 00:01:47, nullifier count `1`, duplicate admit `not-applied`, report under `.demo-runs/local-full/20260710T225915Z/gate/report.md`. |
| Full local E2E harness | passed locally | `scripts/demo-local-full-e2e.sh` composes `demo-local-sequencer-e2e.sh` and `demo-local-gate-e2e.sh` into one operator command. Latest run: 2026-07-10 against a clean latest LEZ checkout, `RISC0_DEV_MODE=0`, proof phase 00:01:49, gate phase 00:01:47, total 00:03:36, verify `ok`, nullifier count `1`, duplicate admit `not-applied`, report under `.demo-runs/local-full/20260710T225915Z/report.md`. |
| Deployable LEZ program | passed locally and on public testnet | `lez-verifier/program/` builds against LEZ core helpers (`nssa_core` on older checkouts, `lee_core` on current checkouts); `cargo test --manifest-path lez-verifier/program/Cargo.toml --test program_journal_roundtrip` is 6/6 (register presenter + init + admit happy path + BA206 replay + BA203 wrong presenter + BA102 wrong inner image id; image id matches embedded ELF). Public testnet deploy/admit succeeded on 2026-06-01, but receipt binding remains host-side. |
| Spike 08 — program-chaining receipt-binding probe | passed locally — Workable | Run on 2026-05-05 against local sequencer. `register_presenter` stored pubkey data, `init_gate` flipped gate-state account to program-owned with BAT1 magic + full GateState layout, real-shape `admit` applied nullifier `aa`, duplicate admit did not apply a second copy after block settle, and `admit-fabricated` applied nullifier `bb`. Result: host pre-verification (`attestation_verifier::verify_envelope`) is the documented trust seat for the on-chain path. See `spikes/spike-08-program-chaining/README.md`. |
