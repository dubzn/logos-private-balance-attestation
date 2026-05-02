# Reference Notes: LP-0016 PR #465 (syafiqeil:main)

**Date read:** 2026-05-02  
**PR:** https://github.com/logos-blockchain/logos-execution-zone/pull/465  
**Author:** syafiqeil  
**Head branch:** syafiqeil:main — HEAD `8fbf9ebcf18306b61c67b20c1ce938eadbaf3b96`  
**Base:** logos-blockchain:main  
**Status:** Open  
**Local clone:** `logos/` (gitignored, not committed to this repo)  
**Prize:** LP-0016 (Anonymous Forum with Threshold Moderation) — NOT LP-0005  
**Why it matters:** shares the exact same LEZ stack and RISC Zero version, and ships several artifacts that LP-0005 still needs

---

## 1. File-by-file relevance map

| Path in clone | What it does | LP-0005 applicability | Copy risk |
|---|---|---|---|
| `program_methods/guest/src/bin/forum_membership_proof.rs` | RISC Zero guest: derives commitment from NSK, verifies Merkle membership, outputs tracing-tag | Pattern for circuit structure, input/output split, `Commitment::new` + `compute_digest_for_path` usage | Low — similar to our Spike 03, ours already more advanced |
| `programs/membership_registry/src/lib.rs` | Program library: `MembershipInstruction` enum, `VerifyPost { zk_receipt }` arm calls `zk_receipt.verify(FORUM_MEMBERSHIP_PROOF_ID)` | Shows exact host-side receipt verification API; NOT a deployed LEZ on-chain program (see §2.1) | Low — pattern only |
| `programs/membership_registry/src/state.rs` | `ForumInstance` struct with `nssa::merkle_tree::MerkleTree` for nullifier/registry state | Template for LEZ program state that embeds a Merkle tree (applicable to our nullifier registry) | Low — adapt shape |
| `programs/membership_registry/src/register.rs` | Inserts `nssa_core::Commitment` into forum registry tree | Shows how to use `MerkleTree::insert` with real LEZ commitments | Low |
| `programs/membership_registry/src/slash.rs` | Revokes a commitment via NSK reconstruction and `revoked_commitments` list | Pattern for on-chain nullifier/blacklist management | Low |
| `programs/membership_registry/Cargo.toml` | Deps: `nssa_core`, `nssa`, `risc0-zkvm`, `borsh`, `serde`; `crate-type = ["cdylib", "rlib"]` | Confirms the exact dep set needed for a LEZ-compatible program crate | Direct reference |
| `nssa/src/lib.rs` | Re-exports `pub mod merkle_tree` (was private before this PR) | `nssa::merkle_tree::MerkleTree` is now public; our verifier program can use it for nullifier state | Note API availability |
| `nssa/src/merkle_tree/mod.rs` | Full `MerkleTree` with `insert`, `root`, `get_authentication_path_for` (SHA-256 internal hashing, BorshSerialize) | Usable in our LEZ verifier program state. Note: different tree than the LEZ token commitment tree — syafiqeil's forum has its OWN registry, while LP-0005 reads from the sequencer-managed token tree | Context matters |
| `nssa/core/src/lib.rs` | Unchanged public API: `Commitment`, `compute_digest_for_path`, `NullifierPublicKey`, `NullifierSecretKey`, `Nullifier` | No new hooks for LP-0005; confirms our attestation-core re-exports already match | Verify-only |
| `logos_moderation_sdk/src/wasm_bindings.rs` | `#[wasm_bindgen]` wrappers over `MemberClient`, `ModeratorClient`, `SlashAggregator`; uses `serde_wasm_bindgen::{to_value, from_value}` | **Direct template for LP-0005 `apps/basecamp/`**. Pattern: `WasmXClient { inner: XClient }`, methods return `Result<JsValue, JsValue>` | Adapt — LP-0005 needs different methods |
| `logos_moderation_sdk/src/crypto/signature/mod.rs` | BIP340 Schnorr via `k256::schnorr`: `Signature::new(key, msg)` and `Signature::is_valid_for(bytes, pubkey)` | **Direct reference for presenter binding** in LP-0005 envelope — Schnorr sign/verify over `journal_hash || challenge` | High value — adapt API |
| `logos_moderation_sdk/src/crypto/signature/private_key.rs` (implied) | `PrivateKey` wrapper over 32-byte scalar, exposed to WASM | Template for `PresenterKey` type in our SDK | Adapt |
| `logos_moderation_sdk/src/clients/member.rs` | `MemberClient` owns NSK, maintains SSS share list, generates `PostPayload` with tracing-tag and encrypted shares | LP-0016 specific — SSS not needed for LP-0005. BUT the `generate_tracing_tag(nsk, msg_hash, salt)` sub-function is useful reference for post-binding (not context-binding) | Low |
| `logos_moderation_sdk/src/types.rs` | `PostPayload`, `EncryptedSharePerPost`, `ModerationCertificate` | LP-0016 specific | Low |
| `integration_tests/tests/forum.rs` | Full E2E: generate ZK receipt with `default_prover()` + `ExecutorEnv`, call `process_instruction` directly, assert state | **Template for our E2E integration test** without a live sequencer. Shows how to compose prover + verifier in a single test | Adapt |
| `logos/Cargo.toml` (root workspace) | `risc0-zkvm = "3.0.5"`, `risc0-build = "3.0.5"` | Confirms version is identical to our `check-risc0-version.sh` expectation. No drift. | Verify-only |

