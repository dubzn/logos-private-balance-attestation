# LEZ Verifier IDL Reference

This is the **human-readable** companion to `idl/balance-attestation-verifier.json`,
the SPEL-compatible IDL artifact. The two MUST be kept in sync; the JSON is
authoritative for our tooling, this file is authoritative for prose.

> **SPEL on the LEZ side is documentation-only.** The local LEZ build does
> not ingest a SPEL/IDL artifact at deployment time — instruction dispatch
> is static Rust code in the deployed program guest, with calldata
> serialized via `risc0_zkvm::serde` (the `Vec<u32>` instruction words
> consumed by `read_lee_inputs`). The IDL JSON in this repo is the public
> contract our developers and external integrators read; the deployed
> program's wire format is whatever `serde::Deserialize` accepts on its
> `Instruction` enum and whatever `borsh` accepts on its `OuterJournal` /
> `GateState` types.

The architecture resolved to the **Workable** Spike 0C path (per
`docs/ONCHAIN_PATH_DECISION.md`): the LEZ program does NOT verify the off-chain
envelope or outer RISC Zero receipt directly. The host (CLI) calls
`attestation_verifier::verify_envelope` before submission so the deployed
program can treat the journal it receives in calldata as host-verified. The
deployed program
(`lez-verifier/program/guest/src/bin/balance_attestation_program.rs`) then
checks (a) `outer_journal.version == 1`, (b)
`outer_journal.inner_image_id == gate_state.expected_inner_image_id`,
(c) gate context match, (d) exact context-bound threshold, (e) presenter id
== `H(PRESENTER_DOMAIN || presenter.account.data[..32])` over an authorized
presenter pre-state, and (f) nullifier dedup against
`gate_state.admitted_nullifiers`.

This reference covers the deployable public gate-state program. It does not
describe the experimental Spike 09 PPE-native gate, which is intentionally kept
under `spikes/spike-09-ppe-gate/` until evaluators confirm whether that path is
the expected LP-0005 on-chain verification model.

## Program

```text
name: balance_attestation_verifier
version: 1
```

Purpose:

```text
Record a host-verified private balance attestation admission and gate a public
action without revealing the private token account.
```

## Accounts

### Gate State Account

Writable public account owned by `balance_attestation_verifier`.

Stores:

```text
magic: "BAT1"
version: u16
chain_id: [u8; 32]
verifier_id: [u8; 32]
gate_id: [u8; 32]
threshold: u128
expected_inner_image_id: [u8; 32]
admitted_nullifiers: Vec<[u8; 32]>
```

### Presenter Account

Authorized public account representing the identity allowed to use the proof.
This account is not the private token account.

The program checks:

```text
presenter_account.is_authorized == true
presenter_account.program_owner == balance_attestation_verifier
presenter_account.data[..32] == registered BIP-340 x-only pubkey
derive_presenter_id(presenter_account) == proof.journal.presenter_id
```

## Data Types

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

pub struct LezGateJournal {
    pub version: u16,
    pub inner_image_id: [u8; 32],
    pub gate_context_id: [u8; 32],
    pub accepted_context_nullifier: [u8; 32],
    pub accepted_presenter_id: [u8; 32],
    pub accepted_threshold: u128,
}
```

The LEZ wire format is Borsh V1 unless the current LEZ/SPEL tooling requires a
different encoding. CLI JSON can remain a developer-facing interchange format,
but runners must convert JSON into the Borsh payload used on-chain.

## Instructions

### `register_presenter`

Writes a presenter's 32-byte BIP-340 x-only pubkey into a fresh presenter
account and claims that account for the verifier program. This gives the live
LEZ gate a concrete account to compare against `outer_journal.accepted_presenter_id`.

Inputs:

```text
presenter_pubkey: [u8; 32]
```

Accounts:

| Index | Account | Writable | Authorized | Notes |
| --- | --- | --- | --- | --- |
| 0 | presenter | yes | yes | Must be uninitialized; claimed by this program. |
| 1 | admin | no | yes | Demo admin or deployer account. |

Errors:

- `BA502 UnauthorizedPresenterAccount`

### `init_gate`

Initializes a gate state account.

Inputs:

```text
chain_id: [u8; 32]
verifier_id: [u8; 32]
gate_id: [u8; 32]
threshold: u128
expected_inner_image_id: [u8; 32]
```

Accounts:

| Index | Account | Writable | Authorized | Notes |
| --- | --- | --- | --- | --- |
| 0 | gate_state | yes | no | Must be uninitialized. |
| 1 | admin | no | yes | Demo admin or deployer account. |

Errors:

- `BA500 GateAlreadyInitialized`
- `BA503 InvalidGateAccount`

### `admit`

Records access for the presenter/context from a host-verified outer journal.

Current status:

```text
Direct public verification of an external RISC Zero receipt is not currently
supported by the tested LEZ public execution path because no assumption/receipt
channel is available to env::verify.
```

Spike 06 closes the current local decision: this IDL should be read as the
deployed gate state interface, not as proof that public receipt verification is
currently deployable. Spike 08 confirmed that a fabricated but well-formed
journal can be applied if the host submits it. Therefore a production CLI must
verify the envelope/outer receipt before it submits this instruction.

Inputs:

```text
outer_journal: Vec<u8> // Borsh LezGateJournal
```

Accounts:

| Index | Account | Writable | Authorized | Notes |
| --- | --- | --- | --- | --- |
| 0 | gate_state | yes | no | Stores context and prior claims. |
| 1 | presenter | no | yes | Must match proof journal presenter id. |

Checks:

1. Gate state is initialized.
2. Outer journal version is supported.
3. Outer journal inner image id is expected.
4. Journal context id matches the derived gate context.
5. Journal threshold exactly equals the gate threshold.
6. Journal presenter id matches the authorized presenter account's pubkey hash.
7. Context nullifier has not already been admitted for this gate.

Errors:

- `BA100 InvalidEnvelopeVersion`
- `BA102 InvalidImageId`
- `BA200 InvalidReceipt`
- `BA201 ThresholdMismatch`
- `BA202 ContextMismatch`
- `BA203 PresenterMismatch`
- `BA206 DuplicateNullifier`
- `BA501 GateNotInitialized`
- `BA502 UnauthorizedPresenterAccount`
- `BA503 InvalidGateAccount`

Read helpers are off-chain CLI/Basecamp utilities for now: they fetch the gate
state account and decode `GateState`. The deployed program currently exposes no
separate read-only instruction.

## SPEL Status

The repository now exposes a root-level IDL symlink for evaluator tooling:

```text
balance-attestation-verifier.idl.json -> idl/balance-attestation-verifier.json
```

The JSON file is hand-written SPEL-style IDL because the current local LEZ
deployment path does not consume SPEL artifacts directly. If Logos provides a
required SPEL generator or schema during review, regenerate this JSON from that
tooling and keep this human-readable reference in sync.
