# Spike 08 — LEZ program chaining + receipt-binding probe

## Question

Once the deployable LEZ program from `lez-verifier/program/` is registered on
a local sequencer via `wallet deploy-program`, what does the LEZ submission
path require to bind the on-chain `Admit` instruction to the off-chain
balance-attestation receipt? Three pre-stated outcomes from the plan:

- **Best**: the privacy-preserving transaction circuit transparently chains
  our program in a way that lets us attach the outer balance-attestation
  receipt as an admission assumption — i.e. the LEZ tx admission step
  refuses to admit a transaction whose `Admit` instruction is not backed by a
  real outer receipt against `LEZ_BALANCE_GATE_ID`.
- **Workable**: the LEZ does not bind the receipt itself, but the program
  runs as a privacy-preserving transaction inner program and the host can be
  trusted to call `attestation_verifier::verify_envelope` before submitting.
  The on-chain program then serves as a deterministic ledger of admissions
  and a nullifier-dedup mechanism, with the cryptographic trust seated at the
  host. Documented as such in `docs/PRIZE_CHECKLIST.md`.
- **Blocked**: the local sequencer rejects deployed-program transactions of
  this shape outright.

## Pre-known evidence (Spike 06)

`docs/ONCHAIN_PATH_DECISION.md` already established that direct public
`env::verify` of an external receipt is not exposed by the local LEZ build
(`sys_verify_integrity: no receipt found to resolve assumption`). That rules
out **Best** unless something has changed upstream — which we should reverify
with the actual deployed-program flow before declaring a result.

The existing `spikes/private-balance-gate/lez/runner/src/bin/run_private_balance_gate.rs`
demonstrates that an arbitrary deployed program can be invoked via
`WalletCore::send_privacy_preserving_tx`, so the **submission path itself**
is not the blocker — the blocker is whether that path lets us bind an
external receipt as an assumption.

## What this spike actually does

`scripts/spike-08-run.sh` (when run against a live local sequencer) executes:

1. Build `lez-verifier-program` (the deployable LEZ guest) into the
   `balance_attestation_program.bin` ELF.
2. `wallet deploy-program <bin>` and capture the assigned program id.
3. Create four LEZ accounts: a public gate-state account (uninitialized), a
   register-admin signer, a separate fresh init-admin signer, and a public
   presenter account.
4. Run `register_presenter`, which stores `PRESENTER_PUBKEY_HEX` in the
   presenter account's `data` and claims that account for the verifier program.
5. Run `init_gate` (the runner builds an `Instruction::InitGate` and submits
   it as an all-public `nssa::PublicTransaction`).
6. Run `admit` with a real-shape journal assembled from CLI fields. This
   runner does **not** consume an envelope or receipt; the first-class
   `balance-attest gate-admit` command is the operator-facing wrapper that
   performs host-side envelope verification before invoking this live adapter.
7. Wait until the nullifier appears in the gate-state account, then run
   `admit` again with the same nullifier:
   - First call should succeed and record the nullifier.
   - Second call must not append the nullifier again after a block-settle
     window. If it does, the gate ledger is unsafe for one-nullifier-one-admit.
8. Run `admit` once with a fabricated outer journal (no matching receipt at
   all). If the sequencer admits it, this confirms the sequencer does NOT
   bind external receipts and we are in the **Workable** branch (not
   Blocked).

## How to run

Requires a running local LEZ sequencer per `docs/LOCAL_SETUP.md`.

```sh
LOGOS_LEZ_REPO=$HOME/logos/src/logos-execution-zone \
  RISC0_DEV_MODE=1 \
  scripts/spike-08-run.sh
```

The script prints a final block:

```
SPIKE-08 RESULT: Best | Workable | Blocked
```

That marker is what fills out the result table below.

## Result

Latest run: 2026-05-05, local LEZ sequencer (`RISC0_DEV_MODE=1`)
Outcome:   **Workable**
Notes:

Successful hardened run submitted five transactions via the all-public
`nssa::PublicTransaction` path:

- `register_presenter` (`e796dddf…a4cf1`) — presenter account flipped to
  program-owned and stored the 32-byte test pubkey in `account.data`.
- `init_gate` (`554444ba…326b0`) — gate-state account flipped from
  `Account is Uninitialized` to the deployed program id, with borsh
  `GateState` starting with magic `0x42415431` ("BAT1").
- `admit` (`48732644…1eaefc`) — accepted; nullifier `aa…aa` appeared once in
  gate-state data.
- duplicate `admit` (`e9c69c22…d1347c`) — submitted, but after the settle
  window the nullifier still appeared exactly once.
- `admit-fabricated` (`bad1e9c5…f26e51`) — accepted; fabricated nullifier
  `bb…bb` appeared once in gate-state data.

The `admit-fabricated` accept is the discriminator. The local sequencer
admits any journal with the right shape; it does not bind the outer
balance-attestation receipt at admission time. That confirms the
architecture's host trust seat: `attestation_verifier::verify_envelope`
must run on the host before submission, otherwise the deployed program
would record forged admissions.

The script waits for the first nullifier to appear in account data before
probing duplicate admission. The expected Workable marker is:

- `ADMIT real-shape: ok`
- `ADMIT duplicate settled: rejected-or-not-applied`
- `ADMIT fabricated: ok`

If the duplicate appears twice in `GateState.admitted_nullifiers`, the result
must be treated as **Blocked** until the account-state race is resolved.

`BALANCE_ATTESTATION_PROGRAM_ID` is derived from the guest ELF at build time.
Rerun this spike after any guest source, LEZ `nssa_core`, or RISC Zero guest
toolchain change; RISC Zero image ids intentionally drift with those inputs.

## Decision (independent of the marker above)

Even before the live run, the architecture commits to the **Workable** path
because Spike 06's evidence is unlikely to be invalidated by a re-run. That
means:

- The live `gate-admit` CLI calls `attestation_verifier::verify_envelope` on
  the envelope **before** building the LEZ tx; if that fails, no tx is
  submitted. `gate-register-presenter` and `gate-init` cover the setup path.
- `docs/PRIZE_CHECKLIST.md` documents the host-side trust seat openly so the
  evaluator sees the limitation.
- `lez-verifier/program/` ships as deployable; the gate-state and nullifier
  set are real on-chain artifacts.

If the live spike returns **Best**, we tighten the host adapter to also
attach the outer receipt as an admission assumption (eliminating the host
trust seat) and bump `docs/PRIZE_CHECKLIST.md` accordingly. If it returns
**Blocked**, we surface the deployment failure mode in the prize checklist
and propose the LEZ-side change required.
