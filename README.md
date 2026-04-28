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

This repo is currently in the planning and architecture phase. The first slice
is intentionally documentation-first so the implementation can continue across
tools and sessions without losing context.

No prize submission should be made from this state. The project still needs the
RISC Zero circuit, CLI, LEZ verifier program, Messaging integration, Basecamp
GUI, tests, benchmarks, and demo video.

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

Only after Blocker 0 is green should the project build the core off-chain proof
loop:

```text
wallet private account
  -> real LEZ commitment reconstruction
  -> getProofForCommitment
  -> RISC Zero proof for balance >= N
  -> local verifier accepts/rejects
```

Only after that core loop is true should the project add the LEZ verifier
program, Messaging path, Basecamp GUI, and external integrations.

