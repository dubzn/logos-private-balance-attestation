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
- `attestation-prover` initial witness builder with redacted debug/summary
  output
- `attestation-cli inspect-private` for local-only and real
  `getProofForCommitment` inspection
- M2 compatibility scripts against local `nssa_core` and `WalletCore`
- real `getProofForCommitment` proof mode passed against a local sequencer
- `scripts/demo-local-sequencer-e2e.sh` harness that builds a private witness
  from real wallet state, proves, verifies, and writes a run report
- local-sequencer E2E passed with `RISC0_DEV_MODE=1` against a real wallet
  private account and real membership proof
- local-sequencer E2E passed with `RISC0_DEV_MODE=0`: total 00:01:38, proving
  00:00:23, verify `status: ok`

Current command set:

```sh
cargo test
scripts/m2-check-lez-commitment-compat.sh
cargo run -p attestation-cli -- inspect-private --account Private/<id> --local-only
cargo run -p attestation-cli -- inspect-private --account Private/<id> --require-proof
PRIVATE_ACCOUNT=Private/<id> scripts/demo-local-sequencer-e2e.sh
```

## Ordered Backlog

1. Finish Milestone 2 as reusable product code.
   - Move the remaining M2 temporary wallet adapter into Rust.
   - Keep LEZ path dependencies isolated from the portable workspace.
   - Replace the CLI's script bridge with a direct Rust adapter when packaging
     the LEZ wallet dependency is clean.
   - Preserve the current redaction policy.

2. Build the production RISC Zero method.
   - Port Spike 04 into `methods/guest`.
   - Reuse or mirror `attestation-core` hash ordering exactly.
   - Produce `BalanceAttestationJournal`.
   - Keep commitment leaf and witness data out of the journal.

3. Harden the real local-sequencer E2E.
   - Re-run `scripts/demo-local-sequencer-e2e.sh` from a clean wallet/sequencer.
   - Turn the successful `RISC0_DEV_MODE=0` run into a clean-room recording
     flow for the final demo.
   - Decide whether the temporary wallet adapter stays script-local or moves
     behind a reusable Rust boundary.
   - Keep `witness.json` private and only publish envelope/report artifacts.

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
