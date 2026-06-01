# Prize PR Reopen Message

Use this as the initial comment when reopening the LP-0005 solution PR. Keep the
title exactly:

```text
Solution: LP-0005 — Private Token Balance Attestation
```

## Draft Comment

```md
Hi Logos team,

Reopening LP-0005 with a more complete public implementation and clearer
status boundaries.

Repository:
https://github.com/dubzn/logos-private-balance-attestation

What is implemented:

- RISC Zero balance-attestation circuit for the LEZ private account commitment
  format.
- Real local wallet/sequencer witness path using `getProofForCommitment`.
- Off-chain verifier for the public proof envelope.
- Context binding, presenter binding, and context nullifier derivation.
- Local/pluggable Messaging adapter with token-gated admission and duplicate
  nullifier rejection.
- Backend-backed Basecamp MVP.
- Standalone consumer integration demos: governance gate, chat gate, and
  fee-tier gate.
- Public LEZ testnet deploy/admit evidence for both current on-chain candidate
  paths, documented in `docs/TESTNET_DEPLOYMENT.md`.
- Root `demo.sh`, root SPEL-style IDL JSON, Basecamp `module.json`, CI, local
  benchmark docs, and clean-room runner.

Validation commands:

```sh
./demo.sh --clean-room --real-prover
```

With local LEZ sequencer/wallet/private account:

```sh
PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
  ./demo.sh --clean-room --real-prover --with-lez --with-live --with-ppe
```

Current on-chain status:

- The public LEZ gate path is Workable/host-preverified: the host verifies the
  proof envelope before submitting the LEZ gate transaction, and the deployed
  LEZ program records/deduplicates the context nullifier.
- We confirmed locally that public LEZ execution does not currently expose a
  receipt/assumption channel for `env::verify(...)` over an externally supplied
  RISC Zero receipt.
- Spike 09 implements a stronger Logos-native PPE path: private LEZ execution
  checks `balance >= threshold` and writes public `BAP1` gate/nullifier state.
  This passed locally and on public testnet with `RISC0_DEV_MODE=0`, including
  duplicate and insufficient-balance rejection.

Open evaluator question:

For LP-0005, should the final on-chain path be the PPE-native private execution
gate, or is there a supported public LEZ pattern for verifying an externally
submitted RISC Zero receipt? The repo documents both paths and their trust
boundaries in `docs/ONCHAIN_PATH_DECISION.md` and
`docs/SUBMISSION_STATUS.md`.

Still pending before this should be treated as a complete award-ready
submission:

- evaluator-approved on-chain verifier model
- CU/performance metrics for the accepted on-chain path
- real Logos Messaging network adapter if local/pluggable transport is not
  sufficient
- narrated demo video with `RISC0_DEV_MODE=0`
```
```

## Short Discord Follow-Up

```text
Quick LP-0005 implementation question.

We now have two on-chain candidates implemented and exercised on public testnet:

1. Workable public LEZ gate:
   host verifies the RISC Zero proof envelope, then submits a gate admit tx.
   This persists/dedupes the nullifier on-chain, but the public LEZ program
   does not verify the receipt itself.

2. PPE-native LEZ gate:
   LEZ private execution checks private balance >= threshold and writes the
   public gate/nullifier state. This passed with RISC0_DEV_MODE=0, including
   duplicate and insufficient-balance rejection.

The open question is: for LP-0005, should the final on-chain path be the
PPE-native LEZ flow, or do evaluators require the same portable off-chain proof
envelope to be verified by a public LEZ program?

Repo:
https://github.com/dubzn/logos-private-balance-attestation

Relevant docs:
docs/ONCHAIN_PATH_DECISION.md
docs/TESTNET_DEPLOYMENT.md
docs/SUBMISSION_STATUS.md
```
