# Spike 10: Direct Cryptographic Receipt Verification

## Question

Can a RISC Zero guest shaped like a public LEZ program deserialize and
cryptographically verify a portable receipt without calling `env::verify` and
without relying on an externally injected assumption?

This is deliberately different from Spike 0A. Spike 0A called `env::verify`,
which records an assumption and failed in public LEZ execution because the host
did not provide a matching receipt. Spike 10 calls `Receipt::verify` directly.

## Phases

Compile the guest and record image IDs:

```sh
scripts/spike-10-direct-receipt-verifier.sh compile-only
```

Generate a real succinct receipt, verify it inside the direct-verifier guest,
and reject a tampered journal:

```sh
RISC0_DEV_MODE=0 \
  scripts/spike-10-direct-receipt-verifier.sh real-succinct
```

Run the same experiment with a smaller Groth16 receipt:

```sh
RISC0_DEV_MODE=0 \
  scripts/spike-10-direct-receipt-verifier.sh real-groth16
```

Groth16 proving requires Docker.

The real run may be slow. It uses real proving and intentionally refuses
`RISC0_DEV_MODE=1`, because a fake receipt would not answer the cryptographic
question.

## Pass Conditions

- both guests compile for `riscv32im-risc0-zkvm-elf`
- a real succinct receipt is accepted by `Receipt::verify` inside the verifier
  guest
- a receipt with a modified journal is rejected
- the report records receipt size, verifier user cycles, segments, and wall
  time
- the report repeats verification with LEZ's current public execution limit of
  32M user cycles and records whether that limit accepts or rejects the receipt

## Limits

Cryptographic verification alone does not prove that the receipt fits in a real
public LEZ transaction or stays within sequencer execution limits. The runner
therefore repeats the execution with the exact public LEZ cycle limit before
recommending a sequencer submission.

## Result (2026-07-13)

Both real receipt variants passed cryptographic verification inside the guest,
and both rejected a modified journal:

| Receipt | Bytes | Direct verifier cycles | Segments | LEZ 32M limit |
| --- | ---: | ---: | ---: | --- |
| Succinct | 223,191 | 313,056,015 | 315 | rejected |
| Groth16 | 470 | 162,362,189 | 167 | rejected |

The Groth16 receipt solves transaction payload size but still requires about
4.8 times LEZ's current public execution budget. The succinct receipt requires
about 9.3 times that budget.

The limit comes from latest LEZ at
lee/state_machine/src/program/mod.rs:

    const MAX_NUM_CYCLES_PUBLIC_EXECUTION: u64 = 1024 * 1024 * 32;

The runner applies that exact limit through
ExecutorEnvBuilder::session_limit. Both valid receipts fail with:

    Session limit exceeded: 33554432 >= 33554432

An actual public transaction cannot improve this result because LEZ applies the
same limit before decoding the program journal. We therefore stop before a
sequencer submission instead of presenting a predictably rejected transaction
as new evidence.

Artifacts:

    .spike-results/spike-10-direct-receipt-verifier/20260713T033401Z
    .spike-results/spike-10-direct-receipt-verifier/20260713T033503Z

Conclusion: explicit in-guest verification is cryptographically viable but
blocked by the current public LEZ cycle budget. A production path needs one of:

- a higher public execution limit
- a native receipt-verification syscall/precompile
- an external receipt-assumption channel
- evaluator acceptance of the PPE-native path
