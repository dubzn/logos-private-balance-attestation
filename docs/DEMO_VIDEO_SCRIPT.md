# Demo Video Script

This is a recording plan, not a transcript. The goal is to show the project
clearly without overstating the on-chain status.

## Recording Setup

- Use a clean terminal with large font.
- Export the local environment from `scripts/env.example`.
- Start the local LEZ sequencer from the same `logos-execution-zone` checkout
  used by the wallet.
- Run `scripts/check-wallet-preflight.sh` before recording.
- Keep `witness.json`, wallet home, and private account details out of the
  final published artifacts.
- Use `RISC0_DEV_MODE=0` for final evidence.

## Suggested Flow

### 1. Project Overview

Say:

```text
This is an LP-0005 implementation for private token balance attestation. The
goal is to prove that a private LEZ account has balance >= threshold without
revealing the exact balance, npk, nonce, account data, or private account id.
```

Show:

```sh
ls
sed -n '1,120p' docs/SUBMISSION_STATUS.md
```

Call out:

- `demo.sh`
- `balance-attestation-verifier.idl.json`
- `apps/basecamp/module.json`
- `docs/PRIZE_CHECKLIST.md`
- `docs/ONCHAIN_PATH_DECISION.md`

### 2. Clean-Room Proof And Messaging Demo

Run:

```sh
./demo.sh --clean-room --real-prover
```

Explain:

```text
This path does not require wallet setup. It proves and verifies a deterministic
fixture, then wraps the public envelope as a local Messaging-style proof
message, receives it, verifies it, admits it to a token-gated group, and rejects
a duplicate nullifier.
```

Show the generated report path.

### 3. Live Local Wallet + Sequencer Proof

Run:

```sh
scripts/check-wallet-preflight.sh

PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  scripts/prepare-local-private-account.sh

PRIVATE_ACCOUNT=Private/<private-account-id> \
THRESHOLD=1 \
RISC0_DEV_MODE=0 \
  ./demo.sh --full --real-prover
```

Explain:

```text
This is the live local path. It reads private wallet state, requests the real
membership proof through getProofForCommitment, builds a private witness,
generates a public RISC Zero proof envelope, verifies it off-chain, and submits
the Workable LEZ gate transaction after host-side verification.
```

Call out that `witness.json` is private.

### 4. PPE-Native Gate Candidate

Run:

```sh
RISC0_DEV_MODE=0 ./demo.sh --ppe-gate --real-prover
```

Explain:

```text
This is the stronger local on-chain candidate. Instead of submitting the
portable off-chain proof envelope, the LEZ privacy-preserving execution path
checks the private balance condition and writes public gate/nullifier state. It
also demonstrates duplicate rejection and insufficient-balance rejection.
```

Make the limitation explicit:

```text
This is pending evaluator confirmation because LP-0005 asks for on-chain proof
verification, and the current public LEZ env::verify path for external receipts
does not expose an assumptions channel locally.
```

### 5. Basecamp MVP

Show Basecamp with the `balance_attestation` module loaded.

Before recording this section, run:

```sh
scripts/check-basecamp-package.sh
```

To launch the app:

```sh
scripts/run-basecamp-local.sh --reset --real-prover
```

Click through:

- preflight
- generate proof
- verify envelope
- gate admit

Explain:

```text
The UI is intentionally a local operator surface over the same CLI/scripts used
in the terminal demo. It does not display or parse witness.json.
```

### 6. Close With Honest Status

Say:

```text
The off-chain proof envelope, local Messaging-style transport, Basecamp MVP,
and local sequencer flows are implemented. The remaining blocker is evaluator
confirmation of the accepted on-chain verifier model: PPE-native private
execution, a public LEZ receipt-verification API, or another Logos-approved
pattern.
```

Show:

```sh
sed -n '1,180p' docs/PRIZE_CHECKLIST.md
```

## What Not To Say

- Claim public testnet deployment only through the concrete evidence in
  `docs/TESTNET_DEPLOYMENT.md`; do not imply the accepted on-chain model is
  resolved.
- Do not claim CU measurements until they are measured on devnet/testnet.
- Do not claim the deployable public LEZ program verifies external RISC Zero
  receipts in-guest.
- Do not publish `witness.json`, wallet storage, recovery phrases, private
  account ids, `npk`, or `vpk`.
