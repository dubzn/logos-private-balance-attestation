# Spike 04: Binding Circuit

This spike upgrades the Spike 03 balance circuit with the proof-shape decisions
that must exist before Milestone 1:

```text
context binding
presenter binding
context nullifier
public journal without the private commitment leaf
```

## What It Proves

The RISC Zero guest checks:

1. `account.balance >= threshold`.
2. `Commitment::new(npk, account)` resolves to the public Merkle root.
3. `presenter_secret` derives the public `presenter_id`.
4. `context_id` is derived from public gate data and the circuit image id.
5. `context_nullifier` is derived from `npk`, `context_id`, and `presenter_id`.

The journal includes:

```text
threshold
commitment_root
context_id
context_nullifier
presenter_id
verifier_id
circuit_image_id
proof_index
proof_depth
```

It does not publish the exact balance, `npk`, private account data, nonce,
private keys, Merkle siblings, or the commitment leaf.

## Presenter Binding Note

Spike 04 uses a synthetic 32-byte presenter secret:

```text
presenter_id = H("logos-balance-attestation/v1/presenter" || presenter_secret)
```

That proves the circuit shape. The production adapter still needs to map this
to a wallet-compatible presenter identity or add an envelope signature over a
fresh verifier challenge.

## Commands

Build:

```sh
scripts/spike-04-build-binding-circuit.sh
```

Run fixture checks:

```sh
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh
```

Run against a live private account:

```sh
export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
export THRESHOLD=25
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live
```

Run a live negative threshold check:

```sh
export PRIVATE_ACCOUNT=<initialized-private-account-id-without-Private>
export THRESHOLD=999999
RISC0_DEV_MODE=1 scripts/spike-04-run-binding-circuit.sh live-below-threshold
```

## Pass Conditions

- valid fixture proves and verifies
- below-threshold fixture fails during proving
- wrong-root fixture fails during proving
- wrong-presenter fixture fails during proving
- wrong-nullifier fixture fails during proving
- context variant fixture proves twice and shows different context ids and
  different nullifiers
- live mode proves and verifies for a funded account above threshold
- live below-threshold mode fails during proving with the expected guest error
