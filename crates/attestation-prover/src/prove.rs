use std::fmt;

use attestation_core::{
    BalanceAttestationEnvelope, BalanceAttestationJournal, Digest32, JOURNAL_VERSION,
};
use methods::{BALANCE_ATTESTATION_ELF, BALANCE_ATTESTATION_ID};
use risc0_zkvm::{default_prover, Digest, ExecutorEnv};
use serde::{Deserialize, Serialize};

use crate::{AttestationPublicParams, BalanceAttestationWitness};

#[derive(Debug)]
pub struct ProveError(pub String);

impl fmt::Display for ProveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ProveError {}

// Guest input — must match BalanceAttestationInput in methods/guest/src/bin/balance_attestation.rs.
#[derive(Serialize, Deserialize)]
struct GuestInput {
    npk: [u8; 32],
    program_owner: [u32; 8],
    balance: u128,
    nonce: u128,
    data: Vec<u8>,
    proof_index: u64,
    proof_siblings: Vec<[u8; 32]>,
    threshold: u128,
    commitment_root: [u8; 32],
    chain_id: [u8; 32],
    verifier_id: [u8; 32],
    gate_id: [u8; 32],
    circuit_image_id: [u8; 32],
    presenter_pubkey: [u8; 32],
    presenter_id: [u8; 32],
    expected_context_nullifier: [u8; 32],
}

