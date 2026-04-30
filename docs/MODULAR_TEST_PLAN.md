# Modular Test Plan

The project should not wait for a full end-to-end demo before discovering
whether individual parts work. Each layer below gets its own local command.

## Layer 0: Environment And Risk Inspection

Purpose:

```text
Confirm the local LEZ checkout and RISC Zero assumptions before coding deeper.
```

Commands:

```sh
scripts/check-risc0-version.sh
scripts/spike-00-inspect-lez-proof-path.sh
```

Pass:

- LEZ RISC Zero version is detected.
- Static inspection finds `env::verify` in private execution and confirms the
  public execution path needs runtime validation.

## Layer 1: Blocker 0 Direct Receipt Gate

Purpose:

```text
Test whether a public LEZ program can gate an action through RISC Zero
guest-side receipt verification.
```

Commands:

```sh
scripts/spike-00-build-lez-program.sh

export SPIKE_ACCOUNT=<public-account-id-without-Public>
scripts/spike-00-run-direct-receipt-gate.sh
```

Pass:

- `receipt_gate.bin` builds.
- Program deploys.
- Transaction succeeds or fails with a clear sequencer error that tells us which
  path to test next.

The first run may fail because `receipt_gate` intentionally calls
`env::verify` without a real assumption channel. That failure is useful data,
not wasted work.

## Layer 2: Logos-Native Private Execution Gate

Purpose:

```text
Prototype the likely fallback: use LEZ private execution itself as the proof
system for the on-chain gate.
```

Command:

```sh
scripts/spike-01-build-private-gate.sh

export PRIVATE_ACCOUNT=<private-account-id-without-Private>
export GATE_ACCOUNT=<public-account-id-without-Public>
export THRESHOLD=1
scripts/spike-01-run-private-gate.sh
```

Automated fixture command:

```sh
RISC0_DEV_MODE=1 scripts/spike-01-demo-private-gate.sh
```

The demo command creates fresh accounts, funds a private holder, runs a
positive gate, then runs a negative gate with `threshold = balance + 1`.
It polls account readback so a slow block interval does not produce a false
negative.

Pass:

- Private account with sufficient balance can claim public access.
- Insufficient balance fails.
- Public output does not reveal the private account id or balance.
- Evaluators confirm this satisfies LP-0005, or it remains a fallback only.

## Layer 3: Core Hashing And Types

Purpose:

```text
Make pure logic testable without sequencer, wallet, RISC Zero, or Basecamp.
```

Future command:

```sh
cargo test -p attestation-core
```

Pass:

- context id derivation is stable
- nullifier derivation is domain-separated
- proof envelope serialization is stable
- deterministic error codes round-trip

## Layer 4: LEZ Commitment Compatibility

Purpose:

```text
Prove our commitment helper matches the local LEZ implementation exactly.
```

Future command:

```sh
cargo test -p attestation-core commitment
```

Pass:

- our helper matches `nssa_core::Commitment::new`
- domain prefix and endian encoding are covered by tests

## Layer 5: Sequencer Adapter

Purpose:

```text
Use real JSON-RPC getProofForCommitment, not invented endpoints.
```

Current spike command:

```sh
export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
scripts/spike-02-inspect-membership-proof.sh
```

Future CLI command:

```sh
cargo run -p attestation-cli -- fetch-membership-proof \
  --account Private/<id> \
  --json
```

Pass:

- initialized private account returns a membership proof
- missing/stale private account returns a clear error
- proof root can be computed from the returned path

## Layer 6: Balance Circuit

Purpose:

```text
Prove balance >= threshold over a real private account commitment.
```

Current spike commands:

```sh
scripts/spike-03-build-balance-circuit.sh
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh

export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
export THRESHOLD=25
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live

export THRESHOLD=999999
RISC0_DEV_MODE=1 scripts/spike-03-run-balance-circuit.sh live-below-threshold
```

Future command:

```sh
cargo test -p attestation-prover
cargo run -p attestation-cli -- prove \
  --account Private/<id> \
  --threshold 100 \
  --context local/demo \
  --presenter Public/<id> \
  --out proof.json
```

Pass:

- proof succeeds above threshold
- proof fails below threshold
- bad Merkle root fails inside the guest
- receipt journal does not include exact balance, `npk`, account data, nonce,
  private keys, or Merkle siblings
- tampered context, threshold, or root fails verification

## Layer 6.5: Binding Circuit

Purpose:

```text
Bind the proof to context, presenter, and nullifier before creating the
production proof envelope.
```

Current spike commands:

```sh
scripts/spike-04-build-binding-circuit.sh
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh

export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
export THRESHOLD=25
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live

export THRESHOLD=999999
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live-below-threshold
```

Pass:

- proof succeeds above threshold
- proof fails below threshold
- bad Merkle root fails inside the guest
- bad presenter id fails inside the guest
- bad context nullifier fails inside the guest
- changing the context changes `context_id` and `context_nullifier`
- receipt journal does not include the commitment leaf

## Layer 6.6: Dev/Prod Proving Baseline

Purpose:

```text
Measure the current circuit with RISC0_DEV_MODE=1 and RISC0_DEV_MODE=0 before
the final demo depends on real proving.
```

Planned commands:

```sh
scripts/spike-05-run-devmode-baseline.sh
scripts/spike-05-run-prod-baseline.sh
```

Output contract:

```text
| Step | Command | Status | Output | Duration |
| --- | --- | --- | --- | --- |
| total | - | ok/fail | result file path | 00:00:00 |
```

Pass:

- each step records status, important output, and duration
- dev-mode and prod-mode results are written to separate Markdown files
- prod mode runs with `RISC0_DEV_MODE=0`
- failures identify the exact failing step

## Layer 7: Off-Chain Verification And Messaging

Purpose:

```text
Verify the same proof locally and transport it over Logos Messaging.
```

Future commands:

```sh
cargo run -p attestation-cli -- verify-offchain --proof proof.json
cargo run -p attestation-cli -- send-proof --proof proof.json --topic <topic>
cargo run -p attestation-cli -- receive-proof --topic <topic>
```

Pass:

- recipient verifies proof without private witness data
- messaging failure does not leak private state

## Layer 8: Basecamp

Purpose:

```text
Run the CLI-backed flow from a visual app.
```

Future command:

```sh
nix run ./apps/basecamp
```

Pass:

- generate proof
- verify off-chain
- run on-chain gate
- display status without private witness fields

## Final Layer: E2E

Purpose:

```text
Clean local sequencer -> private account -> proof -> on-chain gate ->
Messaging -> Basecamp/readback.
```

Future command:

```sh
RISC0_DEV_MODE=0 scripts/demo-e2e.sh
```

Pass:

- works from a clean checkout
- prints progress markers during proving
- records program ids and proof generation time
