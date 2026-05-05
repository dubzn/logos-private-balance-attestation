# `lez-verifier-program`

Deployable LEZ program for the balance-attestation gate.

The guest binary at `guest/src/bin/balance_attestation_program.rs` follows the
LEZ program contract (`read_nssa_inputs::<Instruction>()` →
`ProgramOutput::write()`) and is shipped to a sequencer via
`wallet deploy-program <bin>.bin`. The assigned LEZ `ProgramId` is the RISC0
image id of the ELF, computed deterministically by `risc0-build` at compile
time and re-exported as `BALANCE_ATTESTATION_PROGRAM_ID`.

This crate is an **isolated workspace**: it depends on `nssa_core` from the
local LEZ checkout (`logos/`). Default `cargo build --workspace` from the
repository root does not touch this crate; the workspace member list in the
root `Cargo.toml` deliberately excludes it.

## Build

The LEZ checkout must be present at `<repo-root>/logos/`.

```sh
cargo build --release --manifest-path lez-verifier/program/Cargo.toml
```

The ELF lands under the RISC Zero guest target tree, for example:

```text
lez-verifier/program/target/riscv-guest/lez-verifier-program/lez-verifier-program-guest/riscv32im-risc0-zkvm-elf/release/balance_attestation_program.bin
```

## Instructions

The program understands three instructions, dispatched by `Instruction` in
`guest/src/bin/balance_attestation_program.rs`.

### `RegisterPresenter { presenter_pubkey }`

Pre-states:

| Index | Account | Required state |
| --- | --- | --- |
| 0 | presenter | uninitialized (`Account::default()`) |
| 1 | admin | `is_authorized == true` |

Post-states write the 32-byte BIP-340 x-only presenter pubkey into the
presenter account's `data`, claim ownership, and pass the admin account through
unchanged. `Admit` later requires this presenter account to be owned by the
program and hashes `data[..32]` to compare with `accepted_presenter_id`.

### `InitGate { chain_id, verifier_id, gate_id, threshold, expected_inner_image_id }`

Pre-states:

| Index | Account | Required state |
| --- | --- | --- |
| 0 | gate state | uninitialized (`Account::default()`) |
| 1 | admin | `is_authorized == true` |

Post-states write a borsh-encoded `GateState` into the gate state account's
`data`, claim ownership, and pass the admin account through unchanged.

### `Admit { outer_journal: Vec<u8> }`

`outer_journal` is the borsh-serialized `OuterJournal` (mirrors
`lez_verifier::LezGateJournal`).

Pre-states:

| Index | Account | Required state |
| --- | --- | --- |
| 0 | gate state | initialized, `program_owner == self_program_id` |
| 1 | presenter | registered by `RegisterPresenter`, `is_authorized == true`, `account.data[..32]` is the BIP-340 x-only Schnorr pubkey |

The program checks the journal version, the inner image id matches the gate
state's pinned `expected_inner_image_id`, the gate context id is recomputed
and equals the journal's, the threshold matches, the presenter id equals
`hash_segments([PRESENTER_DOMAIN, presenter_pubkey])`, and the nullifier has
not been admitted before. On success, the nullifier is appended to the gate
state and the new state is committed in the post-state.

## Trust bridge (important)

This program does **not** call `env::verify` on the outer balance-attestation
receipt. Spike 06 established that a deployed public LEZ program has no
`add_assumption` channel for an external receipt. Spike 08 then showed that
the current local sequencer accepts `Admit { outer_journal }` without binding
that journal to an external RISC Zero receipt at admission time.

For this Workable path, receipt verification must happen before submission:

- The CLI/SDK verifies the public envelope with
  `attestation_verifier::verify_envelope`.
- The CLI/SDK verifies or constructs the outer gate proof (`LEZ_BALANCE_GATE_ID`)
  when using the recursive gate path.
- Only after those host-side checks pass does it submit
  `Instruction::Admit { outer_journal }`.
- The deployed program treats the journal in instruction calldata as
  host-verified and validates only the application-level binding (gate
  context, exact threshold, presenter account hash, nullifier dedup).

This is **not** a complete LP-0005 on-chain verifier yet. It is a deployable
LEZ gate ledger plus nullifier set with host-side cryptographic enforcement.
To satisfy LP-0005 strictly, we still need either an evaluator-approved
interpretation of this Workable path or a LEZ-supported receipt-assumption /
native-verifier path that rejects forged journals before program execution.

## BAxxx error codes

Panics carry the deterministic error codes from
`crates/attestation-core/src/error.rs` so a host runner can parse them out of
the program failure log:

- `BA100 InvalidEnvelopeVersion`
- `BA102 InvalidImageId`
- `BA200 InvalidReceipt`
- `BA201 ThresholdMismatch`
- `BA202 ContextMismatch`
- `BA203 PresenterMismatch`
- `BA206 DuplicateNullifier`
- `BA500 GateAlreadyInitialized`
- `BA501 GateNotInitialized`
- `BA502 UnauthorizedPresenterAccount`
- `BA503 InvalidGateAccount`

## Wire layout cross-references

- `GateState` borsh layout: `idl/balance-attestation-verifier.json`
  (`accounts/GateState`).
- `OuterJournal` borsh layout: `lez_verifier::LezGateJournal` in
  `lez-verifier/src/lib.rs`.
- `derive_context_id` / `derive_presenter_id` segments and `hash_segments`
  framing: `crates/attestation-core/src/hash.rs`.
