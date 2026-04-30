# Spike 01: Logos-Native Private Balance Gate

This spike tests the fallback path from `docs/RISK_SPIKES.md`.

Question:

```text
Can LEZ private execution itself prove balance >= threshold and gate a public
claim without revealing the private account?
```

## Shape

The program receives two accounts:

1. private holder account
2. public gate account

The guest checks:

```text
private_holder.balance >= threshold
private_holder.is_authorized == true
public_gate.is_authorized == true
```

If the checks pass, it leaves the private account unchanged and writes a marker
to the public gate account.

## Why This Matters

The direct receipt-verification spike shows whether public LEZ execution can
verify a standalone RISC Zero proof. If that path is blocked by missing
assumptions, this spike tests a more native design: let LEZ's privacy-preserving
transaction circuit be the proof system for the on-chain gate.

This still needs evaluator confirmation before prize submission, because
LP-0005 asks for a proof that can be submitted to a verifier program. It may be
acceptable if the verifier program is executed privately and the sequencer
verifies the private execution proof.

## Commands

Build:

```sh
scripts/spike-01-build-private-gate.sh
```

Run:

```sh
export PRIVATE_ACCOUNT=<private-account-id-without-Private>
export GATE_ACCOUNT=<public-account-id-without-Public>
scripts/spike-01-run-private-gate.sh
```

The private account must already be initialized and funded. Use the wallet
native transfer flow from `docs/LOCAL_SETUP.md` to create that state.

Automated fixture:

```sh
RISC0_DEV_MODE=1 scripts/spike-01-demo-private-gate.sh
```

The automated fixture creates fresh accounts, funds a private holder, verifies
that `balance >= threshold` writes a public marker, and verifies that
`threshold = balance + 1` fails with `private balance is below threshold`.
