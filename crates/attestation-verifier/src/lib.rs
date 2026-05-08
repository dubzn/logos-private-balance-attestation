//! Off-chain verification for balance-attestation envelopes.
//!
//! `verify_envelope` performs all checks required to admit a presenter to a gate:
//! receipt verification against the pinned RISC Zero image id, journal shape,
//! context binding, exact threshold check, challenge binding, presenter pubkey
//! ↔ id, and BIP-340 signature over the presentation digest.
//!
//! On-chain verifiers and reference integrations should call this same function
//! (or duplicate its checks deterministically) so failure modes match.

use std::fmt;

use attestation_core::{
    AttestationError, AttestationErrorCode, BalanceAttestationEnvelope, Digest32, PresenterPubkey,
    PresenterSignature, ENVELOPE_VERSION, JOURNAL_VERSION,
};
use methods::BALANCE_ATTESTATION_ID;
use risc0_zkvm::{Digest, Receipt};

#[derive(Debug)]
pub enum VerifyError {
    /// Wrapper for the structured shape errors from `validate_shape()`.
    Shape(AttestationError),
    /// Receipt JSON could not be deserialized.
    MalformedReceipt(String),
    /// Receipt failed `Receipt::verify(BALANCE_ATTESTATION_ID)`.
    InvalidReceipt(String),
    /// Receipt's image_id (in receipt metadata) does not match the journal's image_id.
    ImageIdMismatch,
    /// Receipt's journal bytes don't decode to the expected struct OR don't match the envelope's journal field.
    JournalBytesMismatch(String),
    /// `journal.context_id` does not match the verifier's expected context.
    ContextMismatch {
        expected: Digest32,
        actual: Digest32,
    },
    /// `journal.threshold` does not match the context-bound gate threshold.
    ThresholdMismatch { expected: u128, journal: u128 },
    /// `envelope.presentation_challenge` does not match the verifier's session challenge.
    ChallengeMismatch {
        expected: Digest32,
        actual: Digest32,
    },
    /// `H(envelope.presenter_pubkey) != journal.presenter_id`.
    PresenterMismatch,
    /// Pubkey bytes were not a valid BIP-340 x-only Schnorr key.
    InvalidPresenterPubkey,
    /// Signature bytes were not a valid 64-byte Schnorr signature OR did not verify.
    InvalidPresenterSignature,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape(e) => write!(f, "envelope shape invalid: {e}"),
            Self::MalformedReceipt(d) => write!(f, "receipt deserialize failed: {d}"),
            Self::InvalidReceipt(d) => write!(f, "receipt verify failed: {d}"),
            Self::ImageIdMismatch => {
                f.write_str("envelope image_id does not match compiled BALANCE_ATTESTATION_ID")
            }
            Self::JournalBytesMismatch(d) => {
                write!(f, "receipt journal does not match envelope.journal: {d}")
            }
            Self::ContextMismatch { expected, actual } => write!(
                f,
                "context_id mismatch: expected {}, got {}",
                expected.to_hex(),
                actual.to_hex()
            ),
            Self::ThresholdMismatch { expected, journal } => {
                write!(
                    f,
                    "threshold mismatch: expected context-bound {expected}, journal commits {journal}"
                )
            }
            Self::ChallengeMismatch { expected, actual } => write!(
                f,
                "presentation challenge mismatch: expected {}, got {}",
                expected.to_hex(),
                actual.to_hex()
            ),
            Self::PresenterMismatch => {
                f.write_str("presenter pubkey does not hash to journal.presenter_id")
            }
            Self::InvalidPresenterPubkey => {
                f.write_str("presenter_pubkey is not a valid Schnorr x-only pubkey")
            }
            Self::InvalidPresenterSignature => {
                f.write_str("presenter signature failed Schnorr verification")
            }
        }
    }
}

impl std::error::Error for VerifyError {}

impl VerifyError {
    /// Map to a public-facing error code, suitable for off-chain UIs and
    /// (eventually) on-chain instruction return codes.
    pub fn code(&self) -> AttestationErrorCode {
        match self {
            Self::Shape(e) => e.code(),
            Self::MalformedReceipt(_) => AttestationErrorCode::MalformedEnvelope,
            Self::InvalidReceipt(_) => AttestationErrorCode::InvalidReceipt,
            Self::ImageIdMismatch => AttestationErrorCode::InvalidImageId,
            Self::JournalBytesMismatch(_) => AttestationErrorCode::MalformedJournal,
            Self::ContextMismatch { .. } => AttestationErrorCode::ContextMismatch,
            Self::ThresholdMismatch { .. } => AttestationErrorCode::ThresholdMismatch,
            Self::ChallengeMismatch { .. } => AttestationErrorCode::InvalidPresenterSignature,
            Self::PresenterMismatch => AttestationErrorCode::PresenterMismatch,
            Self::InvalidPresenterPubkey => AttestationErrorCode::PresenterMismatch,
            Self::InvalidPresenterSignature => AttestationErrorCode::InvalidPresenterSignature,
        }
    }
}

