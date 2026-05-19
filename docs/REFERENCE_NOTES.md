# Reference Notes

These notes capture public implementation facts that influenced the current
design. They intentionally avoid local filesystem paths, private account data,
and scratchpad-style review text.

## LP-0005 Requirements Used By This Repo

LP-0005 asks for a reusable private balance attestation primitive:

```text
private LEZ balance >= public threshold
```

The important engineering requirements are:

- prove the threshold without revealing the private account id, nullifier public
  key, exact balance, nonce, or account data
- target the real LEZ private-account commitment format
- verify Merkle membership against a sequencer-provided proof
- bind the proof to a specific gate/context
- bind presentation to a presenter key or authenticated session
- support both off-chain verification and an on-chain gate path
- provide CLI/SDK, IDL, Basecamp usability work, benchmarks, and final demo
  evidence with `RISC0_DEV_MODE=0`

## Prior Failure Modes To Avoid

Earlier public LP-0005 work showed several failure modes this repo explicitly
guards against:

- RISC Zero version drift between the project and the LEZ checkout
- dummy Merkle data instead of the real `getProofForCommitment` path
- invented sequencer endpoints
- verifier code that works as a Rust library but is not represented in a LEZ
  deployable path
- demos that run only in `RISC0_DEV_MODE=1`

Current guardrails:

- `scripts/check-risc0-version.sh`
- `scripts/m2-check-lez-commitment-compat.sh`
- `scripts/check-wallet-preflight.sh`
- `scripts/demo-local-sequencer-e2e.sh`
- `scripts/demo-local-gate-e2e.sh`
- `scripts/demo-local-full-e2e.sh`

## LEZ Files Worth Inspecting

When updating this project against a new LEZ checkout, inspect these files in
that checkout:

```text
nssa/core/src/commitment.rs
nssa/core/src/account.rs
nssa/core/src/program.rs
nssa/src/state.rs
nssa/src/privacy_preserving_transaction/circuit.rs
sequencer/service/rpc/src/lib.rs
sequencer/service/src/service.rs
wallet/src/lib.rs
wallet/src/cli/account.rs
wallet/src/cli/mod.rs
program_methods/guest/src/bin/privacy_preserving_circuit.rs
```

Relevant observed facts:

- The local LEZ checkout currently pins `risc0-zkvm = "3.0.5"` and
  `risc0-build = "3.0.5"`.
- The real membership-proof RPC is `getProofForCommitment`.
- `Account` contains `program_owner`, `balance`, `data`, and `nonce`.
- Private account state is local wallet state.
- `wallet account get --account-id Private/<id>` reads local wallet storage.
- `wallet account sync-private` refreshes private state from encrypted
  transaction outputs.

## Commitment Compatibility

The LP text describes the commitment conceptually as:

```text
SHA256(npk || program_owner || balance || nonce || SHA256(data))
```

The implementation must follow the checked-out LEZ code exactly. The current
LEZ implementation uses its own domain-separated helper, so this repo tests
against `nssa_core` instead of relying on a hand-written formula.

The compatibility harness compares:

- `attestation_core::derive_lez_private_account_commitment`
- LEZ `nssa_core::Commitment::new`
- LEZ commitment leaf hashing
- LEZ Merkle path root reconstruction

Run:

```sh
scripts/m2-check-lez-commitment-compat.sh
```

Expected result:

```text
commitment_match: true
leaf_hash_match: true
membership_root_match: true
```

## On-Chain Verification Finding

Public LEZ execution currently does not expose a way to inject receipt
assumptions for verifying an external standalone RISC Zero receipt from inside
a public LEZ guest.

The local symptom from the direct public verifier spike was:

```text
sys_verify_integrity: no receipt found to resolve assumption
```

Spike 06 confirmed:

- public LEZ execution uses a default executor environment
- public LEZ execution does not add receipt assumptions
- Logos-native private execution does use assumptions internally
- no local public verifier API for external receipts was found

Current design consequence:

```text
off-chain path: standalone proof envelope verified locally
on-chain path: host-verified proof, then LEZ gate ledger/nullifier transaction
```

This Workable on-chain path is implemented and demoed locally, but it is not a
cryptographic in-guest receipt verifier. That limitation is documented in
[ONCHAIN_PATH_DECISION.md](ONCHAIN_PATH_DECISION.md) and
[PRIZE_CHECKLIST.md](PRIZE_CHECKLIST.md).

## Public Prior-Art Notes

The project reviewed public Logos token and forum-related work for patterns,
not as code to copy. Useful patterns included:

- keeping shared types in a small core crate
- separating proof generation, verification, CLI, and app integrations
- using BIP-340/Schnorr-style presenter signatures
- using fast in-memory integration tests before live sequencer tests
- keeping Basecamp-facing code behind a thin SDK/UI boundary

Those notes have been folded into the current implementation and docs. The repo
does not vendor code from those reference projects.

## Current Local Evidence

Latest full local E2E evidence, after switching to the synced LEZ fork:

```text
run: .demo-runs/local-full/20260508T150913Z
RISC0_DEV_MODE: 0
proof phase: 00:01:48
gate phase: 00:01:42
total: 00:03:30
verify status: ok
nullifier count: 1
duplicate admit: not-applied
```

The proof phase uses:

```text
wallet private state -> getProofForCommitment -> witness.json -> RISC Zero envelope
```

The gate phase uses:

```text
attestation-verifier precheck -> LEZ gate transaction -> account.data nullifier record
```

`witness.json` is private and must not be published. Public artifacts are the
proof envelope, verifier output, gate report, and run summary.

## Spike 09 PPE-Native Gate Evidence

Spike 09 tested the Logos-native private execution alternative for LP-0005's
on-chain path.

```text
private holder account
  -> LEZ privacy-preserving transaction
  -> guest checks balance >= threshold
  -> public BAP1 gate/nullifier state update
```

Latest local result:

```text
RISC0_DEV_MODE: 0
positive admit: passed
duplicate admit: BA206 DuplicateNullifier
insufficient balance: BA201 ThresholdMismatch
```

This is stronger than the host-preverified public gate ledger because the
balance condition is proven inside LEZ private execution. It is still not the
same portable off-chain proof envelope transported over Messaging, so final
LP-0005 submission should keep asking evaluators whether this is the intended
on-chain verifier model.
