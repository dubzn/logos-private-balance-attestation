# Implementation Plan

This project targets the full LP-0005 prize, but implementation should progress
through small slices that each prove one hard assumption.

## Milestone 0: Documentation Baseline

Goal: preserve the architecture and implementation context before writing code.

Deliverables:

- README with project goal and status.
- Architecture doc with LEZ commitment details.
- Local setup doc with sequencer and wallet flow.
- Security model with replay and proof-forwarding strategy.
- Prize checklist mapping requirements to artifacts.
- Reference notes from local LEZ, `logos-token-suite`, and
  `logos-document-guardian`.

Acceptance:

- A new engineer can understand the planned system without reading chat history.
- The docs explicitly mention real `getProofForCommitment` usage.
- The docs explicitly call out the LEZ commitment prefix mismatch with the
  simplified prize formula.

## Milestone 0.5: Feasibility Spikes

Goal: validate the assumptions that can invalidate the whole submission before
building the circuit, CLI, Messaging path, or Basecamp app.

Blocker 0 is the on-chain proof path. LP-0005 requires a LEZ verifier program
that accepts/verifies a proof and gates an action. We must prove the feasible
runtime path first.

Spike order:

1. Build a toy RISC Zero method and produce a receipt with the same RISC Zero
   version as LEZ.
2. Try direct receipt verification inside a minimal LEZ guest program.
3. If direct verification fails, inspect recursive/native verifier options
   exposed by the current LEZ/RISC Zero runtime.
4. In parallel, prototype a Logos-native private execution gate where LEZ
   private execution itself proves `balance >= threshold`, then ask the prize
   evaluators whether this satisfies the on-chain path.
5. Document the chosen path and update architecture before Milestone 1.

Acceptance:

- A toy proof is generated locally.
- A minimal LEZ program can gate one action using the selected proof strategy,
  or the repo has a documented evaluator answer accepting the native private
  execution strategy.
- The spike runs against a local sequencer, not only a unit test.
- If no path works, the project pauses before implementing lower-risk modules.
- An external builder candidate for the third integration is identified early.

Current automation:

```sh
scripts/spike-00-inspect-lez-proof-path.sh
scripts/spike-00-build-lez-program.sh
SPIKE_ACCOUNT=<id> scripts/spike-00-run-direct-receipt-gate.sh
RISC0_DEV_MODE=1 scripts/spike-01-demo-private-gate.sh
scripts/spike-06-inspect-onchain-path.sh
```

## Milestone 1: Rust Workspace And Shared Types

Goal: create the reusable core interface before proving.

Current crate:

- `attestation-core`
- `attestation-prover`

Later M1/M2 crates:

- `attestation-verifier`
- `attestation-cli`

Core types:

```rust
pub struct BalanceAttestationJournal {
    pub version: u16,
    pub threshold: u128,
    pub context_id: [u8; 32],
    pub commitment_root: [u8; 32],
    pub context_nullifier: [u8; 32],
    pub presenter_id: String,
    pub verifier_id: String,
}

pub struct BalanceAttestationEnvelope {
    pub version: u16,
    pub proof_system: String,
    pub image_id: [u8; 32],
    pub journal: BalanceAttestationJournal,
    pub receipt: Vec<u8>,
    pub presenter_pubkey: Vec<u8>,
    pub presentation_challenge: [u8; 32],
    pub presenter_signature: Vec<u8>,
}
```

Acceptance:

- `cargo test -p attestation-core` passes.
- Journal serialization is stable and hex-encoded.
- Context id derivation is deterministic and changes when any context field
  changes.
- Nullifier derivation changes across contexts, presenters, and private account
  nullifier keys.
- Error codes are enumerated, documented, and round-trip by code string.

Current automation:

```sh
cargo test -p attestation-core
```

## Milestone 2: LEZ Commitment And Sequencer Adapter

Goal: consume real local LEZ state.

Implementation:

- Reconstruct LEZ commitments exactly as `nssa_core::Commitment::new` does.
- Read private account state from local wallet storage through the safest
  available interface.
- Fetch membership proofs through JSON-RPC `getProofForCommitment`.
- Do not use REST endpoints such as `/v1/proof_for_commitment`; those are not
  part of the local sequencer API.

Acceptance:

- A fixture test compares local commitment reconstruction against LEZ
  `Commitment::new`.
- A fixture test compares local Merkle root reconstruction against LEZ
  `compute_digest_for_path`.
- The CLI can inspect a private account commitment without exposing witness data
  in normal output.
- The CLI can fetch a membership proof for an initialized private account.

Current automation:

```sh
cargo test -p attestation-core
scripts/m2-check-lez-commitment-compat.sh
PRIVATE_ACCOUNT=<id> scripts/m2-inspect-private-account.sh --local-only
PRIVATE_ACCOUNT=<id> scripts/m2-inspect-private-account.sh --require-proof
cargo run -p attestation-cli -- inspect-private --account Private/<id> --local-only
cargo run -p attestation-cli -- inspect-private --account Private/<id> --require-proof
```

Current status:

```text
Pure commitment helpers exist in attestation-core.
Local compatibility script passes against nssa_core for dummy, documented, and
wide-value fixtures.
Wallet local inspect script validates private account commitment reconstruction
without printing witness data.
Sequencer proof mode passed locally against getProofForCommitment.
attestation-prover now owns reusable sanitized report/redaction logic and the
initial balance attestation witness builder.
attestation-cli exposes inspect-private as the product-facing command; it still
bridges to the M2 script so LEZ wallet dependencies stay isolated.
```

Planned CLI:

```sh
balance-attest inspect-private \
  --account Private/<id>

balance-attest fetch-membership-proof \
  --account Private/<id> \
  --json
```

