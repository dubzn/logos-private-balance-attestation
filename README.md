# Logos Private Balance Attestation

Documentation-first implementation plan for LP-0005: Private Token Balance
Attestation.

This repository targets a reusable Logos primitive that lets a holder of a
private LEZ account prove:

```text
private_balance >= public_threshold
```

without revealing the private account id, nullifier public key, exact balance,
nonce, or account data.

## Status

This repo is past the initial planning phase and now contains risk spikes for
the highest-uncertainty LP-0005 pieces: on-chain proof path exploration,
Logos-native private balance gating, real membership proof retrieval, a
standalone balance attestation circuit, and a binding/nullifier circuit.

Milestone 1 has started with a real Rust workspace and `attestation-core`, the
runtime-independent crate that owns the proof envelope shape, public journal,
context hashing, presenter/nullifier helpers, and deterministic error codes.
Milestone 2 now has a local-first `balance-attest inspect-private` command for
sanitized wallet/sequencer inspection.

No prize submission should be made from this state. The project still needs the
production prover, verifier, CLI, LEZ verifier decision, Messaging integration,
Basecamp GUI, broader tests, benchmarks, and demo video.

## Target Verification Paths

LP-0005 requires two verification paths over the same attestation primitive:

- On-chain: a LEZ verifier program accepts the proof and gates an on-chain
  action, such as `claim_access` or `cast_vote`.
- Off-chain: a recipient verifies the proof locally after receiving it through
  Logos Messaging, enabling token-gated access without an on-chain
  transaction.

## Non-Negotiables

The implementation must:

- target the real LEZ private account commitment format, including the domain
  prefix used in the local `logos-execution-zone` checkout
- call the real JSON-RPC method `getProofForCommitment`
- use the RISC Zero version compatible with the local LEZ toolchain
- run against a real local sequencer
- produce a final demo with `RISC0_DEV_MODE=0`
- provide a deployable LEZ program, not a standalone mock verifier
- document deterministic error codes and compute costs
- include a SPEL/IDL story before submission
- validate that RISC Zero receipt verification inside a LEZ guest is feasible
  BEFORE writing the circuit; this is the failure mode that killed PR #17

## Planned Repository Shape

```text
logos-private-balance-attestation/
|-- README.md
|-- docs/
|   |-- ARCHITECTURE.md
|   |-- ERROR_CODES.md
|   |-- IDL_DRAFT.md
|   |-- IMPLEMENTATION_PLAN.md
|   |-- LOCAL_SETUP.md
|   |-- MODULAR_TEST_PLAN.md
|   |-- PRIZE_CHECKLIST.md
|   |-- RISK_SPIKES.md
|   |-- SECURITY_MODEL.md
|   `-- REFERENCE_NOTES.md
|-- crates/
|   |-- attestation-core/
|   |-- attestation-prover/
|   |-- attestation-verifier/
|   `-- attestation-cli/
|-- methods/
|   `-- guest/
|-- lez/
|   `-- verifier-program/
|-- apps/
|   `-- basecamp/
|-- examples/
|   |-- governance-gate/
|   |-- messaging-group-gate/
|   `-- third-integration/
`-- scripts/
```

The shape intentionally mirrors the lessons from `logos-document-guardian`:
keep the SDK and CLI reusable, make LEZ runners explicit, keep Basecamp as a
thin UI over local backend commands, and provide scripts that make the local
flow reproducible.

## Documentation Map

- [Architecture](docs/ARCHITECTURE.md): system flow, proof format, LEZ
  commitment compatibility, on-chain/off-chain paths.
- [IDL Draft](docs/IDL_DRAFT.md): human-readable LEZ program interface before
  the final SPEL artifact exists.
- [Error Codes](docs/ERROR_CODES.md): deterministic error code plan shared by
  CLI, off-chain verifier, and LEZ program.
- [Implementation Plan](docs/IMPLEMENTATION_PLAN.md): milestone order and
  acceptance criteria.
- [Local Setup](docs/LOCAL_SETUP.md): sequencer, wallet, private account, and
  future demo commands.
- [Modular Test Plan](docs/MODULAR_TEST_PLAN.md): isolated commands for each
  layer before the final E2E.
- [Next Steps](docs/NEXT_STEPS.md): ordered backlog from the current state to
  the final submission.
- [On-Chain Path Decision](docs/ONCHAIN_PATH_DECISION.md): Spike 06 result for
  the LEZ verifier path.
- [Prize Checklist](docs/PRIZE_CHECKLIST.md): LP-0005 requirements mapped to
  planned artifacts.
- [Risk Spikes](docs/RISK_SPIKES.md): modular validation plan for the highest
  risk assumptions before building the full stack.
- [Security Model](docs/SECURITY_MODEL.md): privacy guarantees, replay,
  forwarding, nullifiers, and known limitations.
- [Reference Notes](docs/REFERENCE_NOTES.md): useful facts from LP-0005,
  local LEZ, `logos-token-suite`, and `logos-document-guardian`.

## First Risk Goal

Before building the full circuit and CLI, validate the on-chain proof
verification path. This is Blocker 0.

The first implementation spike should prove at least one viable path for the
LP-0005 on-chain requirement:

- direct RISC Zero receipt verification inside a LEZ guest
- recursive/native verifier support exposed by LEZ
- Logos-native private execution gate accepted by evaluators as satisfying the
  on-chain proof path

Current Spike 06 decision: direct public `env::verify` is failed/currently
unsupported, recursive/native public verifier support was not found in the
local LEZ checkout, and Logos-native private execution is the only working
local on-chain gate path pending evaluator confirmation.

The core off-chain proof loop is now validated enough to start Milestone 1:

```text
wallet private account
  -> real LEZ commitment reconstruction
  -> getProofForCommitment
  -> RISC Zero proof for balance >= N
  -> local verifier accepts/rejects
```

Only after that core loop is true should the project add the LEZ verifier
program, Messaging path, Basecamp GUI, and external integrations.

Start here:

```sh
scripts/spike-00-inspect-lez-proof-path.sh
scripts/spike-00-build-lez-program.sh
```

Then test the Logos-native private execution gate with a local sequencer
running in dev mode:

```sh
RISC0_DEV_MODE=1 scripts/spike-01-demo-private-gate.sh
```

Next, verify the prover-facing sequencer path for an initialized private
account:

```sh
PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
  scripts/spike-02-inspect-membership-proof.sh
```

Then run the standalone balance circuit spike:

```sh
scripts/spike-03-build-balance-circuit.sh
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh

PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
THRESHOLD=25 \
  RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live
```

Then run the binding/nullifier circuit spike:

```sh
scripts/spike-04-build-binding-circuit.sh
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh

PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
THRESHOLD=25 \
  RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live
```

Before M1, close the remaining spike work:

- Spike 05: passed locally with separate `RISC0_DEV_MODE=1` and
  `RISC0_DEV_MODE=0` Markdown result files.
- Spike 06: passed as a documented decision; do not build M1 around public
  external receipt verification.

Run the first reusable core tests:

```sh
cargo test -p attestation-core
```

Check the local LEZ commitment compatibility against `nssa_core`:

```sh
scripts/m2-check-lez-commitment-compat.sh
```

Inspect a local private account without printing witness data:

```sh
cargo run -p attestation-cli -- inspect-private \
  --account Private/<private-account-id> \
  --local-only
```

With a local sequencer running, request the real membership proof:

```sh
cargo run -p attestation-cli -- inspect-private \
  --account Private/<private-account-id> \
  --require-proof
```
