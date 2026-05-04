# Security Model

This document describes the intended privacy and security properties for the
LP-0005 implementation.

## What The Proof Shows

A valid proof shows:

```text
There exists a private LEZ account in the committed state tree
whose balance is greater than or equal to the public threshold,
and the proof is bound to a specific context and presenter.
```

## Hidden Values

The proof must not reveal:

- private account id
- nullifier public key (`npk`)
- exact balance
- nonce
- account data
- Merkle proof path siblings as private witness data, except what is required
  by the receipt public journal
- the private commitment leaf, unless a future verifier path explicitly proves
  it is safe to publish

## Public Values

The verifier learns:

- threshold
- context id
- verifier/gate id or its hash
- commitment root
- context nullifier
- presenter id
- circuit image id
- whether the proof verifies

The presenter id is intentionally public because it is the identity allowed to
use the proof. It is not the private token account.

## Context Binding

Replay across gates is prevented by deriving `context_id` from gate-specific
public data:

```text
context_id = SHA256(
  "logos-balance-attestation/v1/context"
  || chain_id
  || circuit_image_id
  || verifier_id
  || gate_id
  || threshold
)
```

A proof for one context must fail in another context.

## Context Nullifier

The context nullifier prevents duplicate use inside the same context while
avoiding cross-context linkability:

```text
context_nullifier = SHA256(
  "logos-balance-attestation/v1/nullifier"
  || npk
  || context_id
  || presenter_id
)
```

The domain prefix is mandatory. It separates this nullifier from LEZ spend
nullifiers and from any other program that may hash `npk` into public values.
Reusing a nullifier domain across protocols would create silent linkability or,
in the worst case, collision hazards across applications.

Expected behavior:

- Same account, same presenter, same context -> same nullifier.
- Same account, different context -> different nullifier.
- Same account, different presenter -> different nullifier.

The nullifier should be public, but it should not reveal `npk` without knowing
the private preimage.

## Presenter Binding

The implementation binds the proof to the presenter via BIP-340 Schnorr over
secp256k1. The presenter holds a 32-byte secret; its public counterpart is a
32-byte x-only Schnorr public key.

```text
presenter_id = SHA256(
  "logos-balance-attestation/v1/presenter"
  || presenter_pubkey
)
```

The journal commits `presenter_id`. The envelope additionally carries the raw
`presenter_pubkey` (32 bytes) and a `presenter_signature` (64-byte BIP-340
signature over `journal.digest()`). The context nullifier includes
`presenter_id`, so the same private account produces different nullifiers per
presenter.

The circuit only hashes the pubkey (no in-circuit ECC). Knowledge-of-secret is
proved off-circuit by the BIP-340 signature: only the secret-holder can produce
a signature that verifies under the pubkey committed in the journal.

The full design uses these mechanisms:

1. The proof journal includes `presenter_id = H(presenter_pubkey)`.
2. The envelope carries the raw pubkey + a Schnorr signature over the journal.
3. Off-chain verifiers (`attestation_verifier::verify_envelope`) check both
   `H(pubkey) == journal.presenter_id` and the Schnorr signature over
   `journal.digest()`.

For off-chain verification:

```text
signature = BIP340_Schnorr_Sign(presenter_secret, journal.digest())
verify_envelope:
  - H(envelope.presenter_pubkey) == envelope.journal.presenter_id
  - BIP340_Schnorr_Verify(envelope.presenter_pubkey, journal.digest(), signature)
```

A passive third party who captures the envelope cannot construct a new valid
envelope because they don't have the secret. A challenge-response layer
(verifier nonce signed alongside the journal digest) is a future hardening
the on-wire format leaves room for; V1 already prevents passive replay because
the signature is bound to the journal.

For on-chain verification (Spike 0C path):

- the LEZ tx is signed by the presenter account
- the LEZ program asserts `H(presenter_account) == journal.presenter_id`
- the on-chain path does NOT need the off-chain Schnorr signature because
  LEZ tx signing already enforces presenter identity

This is the resolution of the open decision flagged in earlier drafts of this
doc ("map presenter_secret to a wallet-compatible signing key or keep the
in-circuit proof"). The chosen design is the cheap path: presenter_id binds to
a Schnorr pubkey hashed in-circuit, knowledge-of-secret is proved by the
signature, and no in-circuit ECC is required.

## Remaining Forwarding Limitations

No proof system can stop voluntary collusion by itself. Alice can still:

- generate a proof for Bob's presenter id
- share her presenter private key
- act online as a signing service for Bob

The design prevents passive replay and accidental forwarding, not intentional
credential sharing.

If evaluator feedback requires the presenter secret to be committed inside the
RISC Zero receipt itself, Spike 04 already validates that circuit shape. The
remaining work is mapping it to the real wallet/presenter key material.

## Threshold Privacy Limits

A single proof reveals only whether `balance >= N`.

Repeated challenges with different thresholds can narrow the possible balance
range. Applications that care about this should use coarse threshold tiers and
avoid asking users to prove many nearby thresholds.

Reference integrations should use coarse hardcoded tiers, for example:

```text
100, 1000, 10000
```

Avoid arbitrary per-user thresholds in demos; they make repeated-query balance
inference easier.

## Stale State

A proof is valid for a specific commitment root. If an account later spends
funds, an old proof might still verify unless the verifier enforces freshness.

Mitigations:

- include chain id and verifier id in context
- include a recent root or block id policy in the verifier
- for on-chain gates, check against accepted roots supported by the sequencer or
  verifier program
- for off-chain gates, require short-lived challenges

Root freshness policy for V1:

- off-chain verification uses short-lived verifier challenges
- on-chain gates store a small accepted-roots ring buffer or equivalent
  freshness policy once LEZ exposes the practical root source

This must be finalized before the LEZ verifier program milestone, not during
submission hardening.

## Logging Policy

The CLI, Basecamp backend, and tests must not log:

- `npk`
- nullifier secret key
- viewing keys
- raw private account data
- exact private balance, except in explicitly marked local debugging commands

Normal success output should show proof ids, context ids, presenter ids, and
verification status only.

## Trust Assumptions

The system assumes:

- the local wallet storage accurately represents the user's private account
  state after `wallet account sync-private`
- the sequencer returns a correct membership proof for a submitted commitment
- the verifier uses the expected RISC Zero image id
- the presenter private key remains secret
- RISC Zero receipt verification is sound
