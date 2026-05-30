# Submission Status

Last updated: 2026-05-20.

This repository is a public technical implementation for LP-0005. It is close
to a submission package, but it should not be represented as an award-ready
solution until the on-chain verifier path, testnet deployment, CU metrics, and
narrated demo are complete.

## Current Readiness

| Area | Status | Evidence |
| --- | --- | --- |
| Root demo script | ready for local review | `demo.sh` defaults to `--clean-room` and also supports `--quick`, `--messaging`, `--full`, and `--ppe-gate`. |
| Circuit/proof | ready locally | `methods/` proves LEZ private commitment membership, `balance >= threshold`, context binding, presenter binding, and context nullifier derivation. |
| Real wallet witness path | ready locally | `scripts/demo-local-sequencer-e2e.sh` builds a witness from local wallet private state and real `getProofForCommitment`. |
| Off-chain verifier | ready locally | `crates/attestation-verifier` verifies the public proof envelope. |
| Messaging path | ready as local/pluggable adapter | `crates/attestation-messaging`, `./demo.sh --messaging`, and `examples/chat-gate` export, receive, verify, admit, and reject duplicate nullifiers over local JSON transport. |
| Workable LEZ gate | partial | `lez-verifier/program` records/deduplicates gate state after mandatory host-side proof verification. It does not verify the RISC Zero receipt inside public LEZ execution. |
| PPE-native LEZ gate | local candidate | `./demo.sh --ppe-gate --real-prover` passed locally with private balance check, public nullifier write, duplicate rejection, and insufficient-balance rejection. Evaluator acceptance pending. |
| Basecamp | ready as MVP | `apps/basecamp` builds as a backend-backed `ui_qml` plugin and wraps the local CLI/script flows; `scripts/check-basecamp-package.sh` validates the package install tree and `scripts/run-basecamp-local.sh` launches the local app. |
| IDL | present | Root `balance-attestation-verifier.idl.json` points to `idl/balance-attestation-verifier.json`. |
| Consumer integration demo | ready locally | Governance gate, chat gate, and fee-tier gate examples exist and have local tests. This matches the updated LP-0005 standalone consumer integration requirement; final submission should choose the clearest one to feature in the video. |
| Benchmarks | local timings only | `docs/BENCHMARKS.md` records local proof/gate/PPE timings. Devnet/testnet CU metrics remain pending. |
| CI | ready for current local scope | Workspace tests and deployable-program checks are covered by `.github/workflows/ci.yml`. |

## One-Command Review Paths

No wallet or sequencer required:

```sh
./demo.sh --clean-room --real-prover
```

Equivalent default shape:

```sh
./demo.sh --real-prover
```

This runs public hygiene checks, quick proof/verify, and the local Messaging
path. It writes a consolidated report under `.demo-runs/clean-room/`.

With a local LEZ sequencer, compatible wallet home, and initialized/funded
private account:

```sh
PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
  ./demo.sh --clean-room --real-prover --with-lez --with-live --with-ppe
```

This adds the live wallet/sequencer E2E and PPE-native gate candidate.

## Artifact Map

| Requirement Surface | Artifact |
| --- | --- |
| Root demo script | `demo.sh` |
| Clean-room runner | `scripts/demo-clean-room.sh` |
| SPEL-style IDL | `balance-attestation-verifier.idl.json` |
| Basecamp metadata | `apps/basecamp/module.json` |
| Basecamp QA | `docs/BASECAMP_QA.md` |
| LP requirement map | `docs/PRIZE_CHECKLIST.md` |
| Public evaluator guide | `docs/EVALUATOR_GUIDE.md` |
| Local benchmark evidence | `docs/BENCHMARKS.md` |
| On-chain path analysis | `docs/ONCHAIN_PATH_DECISION.md` |
| Security model | `docs/SECURITY_MODEL.md` |

## What Is Complete Enough To Review

- The proof circuit and public envelope shape.
- The local prover/verifier CLI flow.
- The real local wallet/sequencer witness builder.
- Presenter binding and nullifier behavior.
- Off-chain verification and local Messaging-style admission.
- Local Basecamp MVP wiring.
- Basecamp package build/inspection through `scripts/check-basecamp-package.sh`.
- Workable gate-ledger implementation and its documented trust boundary.
- PPE-native LEZ private execution candidate and local benchmark evidence.

## What Must Be Completed Before Reopening A Final Prize PR

1. Get evaluator confirmation on the accepted LP-0005 on-chain verifier model:
   PPE-native private execution, public receipt verification, or another Logos
   pattern.
2. Deploy the accepted LEZ program path on Logos devnet/testnet and document
   the program id.
3. Record CU or equivalent chain execution costs for the accepted on-chain
   operations.
4. Add real Logos Messaging network transport if local/pluggable JSON transport
   is not accepted for the off-chain path.
5. Record a narrated demo with `RISC0_DEV_MODE=0` that shows CLI, Basecamp, and
   the accepted on-chain path.

## Current On-Chain Position

The public LEZ program path is intentionally described as
Workable/host-preverified:

```text
host verifies proof envelope
  -> host submits LEZ transaction
  -> LEZ program records/deduplicates the nullifier
```

Spike 08 proved this path is not an in-guest receipt verifier because a
fabricated but well-formed journal can be applied by the host. The
cryptographic trust seat is therefore host-side.

Spike 09 is the stronger local candidate:

```text
private holder account
  -> LEZ privacy-preserving execution checks balance >= threshold
  -> public BAP1 gate/nullifier state is written
```

It passed locally with `RISC0_DEV_MODE=0`, but it does not verify the same
portable proof envelope used off-chain. That is the open evaluator decision.

## Open Evaluator Question

The remaining architectural question for LP-0005 is deliberately narrow:

```text
Should the final on-chain path be the PPE-native LEZ gate, where private
execution proves balance >= threshold and writes public gate/nullifier state,
or must the on-chain path verify the same portable RISC Zero proof envelope
that is used by the off-chain verifier?
```

The repo keeps both paths visible so reviewers can evaluate the tradeoff:

- `docs/ONCHAIN_PATH_DECISION.md` explains why direct public LEZ
  `env::verify(...)` over an externally supplied receipt currently fails
  locally without an assumptions channel.
- `./demo.sh --ppe-gate --real-prover` demonstrates the PPE-native candidate
  with real proving, duplicate rejection, and insufficient-balance rejection.
- `lez-verifier/program/` remains the Workable public gate ledger for the
  host-preverified path.
