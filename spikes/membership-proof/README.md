# Spike 02: Private Commitment Membership Proof

This spike attacks the next LP-0005 blocker after the on-chain verifier path.

Question:

```text
Can the client reconstruct the current LEZ private account commitment and fetch
a real Merkle membership proof from the sequencer through getProofForCommitment?
```

## Why This Comes Before The Circuit

The final RISC Zero circuit needs a witness containing:

- private account fields from local wallet storage
- the exact LEZ-compatible commitment
- the sequencer membership proof for that commitment
- the public commitment root

If the wallet/sequencer path cannot provide that proof for an initialized
private account, a balance attestation proof cannot target live LEZ state.

## What This Spike Adds

```text
spikes/membership-proof/
|-- README.md
`-- lez/
    `-- runner/src/bin/inspect_private_commitment.rs
```

`inspect_private_commitment.rs` is a host runner. It does not deploy a LEZ
program. It uses the local wallet to:

1. find a private account in local wallet storage
2. reconstruct the account commitment with `WalletCore::get_private_account_commitment`
3. call the real sequencer RPC through `WalletCore::check_private_account_initialized`
4. compute the Merkle root from the returned proof

Normal output avoids printing private witness fields such as exact balance,
`npk`, private keys, or proof siblings.

## Commands

Start a local sequencer first:

```sh
cd "$HOME/logos/src/logos-execution-zone"
RISC0_DEV_MODE=1 RUST_LOG=info cargo run --features standalone -p sequencer_service \
  sequencer/service/configs/debug/sequencer_config.json
```

Run the spike from this repo:

```sh
export PRIVATE_ACCOUNT=<private-account-id-without-Private>
scripts/spike-02-inspect-membership-proof.sh
```

Pass:

```text
membership_proof_found = true
proof_depth > 0
commitment_root_hex is present
```

