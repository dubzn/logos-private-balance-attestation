# LP-0005 Prize Checklist

This checklist maps LP-0005 requirements to planned repository artifacts.

Status legend:

- `planned`: not implemented yet
- `in-progress`: partially implemented
- `done`: implemented and tested
- `risk`: needs technical validation
- `blocker`: must be resolved before lower-risk implementation

## Functionality

| Requirement | Status | Planned Artifact |
| --- | --- | --- |
| Generate client-side proof for `balance >= N` from a shielded token account. | in-progress | Spike 03/04 prove locally; production `attestation-prover` and CLI still needed. |
| Verify without revealing `npk`, exact balance, or private account identity. | in-progress | Spike 04 journal omits private fields and commitment leaf; verifier crate still needed. |
| Bind proof to a context to prevent cross-gate replay. | in-progress | Spike 04 derives `context_id` and context nullifier in-circuit. |
| Bind proof to presenter identity to reduce forwarding. | in-progress | Spike 04 proves synthetic presenter-secret knowledge; wallet-compatible presenter adapter still needed. |
| Target existing LEZ private account commitment format. | in-progress | Spike 03/04 use `nssa_core::Commitment::new` and live membership proofs. |
| On-chain LEZ verifier gates an action. | blocker | Spike 06 says direct public receipt verification is unsupported locally; use private execution fallback pending evaluator confirmation. |
| Off-chain path over Logos Messaging. | planned | Messaging adapter and token-gated group demo. |
| Three distinct apps integrate on testnet, one outside team. | planned | Governance gate, Messaging group gate, third integration. |
| Full docs and clean public repo. | in-progress | Current documentation baseline. |

## Usability

| Requirement | Status | Planned Artifact |
| --- | --- | --- |
| SDK/module for Logos modules. | planned | `attestation-core`, `attestation-verifier`, package docs. |
| Basecamp GUI with local build instructions. | planned | `apps/basecamp`, backend-backed `ui_qml`. |
| SPEL IDL for LEZ program. | planned | `docs/IDL.md` or `lez/verifier-program/idl/`. |

## Reliability

| Requirement | Status | Planned Artifact |
| --- | --- | --- |
| Proof generation failures surface clear errors. | planned | `AttestationError` enum and CLI error mapping. |
| Messaging verification failures do not expose private data. | planned | Sanitized errors and logging policy. |
| Verifier returns deterministic documented error codes. | planned | Error code table shared by CLI and LEZ program. |

## Performance

| Requirement | Status | Planned Artifact |
| --- | --- | --- |
| Document CU cost of on-chain operations. | planned | `docs/BENCHMARKS.md` after LEZ verifier exists. |
| Proof generation benchmark. | in-progress | Spike 05 dev/prod baseline passed locally; final benchmark docs still needed. |

## Supportability

| Requirement | Status | Planned Artifact |
| --- | --- | --- |
| Program deployed and tested on devnet/testnet. | planned | Deployment docs and verified program id. |
| E2E tests against standalone LEZ sequencer in CI. | planned | `scripts/demo-e2e.sh` plus CI job. |
| CI green on default branch. | planned | GitHub Actions workflow. |
| README covers CLI and Basecamp for both paths. | in-progress | Current README plus future usage docs. |
| Reproducible demo script with `RISC0_DEV_MODE=0`. | planned | Final `scripts/demo-e2e.sh`. |
| Narrated demo video showing proof generation and dev mode off. | planned | Submission artifact. |

## Submission Blockers To Clear

Before submitting, the repo must prove:

0. RISC Zero proof verification or an evaluator-approved native LEZ private
   execution proof path is feasible for the on-chain gate.
1. Tests pass from a clean checkout.
2. No invented sequencer endpoints are used.
3. The verifier program is deployable to LEZ.
4. The on-chain path verifies the proof, not only a host-side precheck.
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
| On-chain path decision | in-progress | Spike 06 documents no local public external receipt verifier path; evaluator confirmation still needed. |