// Decode target — matches guest journal struct exactly, without serde_helpers wrappers.
#[derive(Deserialize)]
struct JournalDecoded {
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

/// The canonical image ID of the production circuit as [u8; 32].
pub fn balance_attestation_image_id() -> [u8; 32] {
    Digest::from(BALANCE_ATTESTATION_ID)
        .as_bytes()
        .try_into()
        .expect("Digest is always 32 bytes")
}

/// Prove a balance attestation and return a ready-to-transport envelope.
///
/// `params` must be the same params used to build `witness`; `chain_id` and `gate_id`
/// are not stored in the witness and are required to reconstruct the guest input.
///
/// `circuit_image_id` in the returned envelope is always `BALANCE_ATTESTATION_ID`
/// from the compiled `methods` crate — the caller's `witness.circuit_image_id` is ignored.
///
/// The envelope carries the presenter's BIP-340 x-only pubkey and a Schnorr signature
/// over a verifier/session challenge. Verifiers must check all three:
/// `H(pubkey) == journal.presenter_id`, `envelope.presentation_challenge`
/// matches the verifier challenge, and `Schnorr_verify(pubkey,
/// presentation_digest(journal.digest(), challenge), sig)`.
pub fn prove_attestation(
    witness: &BalanceAttestationWitness,
    params: &AttestationPublicParams,
    presentation_challenge: Digest32,
) -> Result<BalanceAttestationEnvelope, ProveError> {
    // Always use the compiled image ID, not whatever the witness says.
    let circuit_image_id = balance_attestation_image_id();
    let presenter_pubkey = witness.presenter.pubkey();

    let input = GuestInput {
        npk: witness.private_account.npk.0,
        program_owner: witness.private_account.program_owner,
        balance: witness.private_account.balance,
        nonce: witness.private_account.nonce,
        data: witness.private_account.data.as_bytes().to_vec(),
        proof_index: witness.membership_proof.index,
        proof_siblings: witness
            .membership_proof
            .siblings
            .iter()
            .map(|d| d.0)
            .collect(),
        threshold: witness.threshold,
        commitment_root: witness.commitment_root.0,
        chain_id: params.chain_id.0,
        verifier_id: witness.verifier_id.0,
        gate_id: params.gate_id.0,
        circuit_image_id,
        presenter_pubkey: *presenter_pubkey.as_bytes(),
        presenter_id: witness.presenter_id.0,
        expected_context_nullifier: witness.context_nullifier.0,
    };

    let env = ExecutorEnv::builder()
        .write(&input)
        .map_err(|e| ProveError(format!("input write failed: {e}")))?
        .build()
        .map_err(|e| ProveError(format!("executor env build failed: {e}")))?;

    let prove_info = default_prover()
        .prove(env, BALANCE_ATTESTATION_ELF)
        .map_err(|e| ProveError(e.to_string()))?;

    prove_info
        .receipt
        .verify(Digest::from(BALANCE_ATTESTATION_ID))
        .map_err(|e| ProveError(format!("receipt verification failed: {e}")))?;

    let j: JournalDecoded = prove_info
        .receipt
        .journal
        .decode()
        .map_err(|e| ProveError(format!("journal decode failed: {e}")))?;

    if j.version != JOURNAL_VERSION {
        return Err(ProveError(format!(
            "unexpected journal version {}, expected {JOURNAL_VERSION}",
            j.version
        )));
    }

    let journal = BalanceAttestationJournal::new(
        j.threshold,
        Digest32(j.commitment_root),
        Digest32(j.context_id),
        Digest32(j.context_nullifier),
        Digest32(j.presenter_id),
        Digest32(j.verifier_id),
        Digest32(j.circuit_image_id),
        j.proof_index,
        j.proof_depth,
    );

    // Receipt bytes stored as serde_json for transport; verifier deserializes with
    // serde_json::from_slice::<risc0_zkvm::Receipt>(&bytes).verify(BALANCE_ATTESTATION_ID).
    let receipt_bytes = serde_json::to_vec(&prove_info.receipt)
        .map_err(|e| ProveError(format!("receipt serialization failed: {e}")))?;

    let signature = witness
        .presenter
        .presenter_secret
        .sign_presentation(&journal.digest(), &presentation_challenge);

    Ok(BalanceAttestationEnvelope::new_risc0(
        journal,
        receipt_bytes,
        presenter_pubkey.as_bytes().to_vec(),
        presentation_challenge,
        signature.as_bytes().to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_balance_attestation_witness, PresenterWitness, PrivateAccountWitness};
    use attestation_core::{HexBytes, LezMembershipProof, PresenterSecret};

    fn digest(seed: u8) -> Digest32 {
        Digest32([seed; 32])
    }

    fn challenge() -> Digest32 {
        Digest32([0x44; 32])
    }

    fn fixture() -> (BalanceAttestationWitness, AttestationPublicParams) {
        // circuit_image_id must match the compiled circuit so the guest's context_id
        // derivation produces the same context_nullifier stored in the witness.
        let params = AttestationPublicParams {
            threshold: 25,
            chain_id: digest(0x10),
            verifier_id: digest(0x20),
            gate_id: digest(0x30),
            circuit_image_id: Digest32(balance_attestation_image_id()),
        };
        let witness = build_balance_attestation_witness(
            PrivateAccountWitness {
                npk: digest(0x07),
                program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
                balance: 42,
                nonce: 123_456,
                data: HexBytes::new(b"witness fixture".to_vec()),
            },
            LezMembershipProof {
                index: 5,
                siblings: vec![digest(0x11), digest(0x22), digest(0x33), digest(0x44)],
            },
            PresenterWitness {
                presenter_secret: PresenterSecret::new([0x77; 32]).unwrap(),
            },
            params,
        );
        (witness, params)
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1; run with: RISC0_DEV_MODE=1 cargo test -p attestation-prover -- --include-ignored"]
    fn prove_attestation_returns_valid_envelope() {
        std::env::set_var("RISC0_DEV_MODE", "1");

        let (witness, params) = fixture();
        let envelope = prove_attestation(&witness, &params, challenge())
            .expect("prove_attestation should succeed");

        envelope
            .validate_shape()
            .expect("envelope shape should be valid");
        assert_eq!(envelope.journal.version, JOURNAL_VERSION);
        assert_eq!(envelope.journal.threshold, witness.threshold);
        assert_eq!(envelope.journal.commitment_root, witness.commitment_root);
        assert_eq!(envelope.journal.context_id, witness.context_id);
        assert_eq!(
            envelope.journal.context_nullifier,
            witness.context_nullifier
        );
        assert_eq!(envelope.journal.presenter_id, witness.presenter_id);
        assert_eq!(envelope.presentation_challenge, challenge());
        // circuit_image_id must be the compiled ID, not the fixture's witness value.
        assert_eq!(
            envelope.journal.circuit_image_id,
            Digest32(balance_attestation_image_id())
        );
        assert_eq!(envelope.journal.proof_index, witness.membership_proof.index);
        assert_eq!(
            envelope.journal.proof_depth,
            witness.membership_proof.siblings.len() as u64
        );

        // Receipt bytes round-trip.
        let receipt: risc0_zkvm::Receipt = serde_json::from_slice(envelope.receipt.as_bytes())
            .expect("receipt should deserialize");
        receipt
            .verify(Digest::from(BALANCE_ATTESTATION_ID))
            .expect("deserialized receipt should verify");

        // Presenter binding: pubkey hashes to journal.presenter_id and signature verifies.
        let pubkey =
            attestation_core::PresenterPubkey::from_slice(envelope.presenter_pubkey.as_bytes())
                .expect("envelope pubkey should be valid");
        assert_eq!(pubkey.presenter_id(), envelope.journal.presenter_id);

        let sig = attestation_core::PresenterSignature::from_slice(
            envelope.presenter_signature.as_bytes(),
        )
        .expect("envelope signature should be 64 bytes");
        pubkey
            .verify_presentation(
                &envelope.journal.digest(),
                &envelope.presentation_challenge,
                &sig,
            )
            .expect("presenter signature should verify against journal digest");
    }
}
