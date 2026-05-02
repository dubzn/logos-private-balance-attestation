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

## Spike Results

### Milestone 1 Start: attestation-core

Date: 2026-05-02.

Result:

```text
Rust workspace created.
attestation-core compiles and passes 11 unit tests.
```

Implemented reusable pieces:

- hex-encoded `Digest32` and `HexBytes`
- `ContextBindingParams`
- `BalanceAttestationJournal`
- `BalanceAttestationEnvelope`
- context id derivation
- presenter id derivation
- context nullifier derivation
- journal digest helper
- deterministic LP-0005 error-code enum

Canonical context id order for M1:

```text
SHA256(
  "logos-balance-attestation/v1/context"
  || chain_id
  || circuit_image_id
  || verifier_id
  || gate_id
  || threshold
)
```

This follows the updated architecture/security docs. Spike 04 proved the
binding shape, but production circuit code should import or mirror the M1 core
helpers so hash ordering cannot drift again.

### Spike 00A: Direct Receipt Gate

Date: 2026-04-29.

Result:

```text
receipt_gate.bin builds successfully.
Public transaction submission succeeds.
Sequencer rejects execution because no RISC Zero assumption/receipt is available.
```

Observed sequencer error:

```text
ProgramExecutionFailed(
  "sys_verify_integrity: no receipt found to resolve assumption: ..."
)
```

Interpretation:

- `risc0_zkvm::guest::env::verify` is available to the LEZ guest target.
- Current public LEZ execution does not expose an assumption channel for a
  submitted standalone receipt.
- Direct public receipt verification is not viable unless LEZ adds a way to
  pass assumptions/receipts into public execution.
- Next spike is the Logos-native private execution gate.

### Spike 01: Logos-Native Private Balance Gate

Date: 2026-04-29.

Result:

```text
private_balance_gate.bin builds successfully.
Public -> private funding works with sequencer and wallet both in RISC0_DEV_MODE=1.
A private LEZ transaction can read a private account balance, check a threshold,
and write a marker into a public gate account.
```

Observed positive fixture:

```text
private account balance: 42
threshold: 1
result: public gate account contains marker "private-balance-gate-ok"
```

Observed negative fixture:

```text
private account balance: 42
threshold: 43
result: proving fails locally with "private balance is below threshold"
```

Automated command:

```sh
RISC0_DEV_MODE=1 scripts/spike-01-demo-private-gate.sh
```

Interpretation:

- LEZ private execution is a viable development path for on-chain balance
  gating.
- The sequencer validates the private execution proof at the protocol layer,
  rather than the program verifying a nested receipt.
- This does not yet satisfy the LP-0005 wording by itself. The prize asks for
  a reusable proof that can also be verified off-chain, so we still need the
  standalone attestation circuit and verifier path.
- This spike is valuable because it proves the local wallet, private state,
  private transaction builder, sequencer proof validation, and public gated
  side effect can all work before the full E2E exists.
- Account readback must poll rather than sleep a fixed short duration because
  the local sequencer currently produces blocks roughly every 15 seconds.

### Spike 02: Private Commitment Membership Proof

Date: 2026-04-30.

Result:

```text
The local wallet can reconstruct a private account commitment and fetch a real
membership proof through getProofForCommitment.
```

Observed fixture:

```text
account_id: Private/9S4E6fo9XzawrQAtmtnGdi3GPhtJ4yBprZyhnot1FnY7
private_state_found: true
membership_proof_found: true
proof_index: 8
proof_depth: 4
commitment_root_hex: 9b9eb2ccaea9a32aa5cdf1e79b987b31ee608e59d2ce56c7668946ee180001a8
```

Automated command:

```sh
PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
  scripts/spike-02-inspect-membership-proof.sh
```

Interpretation:

- The sequencer adapter risk is lower: the required RPC exists and works
  against live local sequencer state.
- `WalletCore::get_private_account_commitment` and
  `WalletCore::check_private_account_initialized` are the local APIs to wrap
  before writing custom RPC plumbing.
