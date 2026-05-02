# On-Chain Path Decision

Date: 2026-05-02.

Spike 06 closes the current on-chain verifier-path decision for the next
implementation milestone.

## Short Decision

Do not build the next milestone around a public LEZ program that directly
verifies an external standalone RISC Zero receipt.

Use a two-track architecture:

1. **Off-chain path:** standalone RISC Zero proof envelope verified locally.
2. **On-chain path:** Logos-native private execution gate as the only working
   local route, pending evaluator confirmation for LP-0005.

## Evidence

Reproduce the inspection:

```sh
scripts/spike-06-inspect-onchain-path.sh
```

Observed local result:

| Check | Result |
| --- | --- |
| Public LEZ execution uses default executor | yes |
| Public LEZ execution adds receipt assumptions | 0 |
| Private LEZ proof host adds assumptions | 1 |
| Private LEZ guest calls `env::verify` | 1 |
| Private LEZ proof uses succinct receipts | yes |
| Spike 0A missing assumption error recorded | observed |

Relevant local files:

```text
/Users/dub/logos/src/logos-execution-zone/nssa/src/program.rs
/Users/dub/logos/src/logos-execution-zone/nssa/src/privacy_preserving_transaction/circuit.rs
/Users/dub/logos/src/logos-execution-zone/program_methods/guest/src/bin/privacy_preserving_circuit.rs
```

## Interpretation

RISC Zero assumptions are not the problem by themselves. LEZ already uses them
inside the Logos-native private execution circuit:

```text
program receipt -> env_builder.add_assumption(inner_receipt)
privacy_preserving_circuit -> env::verify(program_id, program_output)
```

The blocker is the public LEZ execution path. Public program execution uses a
plain executor environment and does not expose an assumption injection point for
an externally supplied receipt. Spike 0A already showed the runtime symptom:

```text
sys_verify_integrity: no receipt found to resolve assumption
```

RISC Zero also exposes recursion/succinct receipt tooling, but that does not
currently become a deployable public LEZ verifier path unless LEZ exposes one
of these:

- a public transaction field for receipt assumptions
- a native LEZ verifier API for external receipts
- an evaluator-approved pattern for submitting a receipt into public execution

No such local path was found in the inspected LEZ checkout.

## Impact On LP-0005

This does not block the off-chain path. The off-chain path can use the Spike 04
binding proof envelope and verify it locally.

It does block a naive on-chain verifier program that accepts a receipt and calls
`env::verify` in public execution.

For now, the on-chain path should be treated as:

```text
private account -> Logos-native private execution gate -> public gate claim
```

That path works locally, but it needs evaluator confirmation because LP-0005
wording asks for a reusable proof accepted by a LEZ verifier program.

## Next Implementation Consequence

Milestone 1 can start with reusable off-chain primitives:

```text
attestation-core
proof journal/envelope types
context/nullifier derivation
off-chain verifier shape
```

The LEZ verifier program should remain behind an interface until evaluator or
Logos team feedback confirms the expected on-chain verification path.
