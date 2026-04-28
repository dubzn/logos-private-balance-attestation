# LEZ Verifier IDL Draft

This is a human-readable interface draft for the `balance_attestation_verifier`
program. LP-0005 requires a SPEL IDL before submission; this file is the design
source until the SPEL artifact is added.

## Program

```text
name: balance_attestation_verifier
version: 1
```

Purpose:

```text
Verify a private balance attestation proof and gate a public action without
revealing the private token account.
```

## Accounts

### Gate State Account

Writable public account owned by `balance_attestation_verifier`.

Stores:

```text
magic: "BAT1"
version: u16
context_id: [u8; 32]
threshold: u128
verifier_id: string
claims: Vec<GateClaim>
```

### Presenter Account

Authorized public account representing the identity allowed to use the proof.
This account is not the private token account.

The program checks:

```text
presenter_account.is_authorized == true
presenter_account.account_id == proof.journal.presenter_id
```

## Data Types

```rust
pub struct GateClaim {
    pub presenter_id: String,
    pub context_nullifier: [u8; 32],
    pub claimed_at: u64,
}

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
    pub presenter_signature: Option<Vec<u8>>,
}
```

The LEZ wire format is Borsh V1 unless the current LEZ/SPEL tooling requires a
different encoding. CLI JSON can remain a developer-facing interchange format,
but runners must convert JSON into the Borsh payload used on-chain.

## Instructions

### `init_gate`

Initializes a gate state account.

Inputs:

```text
context_id: [u8; 32]
threshold: u128
verifier_id: string
```

Accounts:

| Index | Account | Writable | Authorized | Notes |
| --- | --- | --- | --- | --- |
| 0 | gate_state | yes | no | Must be uninitialized. |
| 1 | admin | no | yes | Demo admin or deployer account. |

Errors:

- `BA500 GateAlreadyInitialized`
- `BA502 UnauthorizedPresenterAccount`

### `claim_access`

Verifies a proof envelope and records access for the presenter/context.

Inputs:

```text
proof_envelope: BalanceAttestationEnvelope
```

Accounts:

| Index | Account | Writable | Authorized | Notes |
| --- | --- | --- | --- | --- |
| 0 | gate_state | yes | no | Stores context and prior claims. |
| 1 | presenter | no | yes | Must match proof journal presenter id. |

Checks:

1. Gate state is initialized.
2. Proof envelope version is supported.
3. Proof image id is expected.
4. Proof receipt verifies.
5. Journal context id matches gate context.
6. Journal threshold equals the gate threshold.
7. Journal presenter id matches the authorized presenter account.
8. Context nullifier has not already been claimed for this gate.

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

### `read_gate`

Read helper used by CLI/Basecamp runners. It decodes gate state and prints
claims. It does not mutate chain state.

Presenter ids in claims are public by design for V1. The private account id
remains hidden; the public presenter id is the identity being admitted to the
gate. If a future integration needs hidden presenter membership, it should store
presenter-id hashes instead of raw presenter ids.

## SPEL Requirement

Before submission, this draft must become a real SPEL IDL artifact. The final
repo should keep both:

- SPEL file consumed by tooling
- generated or hand-written markdown reference for humans