/// Verifier's expectation of which gate is being claimed and what exact
/// threshold is bound into that gate. The expected `context_id` is computed by the verifier from
/// its own `(chain_id, circuit_image_id, verifier_id, gate_id, threshold)` —
/// we do not trust the journal's `context_id` blindly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedGate {
    pub context_id: Digest32,
    pub threshold: u128,
    pub presentation_challenge: Digest32,
}

/// Verify a balance attestation envelope end-to-end.
///
/// On success, the caller may treat the envelope as a valid credential for
/// `expected.context_id` from the presenter identified by
/// `envelope.journal.presenter_id`. The caller is responsible for nullifier
/// uniqueness (storing `journal.context_nullifier` per `(context_id, presenter_id)`)
/// and for any application-level authorization beyond threshold + context.
pub fn verify_envelope(
    envelope: &BalanceAttestationEnvelope,
    expected: &ExpectedGate,
) -> Result<(), VerifyError> {
    // 1. Shape: version, proof_system, image_id internal consistency, lengths.
    envelope.validate_shape().map_err(VerifyError::Shape)?;

    // Defensive: validate_shape already covers these but we keep them explicit
    // so the error surface here is self-contained.
    debug_assert_eq!(envelope.version, ENVELOPE_VERSION);
    debug_assert_eq!(envelope.journal.version, JOURNAL_VERSION);

    // 2. image_id must match the compiled BALANCE_ATTESTATION_ID.
    let compiled_image_id: [u8; 32] = Digest::from(BALANCE_ATTESTATION_ID)
        .as_bytes()
        .try_into()
        .expect("Digest is 32 bytes");
    if envelope.image_id.0 != compiled_image_id {
        return Err(VerifyError::ImageIdMismatch);
    }

    // 3. Receipt deserialize + verify.
    let receipt: Receipt = serde_json::from_slice(envelope.receipt.as_bytes())
        .map_err(|e| VerifyError::MalformedReceipt(e.to_string()))?;
    receipt
        .verify(Digest::from(BALANCE_ATTESTATION_ID))
        .map_err(|e| VerifyError::InvalidReceipt(e.to_string()))?;

    // 4. Receipt journal bytes must match envelope.journal byte-for-byte
    //    (after re-encoding via the same canonical serializer the guest used).
    //    We decode into the same struct shape and compare fields.
    let decoded: JournalDecoded = receipt
        .journal
        .decode()
        .map_err(|e| VerifyError::JournalBytesMismatch(e.to_string()))?;
    if decoded.version != envelope.journal.version
        || decoded.threshold != envelope.journal.threshold
        || decoded.commitment_root != envelope.journal.commitment_root.0
        || decoded.context_id != envelope.journal.context_id.0
        || decoded.context_nullifier != envelope.journal.context_nullifier.0
        || decoded.presenter_id != envelope.journal.presenter_id.0
        || decoded.verifier_id != envelope.journal.verifier_id.0
        || decoded.circuit_image_id != envelope.journal.circuit_image_id.0
        || decoded.proof_index != envelope.journal.proof_index
        || decoded.proof_depth != envelope.journal.proof_depth
    {
        return Err(VerifyError::JournalBytesMismatch(
            "decoded receipt journal differs from envelope.journal".to_string(),
        ));
    }

    // 5. Context binding.
    if envelope.journal.context_id != expected.context_id {
        return Err(VerifyError::ContextMismatch {
            expected: expected.context_id,
            actual: envelope.journal.context_id,
        });
    }

    // 6. Threshold check. V1 binds the threshold into context_id, so gates are
    // exact-threshold credentials rather than reusable "prove >= any lower
    // threshold" credentials.
    if envelope.journal.threshold != expected.threshold {
        return Err(VerifyError::ThresholdMismatch {
            expected: expected.threshold,
            journal: envelope.journal.threshold,
        });
    }

    // 7. Challenge/session binding.
    if envelope.presentation_challenge != expected.presentation_challenge {
        return Err(VerifyError::ChallengeMismatch {
            expected: expected.presentation_challenge,
            actual: envelope.presentation_challenge,
        });
    }

    // 8. Presenter pubkey hashes to journal.presenter_id.
    let pubkey = PresenterPubkey::from_slice(envelope.presenter_pubkey.as_bytes())
        .map_err(|_| VerifyError::InvalidPresenterPubkey)?;
    if pubkey.presenter_id() != envelope.journal.presenter_id {
        return Err(VerifyError::PresenterMismatch);
    }

    // 9. Schnorr signature over presentation_digest(journal.digest(), challenge).
    let signature = PresenterSignature::from_slice(envelope.presenter_signature.as_bytes())
        .map_err(|_| VerifyError::InvalidPresenterSignature)?;
    pubkey
        .verify_presentation(
            &envelope.journal.digest(),
            &envelope.presentation_challenge,
            &signature,
        )
        .map_err(|_| VerifyError::InvalidPresenterSignature)?;

    Ok(())
}