---

## 2. Implications for open LP-0005 decisions

### 2.1 Spike 0A / ONCHAIN_PATH_DECISION.md — finding NOT contradicted

The `membership_registry` program calls `zk_receipt.verify(FORUM_MEMBERSHIP_PROOF_ID)` at
`logos/programs/membership_registry/src/lib.rs:39`. This is the **host-side** `Receipt::verify()`
from `risc0-zkvm`. In the integration test (`logos/integration_tests/tests/forum.rs:97-99`),
`process_instruction` is called directly in regular Rust — not through a sequencer transaction.

There is no LEZ runner binary that compiles this library into a LEZ ELF and submits it as a
transaction. The `[[bin]]` entry in `membership_registry/Cargo.toml` points to
`src/bin/forum_membership_proof.rs` which does not exist in that directory. The actual circuit ELF
lives in `program_methods/guest/src/bin/forum_membership_proof.rs` and is a separate program.

**Conclusion**: syafiqeil's submission does not deploy a public LEZ program that accepts real
transactions and verifies a receipt inside the guest. The host-side `receipt.verify()` works in
plain Rust tests. The Spike 0A error (`sys_verify_integrity: no receipt found to resolve
assumption`) occurs only when `env::verify` is called from inside a RISC Zero guest — which is
what a deployed public LEZ program would do.

`docs/ONCHAIN_PATH_DECISION.md` stands. The two-track architecture is still the right approach:
off-chain proof envelope + Logos-native private execution for the on-chain gate.

One thing this does confirm: if LP-0016 evaluators accept a simulation-based "on-chain" test
(process_instruction called directly, not via a real transaction), then LP-0005 evaluators might
accept the same. This should be discussed with evaluators before investing in a full LEZ deployment.

### 2.2 Spike 0B — still unexecuted, still needs a decision

Syafiqeil provides no evidence for or against recursive/native RISC Zero verifier support in LEZ.
P5 from FEEDBACK_LP-0005.md is unchanged: either run Spike 0B or formally bury it. Recommendation:
bury it as "not found in inspected LEZ checkout" matching the Spike 06 inspection date of 2026-05-02
and update RISK_SPIKES.md accordingly.

### 2.3 Presenter binding (P1) — concrete reference found