- The next risky step is putting the same commitment root calculation inside a
  standalone RISC Zero guest and verifying it off-chain.

### Spike 03: Balance Attestation Circuit

Date: 2026-04-30.

Result:

```text
balance_attestation_spike.bin builds successfully.
Fixture proving succeeds above threshold and fails below threshold.
Fixture proving fails if the expected commitment root is tampered.
Live proving succeeds against a real private account and membership proof from
the local sequencer.
Live proving fails below threshold with the expected guest error.
```

Observed live success:

```text
account_id: Private/9S4E6fo9XzawrQAtmtnGdi3GPhtJ4yBprZyhnot1FnY7
threshold: 25
proved: true
verified: true
commitment_root_hex: 9b9eb2ccaea9a32aa5cdf1e79b987b31ee608e59d2ce56c7668946ee180001a8
proof_index: 8
proof_depth: 4
```

Observed live negative:

```text
threshold: 999999
expected_failure_observed: true
failure_contains: "Guest panicked: private balance is below threshold"
```

Automated commands:

```sh
scripts/spike-03-build-balance-circuit.sh
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh

PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
THRESHOLD=25 \
  RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live

PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
THRESHOLD=999999 \
  RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live-below-threshold
```

Interpretation:

- The standalone circuit risk is lower: the guest can consume the same LEZ
  account fields and Merkle proof shape used by the wallet/sequencer path.
- The journal is intentionally public-only. It reports threshold, root, context
  id, commitment, proof index, and proof depth, but not balance, `npk`, nonce,
  account data, private keys, or proof siblings.
- This is still a spike, not the final proof envelope. Presenter binding,
  context-derived nullifiers, stable serialization, off-chain verifier crate,
  Messaging, and final `RISC0_DEV_MODE=0` proving are still pending.

### Spike 04: Binding Attestation Circuit

Date: 2026-04-30.

Result:

```text
binding_attestation_spike.bin builds successfully.
Fixture proving succeeds above threshold.
Fixture proving fails for low balance, bad Merkle root, bad presenter id, and
bad context nullifier.
Changing gate/context data changes both context_id and context_nullifier.
Live proving succeeds against a real private account and membership proof from
the local sequencer.
Live proving fails below threshold with the expected guest error.
```

Observed fixture context variant:

```text
context_ids_differ: true
nullifiers_differ: true
first_context_id_hex: 6a27267dd2bf9da5f9c07d3be0f4b06527f26ffacedcc3c383fda10fae659dd5
second_context_id_hex: cc8f3348212596d23e8943dd02a03882fbcfda7fcfc1ee838d0f4cdef67b5bbd
first_nullifier_hex: df2b243768e6a01f3087f3c0efab1e3f3b8e60115ac800cef8316693b6efacfb
second_nullifier_hex: bf919e02413bd6f4d7706873b86deded6be9405f44e4d89057f615e74eada2be
```

Observed live success:

```text
account_id: Private/9S4E6fo9XzawrQAtmtnGdi3GPhtJ4yBprZyhnot1FnY7
threshold: 25
proved: true
verified: true
commitment_root_hex: 9b9eb2ccaea9a32aa5cdf1e79b987b31ee608e59d2ce56c7668946ee180001a8
context_id_hex: 90ebee7a4b647880d0de91454879df52489fca255c614f883656ce61e328acd5
context_nullifier_hex: d6c74e46b0fffd73311e6ace139d433ec53a5200e122c0b665e2bff78421ea5f
presenter_id_hex: ecc0fef4cd3e706458caa9eb944f487c99fa74d6c2c6a02bdae786450b850a48
proof_index: 8
proof_depth: 4
```

Automated commands:

```sh
scripts/spike-04-build-binding-circuit.sh
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh

PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
THRESHOLD=25 \
  RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live

PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private> \
THRESHOLD=999999 \
  RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live-below-threshold
```

