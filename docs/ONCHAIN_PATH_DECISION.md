# On-Chain Path Decision

Date: 2026-05-02.

Spike 06 closes the current on-chain verifier-path decision for the next
implementation milestone.

## Short Decision

Do not build the next milestone around a public LEZ program that directly
verifies an external standalone RISC Zero receipt.

Use a two-track architecture:

1. **Off-chain path:** standalone RISC Zero proof envelope verified locally.
2. **On-chain path:** deployable LEZ gate ledger plus nullifier set, with
   host-side proof verification before submission. This is the only working
   local route today and remains pending evaluator confirmation for LP-0005.

## Evidence

Reproduce the inspection:

```sh
scripts/spike-06-inspect-onchain-path.sh
```

Observed local result:

| Check | Result |
| --- | --- |
| Public LEZ execution uses default executor | yes |
| Public LEZ execution adds receipt assumptions | 0 |
| Private LEZ proof host adds assumptions | 1 |
| Private LEZ guest calls `env::verify` | 1 |
| Private LEZ proof uses succinct receipts | yes |
| Spike 0A missing assumption error recorded | observed |

Relevant files in the selected `logos-execution-zone` checkout:

```text
nssa/src/program.rs
nssa/src/privacy_preserving_transaction/circuit.rs
program_methods/guest/src/bin/privacy_preserving_circuit.rs
```

## Interpretation

RISC Zero assumptions are not the problem by themselves. LEZ already uses them
inside the Logos-native private execution circuit:

```text
program receipt -> env_builder.add_assumption(inner_receipt)
privacy_preserving_circuit -> env::verify(program_id, program_output)
```

The blocker is the public LEZ execution path. Public program execution uses a
plain executor environment and does not expose an assumption injection point for
an externally supplied receipt. Spike 0A already showed the runtime symptom:

```text
sys_verify_integrity: no receipt found to resolve assumption
```

RISC Zero also exposes recursion/succinct receipt tooling, but that does not
currently become a deployable public LEZ verifier path unless LEZ exposes one
of these:

- a public transaction field for receipt assumptions
- a native LEZ verifier API for external receipts
- an evaluator-approved pattern for submitting a receipt into public execution

No such local path was found in the inspected LEZ checkout.

## Feedback Request

Use this concise wording when asking Logos reviewers/builders for the missing
on-chain path detail:

```text
Quick LP-0005 implementation question.

We now have two local on-chain candidates implemented:

1. Workable public LEZ gate:
   host verifies the RISC Zero proof envelope, then submits a gate admit tx.
   This persists/dedupes the nullifier on-chain, but the public LEZ program
   does not verify the receipt itself.

2. PPE-native LEZ gate:
   LEZ private execution checks private balance >= threshold and writes the
   public gate/nullifier state. This passed locally with RISC0_DEV_MODE=0,
   including duplicate and insufficient-balance rejection.

The open question is: for LP-0005, should the final on-chain path be the
PPE-native LEZ flow, or do evaluators require the same portable off-chain
proof envelope to be verified by a public LEZ program?

Repo:
https://github.com/dubzn/logos-private-balance-attestation

Relevant docs:
docs/ONCHAIN_PATH_DECISION.md
docs/SUBMISSION_STATUS.md
```

Spike 09 has now prototyped the Logos-native private execution/PPE gate. The
remaining question is whether Logos evaluators consider that PPE-native proof
path the expected on-chain verifier model for LP-0005, or whether they require
a public LEZ program to verify an externally submitted standalone receipt.

## Impact On LP-0005

This does not block the off-chain path. The off-chain path can use the Spike 04
binding proof envelope and verify it locally.

It does block a naive on-chain verifier program that accepts a receipt and calls
`env::verify` in public execution.

For now, the on-chain path should be treated as:

```text
Path A: private account -> host-verified proof -> public LEZ gate ledger/nullifier claim
Path B: private account -> LEZ PPE balance check -> public gate/nullifier update
```

Path A works locally as an application gate, but the deployed public program
does not cryptographically verify the receipt; it records the host-verified
journal. Path B now works locally as a Logos-native private execution gate, but
it does not verify the same portable off-chain proof envelope. Both paths need
evaluator confirmation against LP-0005's on-chain wording before final
submission.

## Next Implementation Consequence

Milestone 1 can start with reusable off-chain primitives:

```text
attestation-core
proof journal/envelope types
context/nullifier derivation
off-chain verifier shape
```

The LEZ verifier program should remain behind an interface until evaluator or
Logos team feedback confirms the expected on-chain verification path.

## Spike 0C live implementation (2026-05)

The plan settled into the two-layer Spike 0C shape:

1. **Outer recursion artifact** —
   `lez-verifier/guest/src/bin/lez_balance_gate.rs` (built into
   `LEZ_BALANCE_GATE_ID`) is the receipt the host produces via
   `lez_verifier::prove_lez_gate`. This stays unchanged.
2. **Deployable LEZ program** —
   `lez-verifier/program/guest/src/bin/balance_attestation_program.rs`
   (built into `BALANCE_ATTESTATION_PROGRAM_ID`) is the on-chain program
   shipped with `wallet deploy-program`. It follows the LEZ program contract
   (`read_nssa_inputs::<Instruction>()` →
   `ProgramOutput::new(...).write()`), encodes its state as a borsh
   `GateState` in `pre_states[0].account.data`, and dispatches
   `RegisterPresenter` / `InitGate` / `Admit`. Roundtrip and failure-mode
   tests in `lez-verifier/program/tests/program_journal_roundtrip.rs` (6/6).

The deployable program does **not** call `env::verify`. The trust seat for
the outer balance-attestation receipt is at the host: the CLI calls
`attestation_verifier::verify_envelope` before building the LEZ tx. Spike 08
ran against a live local sequencer and confirmed the Workable branch: a
fabricated but well-formed journal was applied, so the sequencer does not bind
external receipts for this public program path. The host pre-verification is
therefore the documented sole cryptographic guarantee — see
`lez-verifier/program/README.md` and
`spikes/spike-08-program-chaining/README.md`.

`attestation-cli` now exposes the live setup/admission flow directly:
`gate-register-presenter`, `gate-init`, and `gate-admit`. This removes the
manual dependency on the Spike 08 script for normal operator testing while
keeping the same honest trust boundary.

## Spike 09 PPE-native gate (2026-05)

Spike 09 tested the Logos-native private execution alternative that LEZ already
supports internally through the privacy-preserving execution circuit.

The spike program receives:

1. a private holder account,
2. a public gate-state account,
3. a public presenter account.

Inside private execution it checks:

```text
holder is authorized
holder.balance >= threshold
presenter is authorized
gate is default or already owned by this program
context nullifier has not been admitted before
```

On success it writes public gate state into `account.data` with `BAP1` magic,
the gate context, threshold, and admitted `(context_nullifier,
presenter_account_id)` records. The private holder account id and balance are
not written to public state.

Local result:

```text
RISC0_DEV_MODE=0
positive admit: passed and wrote one nullifier
duplicate admit: failed with BA206 DuplicateNullifier
insufficient balance: failed with BA201 ThresholdMismatch
```

This is the strongest local on-chain candidate found so far because the
balance condition is checked by LEZ private execution rather than by a host
precheck. It is still a different artifact from the reusable off-chain proof
envelope, so the final submission should ask Logos to confirm whether this is
the intended LP-0005 on-chain verification path.