// Local mirror of the journal layout — we deliberately decode into raw arrays
// (no Digest32) so the comparison against envelope.journal.*.0 is byte-for-byte.
#[derive(serde::Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use attestation_core::{
        derive_context_id, ContextBindingParams, HexBytes, LezMembershipProof, PresenterSecret,
    };
    use attestation_prover::{
        balance_attestation_image_id, build_balance_attestation_witness, prove_attestation,
        AttestationPublicParams, PresenterWitness, PrivateAccountWitness,
    };

    fn digest(seed: u8) -> Digest32 {
        Digest32([seed; 32])
    }

    fn fixture() -> (BalanceAttestationEnvelope, ExpectedGate) {
        std::env::set_var("RISC0_DEV_MODE", "1");

        let params = AttestationPublicParams {
            threshold: 25,
            chain_id: digest(0x10),
            verifier_id: digest(0x20),
            gate_id: digest(0x30),
            circuit_image_id: Digest32(balance_attestation_image_id()),
        };
        let witness = build_balance_attestation_witness(
            PrivateAccountWitness {
                account_id: digest(0x06),
                npk: digest(0x07),
                program_owner: [1, 2, 3, 4, 5, 6, 7, 8],
                balance: 42,
                nonce: 123_456,
                data: HexBytes::new(b"verifier fixture".to_vec()),
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
        let envelope =
            prove_attestation(&witness, &params, digest(0x44)).expect("prove should succeed");

        let ctx_params = ContextBindingParams {
            chain_id: params.chain_id,
            circuit_image_id: params.circuit_image_id,
            verifier_id: params.verifier_id,
            gate_id: params.gate_id,
            threshold: params.threshold,
        };
        let expected = ExpectedGate {
            context_id: derive_context_id(&ctx_params),
            threshold: 25,
            presentation_challenge: digest(0x44),
        };
        (envelope, expected)
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1; run with: RISC0_DEV_MODE=1 cargo test -p attestation-verifier -- --include-ignored"]
    fn verify_envelope_accepts_valid_envelope() {
        let (envelope, expected) = fixture();
        verify_envelope(&envelope, &expected).expect("valid envelope must verify");
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1"]
    fn verify_envelope_rejects_context_mismatch() {
        let (envelope, mut expected) = fixture();
        expected.context_id = digest(0xff);
        let err = verify_envelope(&envelope, &expected).unwrap_err();
        assert!(matches!(err, VerifyError::ContextMismatch { .. }));
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1"]
    fn verify_envelope_rejects_threshold_mismatch() {
        let (envelope, mut expected) = fixture();
        expected.threshold = 1_000_000;
        let err = verify_envelope(&envelope, &expected).unwrap_err();
        assert!(matches!(err, VerifyError::ThresholdMismatch { .. }));
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1"]
    fn verify_envelope_rejects_challenge_mismatch() {
        let (envelope, mut expected) = fixture();
        expected.presentation_challenge = digest(0x45);
        let err = verify_envelope(&envelope, &expected).unwrap_err();
        assert!(matches!(err, VerifyError::ChallengeMismatch { .. }));
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1"]
    fn verify_envelope_rejects_tampered_pubkey() {
        let (mut envelope, expected) = fixture();
        // Replace pubkey with an unrelated valid pubkey (different secret).
        let other_secret = PresenterSecret::new([0x99; 32]).unwrap();
        envelope.presenter_pubkey = HexBytes::new(other_secret.pubkey().as_bytes().to_vec());
        let err = verify_envelope(&envelope, &expected).unwrap_err();
        assert!(matches!(err, VerifyError::PresenterMismatch));
    }

    #[test]
    #[ignore = "requires RISC0_DEV_MODE=1"]
    fn verify_envelope_rejects_tampered_signature() {
        let (mut envelope, expected) = fixture();
        let mut bad_sig = envelope.presenter_signature.as_bytes().to_vec();
        bad_sig[0] ^= 0xff;
        envelope.presenter_signature = HexBytes::new(bad_sig);
        let err = verify_envelope(&envelope, &expected).unwrap_err();
        assert!(matches!(err, VerifyError::InvalidPresenterSignature));
    }
}
