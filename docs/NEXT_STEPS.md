# Next Steps

This is the current ordered backlog. Keep this file aligned with
`IMPLEMENTATION_PLAN.md`, `MODULAR_TEST_PLAN.md`, and `PRIZE_CHECKLIST.md`.

## Current Position

Done locally:

- risk spikes for on-chain proof path, private execution fallback, membership
  proof retrieval, balance circuit, binding circuit, dev/prod baseline, and
  on-chain decision
- `attestation-core` with envelope/journal types, context hashing,
  presenter/nullifier helpers, LEZ commitment helpers, and Merkle root helpers
- `attestation-prover` initial sanitized private account inspection report
  helpers
- M2 compatibility scripts against local `nssa_core` and `WalletCore`
- real `getProofForCommitment` proof mode passed against a local sequencer

Current command set:

```sh
cargo test
scripts/m2-check-lez-commitment-compat.sh
PRIVATE_ACCOUNT=<id> scripts/m2-inspect-private-account.sh --local-only
PRIVATE_ACCOUNT=<id> scripts/m2-inspect-private-account.sh --require-proof
```

## Ordered Backlog

1. Finish Milestone 2 as reusable product code.
   - Move more of the M2 temporary wallet adapter into Rust.
   - Keep LEZ path dependencies isolated from the portable workspace.
   - Add an `attestation-cli inspect-private` command.
   - Preserve the current redaction policy.

2. Build the production RISC Zero method.
   - Port Spike 04 into `methods/guest`.
   - Reuse or mirror `attestation-core` hash ordering exactly.
   - Produce `BalanceAttestationJournal`.
   - Keep commitment leaf and witness data out of the journal.

3. Add `attestation-prover` proof generation.
   - Build witness from local wallet private state.
   - Fetch membership proof through `getProofForCommitment`.
   - Run the RISC Zero prover.
   - Write `BalanceAttestationEnvelope`.

4. Add `attestation-verifier` off-chain verification.
   - Verify RISC Zero receipt.
   - Decode journal.
   - Check expected threshold, context, image id, presenter id, and challenge
     binding.
   - Return deterministic `BAxxx` errors.

5. Add CLI workflow.
   - `inspect-private`
   - `prove`
   - `verify-offchain`
   - later: `send-proof`, `receive-proof`, `claim-access`

6. Resolve the on-chain path.
   - Wait for evaluator/Discord answer if possible.
   - Keep direct public receipt verification marked unsupported for this LEZ
     version.
   - Implement the Logos-native private execution gate fallback behind an
     explicit interface.

7. Add Messaging transport.
   - Start behind a local adapter.
   - Replace with the accepted Logos Messaging path once confirmed.

8. Add Basecamp GUI.
   - Backend-backed `ui_qml`.
   - Wrap CLI/backend commands.
   - Show only public/sanitized proof state.

9. Build reference integrations.
   - Governance/access gate.
   - Messaging group gate.
   - Third integration with external validation.

10. Submission hardening.
    - CI.
    - Clean local E2E.
    - `RISC0_DEV_MODE=0` final demo.
    - Benchmarks and CU docs.
    - SPEL/IDL artifact.
    - Testnet deployment details.
    - Narrated video.
