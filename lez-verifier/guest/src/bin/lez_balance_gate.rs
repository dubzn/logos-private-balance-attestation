// LEZ on-chain gate guest (recursion outer program).
//
// Wraps a balance-attestation receipt as an assumption, calls env::verify to
// nest it, then re-derives the gate's context_id from the verifier-pinned
// (chain_id, inner_image_id, verifier_id, gate_id, threshold) tuple and asserts
// that the inner journal matches. The outer journal commits a small public
// summary that the LEZ on-chain program can dedup on its (gate_context_id,
// nullifier) state.
//
// Field order in derive_context_id MUST mirror attestation_core::hash and the
// inner balance-attestation circuit byte-for-byte:
// crates/attestation-core/src/hash.rs:9-18
// methods/guest/src/bin/balance_attestation.rs

#![no_main]

use risc0_zkvm::{
    guest::env,
    sha::{Impl, Sha256 as _},
    Digest,
};
use serde::{Deserialize, Serialize};

risc0_zkvm::guest::entry!(main);

const CONTEXT_DOMAIN: &[u8] = b"logos-balance-attestation/v1/context";
const LEZ_GATE_JOURNAL_DOMAIN: &[u8] = b"logos-balance-attestation/v1/lez-gate-journal";
const LEZ_GATE_JOURNAL_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
struct LezGateInput {
    /// Inner image id — the BALANCE_ATTESTATION_ID that the inner receipt was
    /// produced under. The outer receipt commits this value; the LEZ program
    /// MUST reject any outer receipt whose committed `inner_image_id` is not
    /// the pinned BALANCE_ATTESTATION_ID.
    inner_image_id: [u8; 32],
    /// Bytes of the inner receipt's committed journal (canonically encoded by
    /// the inner guest via env::commit). We decode them here AND pass them to
    /// env::verify so the assumption resolves.
    inner_journal_bytes: Vec<u8>,
    /// Verifier-pinned gate parameters. The outer guest re-derives context_id
    /// from these and refuses to admit if the inner journal disagrees.
    expected_chain_id: [u8; 32],
    expected_verifier_id: [u8; 32],
    expected_gate_id: [u8; 32],
    expected_threshold: u128,
}

// Mirror of attestation_core::BalanceAttestationJournal — kept inline so the
// outer guest doesn't need a workspace dependency on attestation-core.
#[derive(Serialize, Deserialize)]
struct InnerJournal {
    version: u16,
    threshold: u128,
    commitment_root: [u8; 32],
    context_id: [u8; 32],
    context_nullifier: [u8; 32],
    presenter_id: [u8; 32],
    verifier_id: [u8; 32],
    circuit_image_id: [u8; 32],
    proof_index: u64,
    proof_depth: u64,
}

#[derive(Serialize, Deserialize)]
struct LezGateJournal {
    version: u16,
    /// Echo of the inner image id — LEZ program checks this matches the pinned
    /// BALANCE_ATTESTATION_ID before trusting the gate decision.
    inner_image_id: [u8; 32],
    /// Re-derived from the verifier-pinned gate params; the LEZ program uses
    /// this as the on-chain key for the dedup map.
    gate_context_id: [u8; 32],
    /// What gets inserted into the (gate_context_id → set<nullifier>) map.
    accepted_context_nullifier: [u8; 32],
    /// Bound to the LEZ presenter account (per ARCHITECTURE.md off-chain vs
    /// on-chain split: on-chain authorization is via LEZ tx signing against a
    /// presenter account whose hash matches presenter_id).
    accepted_presenter_id: [u8; 32],
    /// Echoed so the LEZ program can confirm it matches the gate's threshold.
    accepted_threshold: u128,
}

pub fn main() {
    let input: LezGateInput = env::read();

    // 1. Recursion: verify the inner receipt as an assumption. In dev mode this
    //    is a no-op; under real proving the inner receipt must have been added
    //    via host_env_builder.add_assumption(inner_receipt) before proving.
    env::verify(Digest::from(input.inner_image_id), &input.inner_journal_bytes)
        .expect("inner balance-attestation receipt must verify");

    // 2. Decode the inner journal bytes (same byte format the inner guest used).
    let inner: InnerJournal = risc0_zkvm::serde::from_slice(&input.inner_journal_bytes)
        .expect("inner journal bytes must decode");

    // 3. The inner journal claims a circuit_image_id; defense in depth — assert
    //    it matches what the host told us.
    assert_eq!(
        inner.circuit_image_id, input.inner_image_id,
        "inner journal circuit_image_id must match the assumption image_id"
    );

    // 4. Recompute the expected context_id from the verifier-pinned gate params,
    //    using the SAME field order as attestation_core::derive_context_id.
    let expected_context_id = derive_context_id(
        &input.expected_chain_id,
        &input.inner_image_id,
        &input.expected_verifier_id,
        &input.expected_gate_id,
        input.expected_threshold,
    );
    assert_eq!(
        inner.context_id, expected_context_id,
        "inner journal context_id does not match verifier-pinned gate params"
    );

    // 5. Threshold check. V1 binds the threshold into context_id, so this gate
    // accepts the exact threshold it pinned.
    assert_eq!(
        inner.threshold, input.expected_threshold,
        "inner journal threshold does not match verifier-pinned gate"
    );

    // 6. Verifier_id sanity: the inner journal's verifier_id must match the
    //    one we pinned (covered transitively by context_id but kept explicit
    //    so the failure surface is informative).
    assert_eq!(
        inner.verifier_id, input.expected_verifier_id,
        "inner journal verifier_id does not match pinned verifier"
    );

    env::commit(&LezGateJournal {
        version: LEZ_GATE_JOURNAL_VERSION,
        inner_image_id: input.inner_image_id,
        gate_context_id: expected_context_id,
        accepted_context_nullifier: inner.context_nullifier,
        accepted_presenter_id: inner.presenter_id,
        accepted_threshold: inner.threshold,
    });

    // Touch the unused domain constant so the linker keeps it around for any
    // future `journal_digest` introspection from the host side.
    let _ = LEZ_GATE_JOURNAL_DOMAIN;
}

fn derive_context_id(
    chain_id: &[u8; 32],
    circuit_image_id: &[u8; 32],
    verifier_id: &[u8; 32],
    gate_id: &[u8; 32],
    threshold: u128,
) -> [u8; 32] {
    hash_segments(&[
        CONTEXT_DOMAIN,
        chain_id,
        circuit_image_id,
        verifier_id,
        gate_id,
        &threshold.to_le_bytes(),
    ])
}

fn hash_segments(segments: &[&[u8]]) -> [u8; 32] {
    let mut bytes = Vec::new();
    for segment in segments {
        bytes.extend_from_slice(&(segment.len() as u64).to_le_bytes());
        bytes.extend_from_slice(segment);
    }
    sha256(&bytes)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Impl::hash_bytes(bytes).as_bytes().try_into().unwrap()
}