Syafiqeil uses `k256::schnorr::SigningKey::from_bytes(key).sign_raw(msg, aux_random)` and
`k256::schnorr::VerifyingKey::from_bytes(pk).verify_raw(bytes, sig)` at
`logos/logos_moderation_sdk/src/crypto/signature/mod.rs:25-48`.

For LP-0005, the recommended approach is:
1. Inside the circuit (production circuit, not Spike 04's synthetic secret): commit a `presenter_pk`
   (32-byte x-only BIP340 public key) to the journal.
2. Outside the circuit (SDK envelope layer): the presenter Schnorr-signs
   `SHA256("logos-balance-attestation/v1/present" || journal_hash || verifier_id)` with the private
   key corresponding to `presenter_pk`.
3. The verifier (on-chain or off-chain) checks the Schnorr signature over that message.

This maps `presenter_pk` to the wallet's spending key or a derived key — the exact key derivation
is the remaining open decision documented in `docs/SECURITY_MODEL.md:124-127`.

Add `k256 = { version = "0.13", features = ["schnorr"] }` to the attestation-core workspace
dependency. No need to copy syafiqeil's module.

### 2.4 Nullifier scheme (P2) — no new evidence

Syafiqeil does not use a context nullifier. The `tracing_tag = SHA256(NSK || H(M) || salt)` at
`logos/logos_moderation_sdk/src/clients/member.rs:80-85` is a per-post tracker for retroactive
deanonymization — a completely different goal from LP-0005's anti-replay nullifier.

The `nssa_core::Nullifier` at `logos/nssa/core/src/nullifier.rs:78-88` is internal to LEZ's
private transaction accounting, not usable as a public LP-0005 nullifier.

Our Spike 04 design stands: `context_nullifier = SHA256("logos-balance-attestation/v1/nullifier" ||
npk || context_id)`, committed in the journal and checked by the verifier. No change.

### 2.5 Context binding (P3) — no new evidence

Syafiqeil's circuit has no context binding. The `message_hash` and `post_salt` are public inputs
to the circuit but they bind to the post content, not to a gate/verifier identity.

Our attestation-core canonical formula (from REFERENCE_NOTES.md M1 spike result) stands:
`context_id = SHA256("logos-balance-attestation/v1/context" || chain_id || circuit_image_id ||
verifier_id || gate_id || threshold)`.

The circuit must assert that `context_id` in its journal was derived with this formula over the
inputs — not passthrough. This is what separates M1 production circuit from Spike 03.

### 2.6 WASM / Basecamp app — concrete template found

`logos/logos_moderation_sdk/src/wasm_bindings.rs` is a working, battle-tested template for wiring
a Rust SDK to a browser Basecamp app via `wasm_bindgen`. The pattern:

```rust
#[wasm_bindgen]
pub struct WasmAttestationClient { inner: AttestationClient }

#[wasm_bindgen]
impl WasmAttestationClient {
    #[wasm_bindgen(constructor)]
    pub fn new(nsk_js: &[u8]) -> Result<WasmAttestationClient, JsValue> { ... }

    pub fn generate_proof_wasm(...) -> Result<JsValue, JsValue> { ... }
}
```

When LP-0005 reaches the Basecamp milestone, model `apps/basecamp/sdk/` on this file. Add
workspace deps: `wasm-bindgen = { version = "0.2" }` and
`serde-wasm-bindgen = { version = "0.6" }`.

### 2.7 SPEL/IDL — no new evidence

Syafiqeil does not use SPEL for LP-0016. This remains an open item for LP-0005. The SPEL
requirement is in the LP-0005 success criteria (Usability section).

---

## 3. Action items for LP-0005 (ordered by dependency)

1. **Bury Spike 0B formally** — Update `docs/RISK_SPIKES.md` to add a "spike status" summary row
   at the top: 0A=failed, 0B=not-found-in-LEZ-checkout-2026-05-02, 0C=pass, 02=pass, 03=pass,
   04=pass, 05=pass, 06=decision. This closes P5 from FEEDBACK_LP-0005.md.

2. **Rewrite IDL_DRAFT.md `claim_access` for the 0C model** — Under the current two-track
   architecture, `claim_access` is NOT a public program instruction that receives a RISC Zero
   receipt. It is a private execution gate: the prover submits a private transaction that internally
   validates `balance >= threshold` and writes a marker to a public gate account. The IDL should
   reflect this — the public-facing interface is the gate-marker account state, not a receipt
   argument. This closes P4 from FEEDBACK_LP-0005.md. Also: `init_gate` should remove the
   `BA502 UnauthorizedPresenterAccount` error code (P7).

3. **Add `k256` Schnorr dep to attestation-core Cargo.toml** — and add a `PresenterBinding`
   module with `sign_presentation(journal_hash, verifier_id, presenter_sk)` and
   `verify_presentation(journal_hash, verifier_id, presenter_pk, sig)` functions. Closes P1.

4. **Update context_id formula in ARCHITECTURE.md and SECURITY_MODEL.md to match** — current
   divergence between the two files (P6 from FEEDBACK) needs to resolve to the M1 canonical
   formula from REFERENCE_NOTES.md (SHA256 of domain + chain_id + image_id + verifier_id +
   gate_id + threshold). One file needs to update.

5. **Decide `commitment` leaf in journal vs root+nullifier** — P9 from FEEDBACK. The Spike 03
   journal publishes the raw commitment leaf (linkable to on-chain tree by any observer). Spike 04
   already removes the leaf. The production circuit should follow Spike 04: publish only
   `context_nullifier` and `presenter_pk` (or `presenter_id`) in the journal — no raw commitment.
   Write this decision into `docs/SECURITY_MODEL.md` as a closed decision.

6. **Use `nssa::merkle_tree::MerkleTree` for nullifier state in LEZ verifier program** — when
   building `lez/verifier-program/`, use `ForumInstance`-style state struct with an embedded
   `MerkleTree` for tracking spent nullifiers. This prevents replay of the same attestation to the
   same gate.

7. **Follow syafiqeil's E2E test shape** for `examples/governance-gate/` — standalone test that
   calls the program logic directly (no sequencer), using `default_prover()` + `ExecutorEnv`. This
   provides fast CI coverage before the full Spike 01-style sequencer E2E.

8. **Clarify with evaluators** whether a simulation-based on-chain test (like syafiqeil's
   `process_instruction` called directly) satisfies LP-0005's "LEZ verifier program" requirement, or
   whether a fully deployed sequencer-transacted program is required. Syafiqeil's PR offers the
   first viable data point for this question.

---

## 4. What syafiqeil's submission does NOT cover (LP-0005 gaps remain)

- Balance threshold proof — syafiqeil's circuit uses `Account::default()` with no balance check.
- Context binding for gate-identity — no gate-specific context derivation.
- Context nullifier for anti-replay — no per-context nullifier published on-chain.
- Presenter binding in the LP-0005 sense — tracing_tag binds to post content, not to a wallet key.
- Deployed LEZ on-chain verifier program — `process_instruction` called host-side in tests.
- SPEL/IDL artifact.
- Off-chain verification over Logos Messaging.
- Basecamp GUI (though WASM binding pattern is directly applicable).
- CI on the LEZ-deployment and sequencer paths.

---

## 5. License and attribution note

The PR is open (not yet merged) on `logos-blockchain:main`. The parent repo carries the MIT
license (confirmed via `logos/LICENSE`). No code has been copied to this repo — this document
cites paths and short function signatures for reference only. Any adaptation of syafiqeil's
patterns into LP-0005 code should be done from scratch using the same public libraries (`k256`,
`wasm-bindgen`, etc.) and attribute the original design inspiration in the relevant commit message.