## Milestone 3: RISC Zero Circuit

Goal: prove `balance >= threshold` with real commitment membership.

Implementation:

- Pin RISC Zero to the version used by local LEZ. The inspected checkout uses
  `risc0-zkvm = "3.0.5"` and `risc0-build = "3.0.5"`.
- Build a guest method that accepts private witness and public parameters.
- Commit only the public journal.
- Keep witness data out of logs and serialized proof envelopes.

Acceptance:

- Prover succeeds for `balance >= threshold`.
- Prover fails for `balance < threshold`.
- Tampering with context id, root, threshold, or nullifier causes verification
  failure.
- Tests run with dev mode for fast iteration.
- A slow/full proof path is documented for `RISC0_DEV_MODE=0`.

Planned CLI:

```sh
balance-attest prove \
  --account Private/<id> \
  --threshold 100 \
  --context "governance/demo-vote-1" \
  --presenter Public/<id> \
  --out proof.json

balance-attest verify-offchain \
  --proof proof.json \
  --context "governance/demo-vote-1" \
  --threshold 100
```

## Milestone 4: Presenter Binding

Goal: make captured proofs unusable by a passive third party.

Implementation:

- Include `presenter_id` in the public journal and nullifier derivation.
- Off-chain path verifies a fresh challenge signature from the presenter.
- On-chain path requires an authorized presenter account and checks it matches
  the journal.

Acceptance:

- Proof verifies for the original presenter.
- Proof fails if submitted with a different presenter id.
- Proof fails if the challenge signature is missing or invalid.
- Security docs explain remaining collusion limits.

## Milestone 5: LEZ Verifier Program

Goal: satisfy the on-chain path with a deployable LEZ program or
evaluator-approved Logos-native private execution gate.

Current blocker:

```text
Spike 06 found no local public LEZ path for verifying external standalone RISC
Zero receipts. Do not implement Milestone 5 around direct public env::verify
unless Logos exposes an assumption/native verifier path or evaluators give new
guidance.
```

Implementation:

- Create `lez/verifier-program` with LEZ guest source and host runners.
- Use the same local program installation pattern as `logos-document-guardian`.
- Implement a small gated action:

```text
claim_access(context_id, proof_envelope)
```

- Store minimal public state:

```text
context_id -> presenter_id -> context_nullifier
```

Acceptance:

- Invalid proofs return deterministic error codes.
- Valid proof claims access once.
- Replaying the same nullifier in the same context is rejected or idempotently
  reported, depending on final gate semantics.
- The program is deployable to the local sequencer.
- Host-only pre-verification is not the final on-chain verification story.

## Milestone 6: Off-Chain Messaging Path

Goal: transmit proof envelopes over Logos Messaging and verify locally.

Implementation:

- Add a Messaging adapter behind a small trait. **Implemented as
  `attestation-messaging::ProofMessageTransport`.**
- Start with a local JSON adapter while full Logos Messaging app integration is
  not yet stable. **Implemented as `LocalFileTransport` plus CLI commands.**
- Add a Basecamp Logos Delivery adapter for the real module path.
  **Implemented with `delivery_module` create/start/subscribe/send/receive and
  local `message-verify`; validated across two Basecamp instances with a
  chunked 1.3 MB real-prover message. Final recording pending.**
- Preserve the proof envelope format so the transport can be replaced later.

Acceptance:

- Sender publishes proof envelope to a gate topic. **Local equivalent:
  `message-export --group ... --out message.json`.**
- Recipient fetches/receives proof envelope. **Local equivalent:
  `message-receive --message message.json`.**
- Recipient verifies proof and presenter challenge locally. **Implemented:
  `message-verify`.**
- Basecamp sends and receives the same proof-message bytes over Logos Delivery.
  **Validated on 2026-07-13: 17 chunks reassembled to the sender SHA-256 and
  passed receiver-side `message-verify`.**
- Messaging failure surfaces a clear error without dumping witness data.
  **Implemented via `MessagingError` and BA400/BA401 mapping.**

Demo:

```sh
RISC0_DEV_MODE=1 scripts/demo-local-messaging.sh
```

The demo exports a proof message, receives/imports it, verifies it, admits it
to a local group state, and confirms duplicate admission is rejected.

## Milestone 7: Basecamp App

Goal: provide a working visual demo.

Implementation:

- Backend-backed `ui_qml`, following the pattern used by
  `logos-document-guardian`.
- QML UI for:
  - account/context/threshold config
  - generate proof
  - verify off-chain
  - send proof
  - run on-chain gated action
- C++ backend executes CLI commands and exposes structured status.

Acceptance:

- Basecamp can generate and verify a proof using local CLI/backend.
- UI shows clear success and failure states.
- No private witness fields are rendered in the UI.

## Milestone 8: Reference Integrations

Goal: satisfy the prize requirement for multiple integrations.

Planned integrations:

- Governance gate: on-chain `cast_vote` or `claim_voting_access`.
- Messaging group gate: off-chain access to a Logos Messaging group.
- Third integration: fee tier, allowlist, or document access gate.

Acceptance:

- A standalone consumer integration demo consumes the SDK/CLI/proof format.
- Local governance, chat, and fee-tier examples remain useful reference
  integrations, but updated LP-0005 no longer requires an outside-team
  integration.

## Milestone 9: Submission Hardening

Goal: make the repo evaluator-ready.

Deliverables:

- Green CI on default branch.
- End-to-end script against a clean local sequencer.
- Final script works with `RISC0_DEV_MODE=0`.
- CU/gas cost docs for on-chain operations.
- Benchmarks for proof generation time.
- SPEL IDL for the LEZ program.
- Testnet deployment details and verified program id.
- Narrated video demo covering architecture, on-chain path, and off-chain path.
