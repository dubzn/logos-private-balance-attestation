# methods

Production RISC Zero circuit for LP-0005 Private Token Balance Attestation.

## What it proves

1. `balance >= threshold` (private, hidden in journal).
2. LEZ private account commitment resolves to the public Merkle root using the
   current LEZ `Commitment::new(account_id, account)` shape.
3. Presenter secret derives the committed `presenter_id`.
4. `context_id` is derived from public gate params (chain, circuit image, verifier, gate, threshold).
5. `context_nullifier` derives from `npk`, `context_id`, and `presenter_id` — anti-replay tag.

Journal schema mirrors `attestation_core::BalanceAttestationJournal` exactly (version=1).

## Build

Dev mode (no Docker, fast mock proofs):

```sh
RISC0_DEV_MODE=1 cargo build -p methods
```

Production mode (requires RISC Zero hardware prover or GPU):

```sh
cargo build -p methods
```

## Test

```sh
RISC0_DEV_MODE=1 cargo test -p methods --test journal_roundtrip
```

Verifies the receipt journal decodes field-for-field to the canonical
`attestation_core::BalanceAttestationJournal` and that host-side derivations
(`derive_context_id`, `derive_context_nullifier`, `derive_presenter_id`) match
guest output exactly.

## Known constraints

- **No `sha2` in guest.** The guest uses `risc0_zkvm::sha::Impl::hash_bytes` exclusively.
  Using `sha2` would bypass the RISC Zero SHA-256 accelerator and produce correct bytes
  but at ~10x the cycle cost.
- **Field order in `derive_context_id`.** Order is `[domain, chain_id, circuit_image_id,
  verifier_id, gate_id, threshold]` — mirrors `attestation_core/src/hash.rs:9-18`.
  Changing this order silently changes the image_id and breaks all existing context_ids.
- **Journal types.** `version: u16`, `proof_index: u64`, `proof_depth: u64`. Spike 04
  used `u32`/`usize` — those were corrected here to match `BalanceAttestationJournal`.
- **Image id stability.** `BALANCE_ATTESTATION_ID` in `methods.rs` is deterministic for a
  given guest binary. Any change to the guest source changes the image id, which must be
  re-committed to the on-chain verifier program.