Interpretation:

- Context binding is no longer just a passthrough journal field. The circuit
  derives `context_id` from domain-separated gate data and the circuit image id.
- Presenter binding is represented inside the circuit by proving knowledge of a
  synthetic presenter secret that derives `presenter_id`.
- The context nullifier is domain-separated and depends on `npk`, `context_id`,
  and `presenter_id`.
- The public journal no longer publishes the commitment leaf, reducing
  linkability versus Spike 03.
- The remaining production decision is mapping the synthetic presenter secret
  to a real wallet-compatible presenter identity or envelope signature scheme.

### Spike 05: RISC0_DEV_MODE Baseline Harness

Date: 2026-05-01.

Result:

```text
Baseline scripts exist for dev-mode and prod-mode proving.
Each script writes a Markdown table with per-step command, status, output, and
duration, plus a total row.
```

Implemented commands:

```sh
scripts/spike-05-run-devmode-baseline.sh
scripts/spike-05-run-prod-baseline.sh
```

Default output files:

```text
.spike-results/spike-05-devmode.md
.spike-results/spike-05-prod.md
```

Observed dev-mode smoke:

```text
command: LEZ_REPO=/Users/dub/logos/src/logos-execution-zone \
  SPIKE05_SKIP_BUILD=1 scripts/spike-05-run-devmode-baseline.sh
result: ok
prove-fixture-valid duration: 00:00:12
total duration: 00:00:12
```

Observed initial full dev-mode baseline blocker:

```text
build-binding-circuit failed because Docker was not running:
Cannot connect to the Docker daemon at unix:///Users/dub/.docker/run/docker.sock.
```

Observed full dev-mode baseline after Docker was started:

```text
output: .spike-results/spike-05-devmode.md
RISC0_DEV_MODE: 1
build-binding-circuit duration: 00:01:40
prove-fixture-valid duration: 00:00:04
total duration: 00:01:44
dev-mode warning present: yes
```

Observed full prod-mode baseline:

```text
output: .spike-results/spike-05-prod.md
RISC0_DEV_MODE: 0
build-binding-circuit duration: 00:00:03
prove-fixture-valid duration: 00:00:25
total duration: 00:00:28
dev-mode warning present: no
```

Interpretation:

- The benchmark harness works and records failures in the requested table
  format.
- `SPIKE05_SKIP_BUILD=1` is useful when the ELF already exists and we only want
  to compare proving/verification time.
- The first full dev-mode run paid most of the Docker/build cost. The prod build
  reused cache, so compare proof rows more than total rows for this run.
- The current fixture proof is feasible in real proving mode on this machine:
  dev proof was roughly 4 seconds, prod proof roughly 25 seconds.

### Spike 06: On-Chain Path Decision

Date: 2026-05-02.

Result:

```text
The local public LEZ execution path does not expose an assumption injection
point or native verifier API for an external standalone RISC Zero receipt.
The Logos-native private execution path does use assumptions internally and is
the only working local on-chain gate route found so far.
```

Automated command:

```sh
scripts/spike-06-inspect-onchain-path.sh
```

Observed inspection:

```text
public LEZ execution uses default executor: yes
public LEZ execution adds receipt assumptions: 0
private LEZ proof host adds assumptions: 1
private LEZ guest calls env::verify: 1
private LEZ proof uses succinct receipts: yes
Spike 0A missing assumption error recorded: observed
```

Decision:

```text
Do not build M1 around public LEZ verification of external RISC Zero receipts.
Build the reusable off-chain proof envelope/verifier first.
Keep the on-chain path behind an interface and use Logos-native private
execution as the working local fallback pending evaluator confirmation.
```

Interpretation:

- 0A is failed/currently unsupported.
- 0B was inspected; RISC Zero recursion exists, but no local deployable public
  LEZ path was found for external receipt verification.
- 0C remains the only local on-chain path that works today, but prize wording
  still requires evaluator confirmation.
