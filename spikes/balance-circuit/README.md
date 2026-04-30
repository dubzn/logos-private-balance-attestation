# Spike 03: Balance Attestation Circuit

This spike tests the first standalone RISC Zero balance attestation guest.

Question:

```text
Can a RISC Zero guest verify:
  1. private balance >= public threshold
  2. LEZ-compatible private account commitment reconstruction
  3. Merkle membership path resolves to a public commitment root
without leaking the private account fields in the journal?
```

## Why This Comes Next

Spike 02 proved the wallet/sequencer path can produce a real membership proof.
This spike checks that the proof can be consumed inside a standalone zkVM guest.

This is not the final LP-0005 circuit yet. It does not include presenter
binding, context nullifier derivation, proof envelope encoding, Messaging, or
on-chain verifier integration.

## What This Spike Adds

```text
spikes/balance-circuit/
|-- README.md
`-- lez/
    |-- guest/src/bin/balance_attestation_spike.rs
    `-- runner/src/bin/prove_balance_attestation_spike.rs
```

The runner has fixture modes for fast local checks and a live mode that reads a
private account from local wallet state.

## Commands

Build:

```sh
scripts/spike-03-build-balance-circuit.sh
```

Run fixture checks:

```sh
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh
```

Run against a live private account:

```sh
export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
export THRESHOLD=25
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live
```

Run a live negative threshold check:

```sh
export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
export THRESHOLD=999999
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live-below-threshold
```

Pass:

- valid fixture proves and verifies
- below-threshold fixture fails during proving
- wrong-root fixture fails during proving
- live mode proves and verifies for a funded account above threshold
- live below-threshold mode fails during proving with the expected guest error
