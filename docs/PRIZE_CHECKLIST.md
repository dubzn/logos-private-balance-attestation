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
| On-chain LEZ verifier gates an action. | partial / Workable | `lez-verifier/program/` is a deployable LEZ guest (`BALANCE_ATTESTATION_PROGRAM_ID` pinned) that registers presenters, admits a presenter against gate state, and dedups nullifiers. Spike 08 validated local deployment and state updates over the all-public `nssa::PublicTransaction` path, but also showed `admit-fabricated` can be applied if the host submits it. The LEZ program is therefore a gate ledger/nullifier set with host-side proof verification, not yet a cryptographic on-chain proof verifier. CLI `gate-register-presenter`, `gate-init`, and `gate-admit` wrap the live runner; `gate-admit` performs mandatory `attestation_verifier::verify_envelope` precheck before submission. |
| Off-chain path over Logos Messaging. | done (transport-agnostic) | `attestation-verifier` + `examples/chat-gate` (envelope JSON shipped as wire bytes). |
| Three distinct apps integrate on testnet, one outside team. | partial | `examples/governance-gate` + `examples/chat-gate` shipped; third + external integrator still pending. |
| Full docs and clean public repo. | in-progress | README + `docs/`, IDL artifact, smoke demo script, CI; final testnet deployment docs and Basecamp docs pending. |

## Usability

| Requirement | Status | Artifact |
| --- | --- | --- |
| SDK/module for Logos modules. | done | `crates/attestation-sdk/` umbrella; off-chain default + `on-chain` feature. |
| Basecamp GUI with local build instructions. | planned | `apps/basecamp`, backend-backed `ui_qml`. |
| SPEL IDL for LEZ program. | done | `idl/balance-attestation-verifier.json`; `docs/IDL_DRAFT.md` is the prose companion. |

## Reliability

| Requirement | Status | Artifact |
| --- | --- | --- |
| Proof generation failures surface clear errors. | done | `ProveError`, `LezGateError` carry structured detail; CLI maps to non-zero exit + message. |
| Messaging verification failures do not expose private data. | done | `VerifyError::*` carries no journal/witness internals; envelope is public by construction. |
| Verifier returns deterministic documented error codes. | done | `AttestationErrorCode` BAxxx codes exposed via `VerifyError::code()` and `LezGateProgramError`. |

## Performance

| Requirement | Status | Artifact |
| --- | --- | --- |
| Document CU cost of on-chain operations. | planned | `docs/BENCHMARKS.md` after live LEZ deploy. |
| Proof generation benchmark. | in-progress | Local sequencer E2E `RISC0_DEV_MODE=0` after challenge binding: build witness 00:01:01, prove 00:00:23, verify 00:00:03, total 00:01:30. Final benchmark doc still pending. |

## Supportability

| Requirement | Status | Artifact |
| --- | --- | --- |
| Program deployed and tested on devnet/testnet. | partial | Spike 08 deployed the current local build via `wallet deploy-program` on 2026-05-05 and included register/init/admit blocks. Current source pin is `BALANCE_ATTESTATION_PROGRAM_ID = [1814270443, 2715932601, 2328945781, 1735716110, 1384216943, 615796249, 572526872, 2343830086]` (base58 `GrZAsBnfyJ2VXXaxzz2Sd3oUxZ89KWL23PnY321Z1h3x`). Pin tested in `lez-verifier/program/tests/program_journal_roundtrip.rs::program_id_is_pinned`. Public testnet deployment still pending. |
| E2E tests against standalone LEZ sequencer in CI. | partial | Workspace E2E suites are in-memory/synthetic. `scripts/demo-local-sequencer-e2e.sh` now exercises wallet + real `getProofForCommitment`; CI/local clean-run automation still pending. |
| CI green on default branch. | done locally / pending remote confirmation | `.github/workflows/ci.yml`: fmt + clippy + workspace tests (default + `--include-ignored`) plus isolated deployable-program checks. Needs GitHub run after merge. |
| README covers CLI and Basecamp for both paths. | done (CLI side) | `README.md` quick-start; Basecamp section pending the GUI. |
| Reproducible demo script with `RISC0_DEV_MODE=0`. | partial | `scripts/demo-local-sequencer-e2e.sh` passed locally with wallet state + real `getProofForCommitment` + challenge-bound verify `status: ok`. Clean-room rerun/video still pending. |
| Narrated demo video showing proof generation and dev mode off. | planned | Submission artifact. |

## Submission Blockers To Clear

Before submitting, the repo must prove:

0. RISC Zero proof verification or an evaluator-approved native LEZ private
   execution proof path is feasible for the on-chain gate.
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
| Logos-native private execution gate | passed locally | Useful fallback, evaluator acceptance pending. |
| Real `getProofForCommitment` path | passed locally | Spike 02 fetched live membership proof. |
| Standalone balance circuit | passed locally | Spike 03 proved threshold + commitment + Merkle path. |
| Binding/nullifier circuit | passed locally | Spike 04 adds presenter binding, context binding, and nullifier. |
| Dev/prod proving baseline | passed locally | Spike 05 proved fixture with `RISC0_DEV_MODE=0`. |
| On-chain path decision | passed locally | Spike 06 documents no local public external receipt verifier path; evaluator confirmation still needed. |
| `attestation-core` workspace | passed locally | M1 started with context hashing, nullifier/presenter helpers, envelope/journal types, and error-code tests. |
| LEZ commitment helper compatibility | passed locally | M2 script compares commitment, leaf hash, and Merkle root helpers against local `nssa_core`. |
| Sanitized private account inspect | passed local-only | M2 script reads wallet private state and checks commitment reconstruction without printing witness data. |
| Real proof-mode private inspect | passed locally | M2 script calls `WalletCore::check_private_account_initialized` and verifies core root matches wallet root. |
| `attestation-prover` initial crate | passed locally | Reusable sanitized report/redaction logic has unit coverage. |
| Initial witness builder | passed locally | `attestation-prover` builds witness fields from private account, membership proof, context, and presenter inputs with redacted debug/summary output. |
| `attestation-cli inspect-private` | passed locally | CLI wraps the M2 adapter and returns sanitized JSON for local-only and require-proof modes. |
| Local sequencer E2E harness | passed locally (dev + real proving) | `scripts/demo-local-sequencer-e2e.sh` produced envelope + verify `status: ok` from real wallet state and `getProofForCommitment` with `RISC0_DEV_MODE=1` and `RISC0_DEV_MODE=0`. Latest real-prover run: 00:01:30 total, 00:00:23 proving, 1.3 MB public envelope. |
| Deployable LEZ program | passed locally | `lez-verifier/program/` builds against `nssa_core` from the LEZ checkout; `cargo test --manifest-path lez-verifier/program/Cargo.toml --test program_journal_roundtrip` is 6/6 (register presenter + init + admit happy path + BA206 replay + BA203 wrong presenter + BA102 wrong inner image id; image id pinned). Live deploy + admit succeeded locally, but receipt binding remains host-side. |
| Spike 08 — program-chaining receipt-binding probe | passed locally — Workable | Run on 2026-05-05 against local sequencer. `register_presenter` stored pubkey data, `init_gate` flipped gate-state account to program-owned with BAT1 magic + full GateState layout, real-shape `admit` applied nullifier `aa`, duplicate admit did not apply a second copy after block settle, and `admit-fabricated` applied nullifier `bb`. Result: host pre-verification (`attestation_verifier::verify_envelope`) is the documented trust seat for the on-chain path. See `spikes/spike-08-program-chaining/README.md`. |
