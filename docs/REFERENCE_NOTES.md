# Reference Notes

These notes preserve implementation facts discovered before coding.

## LP-0005 Source

Local prize file:

```text
/Users/dub/Desktop/caravana/lambda-prize/prizes/LP-0005.md
```

Important requirements:

- RISC Zero circuit proving `balance >= N`.
- Circuit targets LEZ private account commitments.
- Proof is context-bound.
- Proof is presenter-bound.
- On-chain LEZ verifier gates an action.
- Off-chain verifier receives proof over Logos Messaging.
- Basecamp GUI is required.
- SPEL IDL is required.
- Final demo must run against a real local sequencer with `RISC0_DEV_MODE=0`.

## Prior LP-0005 Submission Feedback

Public feedback on an earlier LP-0005 PR identified failure modes this project
must avoid:

- tests pinned `risc0-zkvm = "1.2"` while the current toolchain was 3.x
- verifier was a standalone Rust library, not deployable to LEZ
- sequencer client used non-existent `/v1/proof_for_commitment` and
  `/v1/current_root` endpoints
- tests used dummy data instead of a live sequencer
- no deployed verifier program id or full demo video

Reference:

```text
https://github.com/logos-co/lambda-prize/pull/17#issuecomment-4152841077
```

## Local LEZ Facts

Local checkout:

```text
/Users/dub/logos/src/logos-execution-zone
```

Useful files:

```text
nssa/core/src/commitment.rs
nssa/core/src/account.rs
sequencer/service/rpc/src/lib.rs
wallet/src/lib.rs
wallet/src/cli/account.rs
examples/program_deployment/README.md
```

Observed facts:

- LEZ currently pins `risc0-zkvm = "3.0.5"` and `risc0-build = "3.0.5"`.
- `Account` contains `program_owner`, `balance`, `data`, and `nonce`.
- Private account data is local wallet state.
- `wallet account get --account-id Private/<id>` reads local storage.
- `wallet account sync-private` refreshes local private state from encrypted
  transaction outputs.
- The real JSON-RPC method is `getProofForCommitment`.
- `MembershipProof` is `(usize, Vec<[u8; 32]>)`.

## Commitment Compatibility Detail

The prize writes the commitment as:

```text
SHA256(npk || program_owner || balance || nonce || SHA256(data))
```

The local code adds a domain prefix:

```text
"/LEE/v0.3/Commitment/" padded to 32 bytes
```

The implementation must match local LEZ code exactly.

## Lessons From logos-document-guardian

Local repo:

```text
/Users/dub/Desktop/logos/logos-document-guardian
```

Reusable patterns:

- keep a small Rust SDK crate separate from the CLI
- make CLI commands wrap local LEZ runners
- copy LEZ guest/runners into `examples/program_deployment`
- keep Basecamp backend-backed and thin
- write local demo scripts with explicit env vars
- document skip flags for already-deployed programs
- account for nonce timing after submitted transactions

The Document Guardian flow proved this shape works:

```text
local data -> SDK -> CLI -> LEZ runner -> sequencer -> read helper -> Basecamp
```

For this project, the analogous flow is:

```text
private account -> witness builder -> prover -> verifier -> LEZ/Messaging/Basecamp
```

## Lessons From logos-token-suite

Local repo:

```text
/Users/dub/Desktop/logos/logos-token-suite
```

It is a useful reference for:

- workspace organization
- core library plus on-chain program split
- CLI examples
- design docs with instruction references
- deterministic account derivation docs
- security property tables

It should not be copied as a submission base. LP-0013 and LP-0014 are separate
prize work by another builder.

## Messaging Notes

The documented Messaging flow is weaker than the LEZ wallet flow:

- local Delivery API compose commands exist
- chat journey docs are stubbed
- demo-specific mixnet paths exist, but are not a general Messaging SDK guide

For the first off-chain milestone, keep Messaging behind an adapter so local
transport can be swapped for the final Logos Messaging path.

Before relying on the local Delivery API as a final substitute, ask the prize
evaluators what they accept as "transmitted over Logos Messaging". The local
Delivery API is useful for development, but may not satisfy the final
submission requirement.

## SPEL/IDL Notes

LP-0005 requires an IDL using SPEL. The first implementation can document the
program interface in a human-readable IDL, but final submission needs the actual
SPEL artifact once the local toolchain path is clear.
