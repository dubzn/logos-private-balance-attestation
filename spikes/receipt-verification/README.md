# Spike 00: Receipt Verification Inside LEZ

This spike attacks Blocker 0 from `docs/RISK_SPIKES.md`.

Question:

```text
Can an on-chain LEZ program verify a RISC Zero proof/receipt and gate a public
action?
```

## Why This Comes First

LP-0005 requires an on-chain verifier program. If the current LEZ public
execution path cannot verify a submitted RISC Zero proof, then the project must
pivot early instead of building the whole SDK/UI stack around an invalid
assumption.

## What This Spike Adds

```text
spikes/receipt-verification/
|-- README.md
`-- lez/
    |-- guest/src/bin/receipt_gate.rs
    `-- runner/src/bin/run_receipt_gate.rs
```

`receipt_gate.rs` is a minimal LEZ guest that calls:

```rust
risc0_zkvm::guest::env::verify(image_id, expected_journal_words)
```

and only writes to an account if verification succeeds.

`run_receipt_gate.rs` is a host runner that submits a public transaction against
that guest.

## Expected Interpretation

If the transaction succeeds, direct receipt verification inside a public LEZ
guest is viable and we can build the real verifier in this direction.

If the transaction fails because no RISC Zero assumption/receipt can be supplied
to public execution, that is still useful: it points us toward either:

- a recursive/native verifier path exposed by LEZ/RISC Zero, or
- a Logos-native private execution gate that the evaluator explicitly accepts.

## Commands

Static inspection:

```sh
scripts/spike-00-inspect-lez-proof-path.sh
```

Build the guest into the local LEZ checkout:

```sh
scripts/spike-00-build-lez-program.sh
```

Run against a local sequencer:

```sh
export SPIKE_ACCOUNT=<public-account-id-without-Public>
scripts/spike-00-run-direct-receipt-gate.sh
```

The run script assumes:

- local sequencer is already running
- `wallet` is installed
- `NSSA_WALLET_HOME_DIR` points at the wallet containing `SPIKE_ACCOUNT`

