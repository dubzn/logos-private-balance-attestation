# Submission Status

Last updated: 2026-07-13.

This repository is a public technical implementation for LP-0005. It is close
to a submission package, but it should not be represented as an award-ready
solution until the on-chain verifier path and narrated demo are complete. LEZ
guest cycles and a live-sequencer CI job are now implemented; evaluator
confirmation is still needed on whether those cycles satisfy the LP's CU
wording. Public testnet deployment evidence exists for both candidate paths.

## Current Readiness

| Area | Status | Evidence |
| --- | --- | --- |
| Root demo script | ready for local review | `demo.sh` defaults to `--clean-room` and also supports `--quick`, `--messaging`, `--full`, and `--ppe-gate`. |
| Circuit/proof | ready locally | `methods/` proves LEZ private commitment membership, `balance >= threshold`, context binding, presenter binding, and context nullifier derivation. |
| Real wallet witness path | ready locally | `scripts/demo-local-sequencer-e2e.sh` builds a witness from local wallet private state and real `getProofForCommitment`. |
| Off-chain verifier | ready locally | `crates/attestation-verifier` verifies the public proof envelope. |
| Messaging path | validated locally over real Logos Delivery | `crates/attestation-messaging`, `./demo.sh --messaging`, and `examples/chat-gate` cover deterministic local transport. The Basecamp adapter uses `delivery_module`; a two-instance run transferred a 1,323,577-byte real-prover proof message in 17 chunks, reassembled the expected SHA-256, and passed local `message-verify` with `status: ok`. Final narrated recording pending. |
| Workable LEZ gate | testnet evidence / partial | `lez-verifier/program` records/deduplicates gate state after mandatory host-side proof verification. It was deployed and exercised on public LEZ testnet on 2026-06-01. Spike 10 proved direct in-guest receipt verification works cryptographically, but even Groth16 requires 162,362,189 cycles against the current 33,554,432-cycle public limit. |
| PPE-native LEZ gate | testnet evidence / evaluator decision | `./demo.sh --ppe-gate --real-prover` passed locally and the Spike 09 PPE-native flow passed on public LEZ testnet on 2026-06-01 with private balance check, public nullifier write, duplicate rejection, and insufficient-balance rejection. Evaluator acceptance pending because it is not the same portable proof envelope used off-chain. |
| Basecamp | ready as MVP | `apps/basecamp` builds as a backend-backed `ui_qml` plugin, wraps the local CLI/script flows, and includes a Logos Delivery panel backed by `delivery_module`; `scripts/check-basecamp-package.sh` validates the package install tree and `scripts/run-basecamp-local.sh` launches the local app. |
| IDL | present | Root `balance-attestation-verifier.idl.json` points to `idl/balance-attestation-verifier.json`. |
| Consumer integration demo | ready locally | Governance gate, chat gate, and fee-tier gate examples exist and have local tests. This matches the updated LP-0005 standalone consumer integration requirement; final submission should choose the clearest one to feature in the video. |
| Benchmarks | gate cycles documented / chain CU unavailable | `scripts/benchmark-lez-cycles.sh` follows upstream LEZ `tools/cycle_bench` and records deterministic RISC Zero user cycles for register, init, and admit. `docs/BENCHMARKS.md` also records wall-clock evidence and clearly separates unavailable network CU. |
| CI | live sequencer job implemented | In addition to workspace/deployable checks, `.github/workflows/ci.yml` now runs a pinned standalone LEZ sequencer with an ephemeral wallet, real `getProofForCommitment`, dev-mode proof, verification, private cleanup, and sanitized artifacts. Local disposable-clone validation passed; first GitHub run pending. |

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
| Logos Delivery QA | `docs/LOGOS_DELIVERY.md` |
| Local benchmark evidence | `docs/BENCHMARKS.md` |
| Public testnet deployment evidence | `docs/TESTNET_DEPLOYMENT.md` |
| On-chain path analysis | `docs/ONCHAIN_PATH_DECISION.md` |
| Security model | `docs/SECURITY_MODEL.md` |

## What Is Complete Enough To Review

- The proof circuit and public envelope shape.
- The local prover/verifier CLI flow.
- The real local wallet/sequencer witness builder.
- Presenter binding and nullifier behavior.
- Off-chain verification, local Messaging-style admission, and Basecamp
  `delivery_module` send/receive wiring.
- Local Basecamp MVP wiring.
- Basecamp package build/inspection through `scripts/check-basecamp-package.sh`.
- Workable gate-ledger implementation and its documented trust boundary.
- Spike 10 explicit succinct/Groth16 receipt verification and measured public
  LEZ cycle-limit rejection.
- PPE-native LEZ private execution candidate and local benchmark evidence.
- Public LEZ testnet deploy/admit evidence for both Workable and PPE-native
  paths.

## What Must Be Completed Before Reopening A Final Prize PR

1. Get evaluator confirmation on the accepted LP-0005 on-chain verifier model:
   PPE-native private execution, public receipt verification, or another Logos
   pattern.
2. Select the accepted LEZ program path for final submission. Both current
   candidate paths have public testnet evidence in
   `docs/TESTNET_DEPLOYMENT.md`, but the accepted architecture is still open.
3. Record a two-instance Basecamp Delivery walkthrough showing proof send,
   receive, local verification, and token-gated admission semantics.
4. Confirm that the documented RISC Zero user cycles satisfy the evaluator's
   CU requirement, or capture network CU if Logos exposes that metric.
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

Spike 09 is the stronger current candidate:

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
