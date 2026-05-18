//! Transport-agnostic messaging helpers for LP-0005 proof envelopes.
//!
//! The crate intentionally starts with a local JSON transport. Logos Messaging
//! can replace `LocalFileTransport` by implementing `ProofMessageTransport`
//! without changing the proof envelope or verifier/admission logic.

use std::{
    collections::HashSet,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use attestation_core::{
    AttestationErrorCode, BalanceAttestationEnvelope, Digest32, ENVELOPE_VERSION,
};
use attestation_verifier::{verify_envelope, ExpectedGate, VerifyError};
use serde::{Deserialize, Serialize};

pub const PROOF_MESSAGE_VERSION: u16 = 1;
pub const ADMISSION_BOOK_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofMessageTransportKind {
    LocalJson,
    LogosMessaging,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProofMessage {
    pub version: u16,
    pub transport: ProofMessageTransportKind,
    pub message_id: String,
    pub group_id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub created_at_unix: u64,
    pub envelope: BalanceAttestationEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofMessageMetadata {
    pub message_id: String,
    pub group_id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdmissionBook {
    pub version: u16,
    pub group_id: String,
    pub admissions: Vec<AdmissionRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdmissionRecord {
    pub session_token: String,
    pub message_id: String,
    pub sender: String,
    pub presenter_id: Digest32,
    pub context_id: Digest32,
    pub context_nullifier: Digest32,
    pub admitted_at_unix: u64,
}

#[derive(Debug)]
pub enum MessagingError {
    InvalidMessageVersion { expected: u16, actual: u16 },
    InvalidEnvelopeVersion { expected: u16, actual: u16 },
    InvalidAdmissionBookVersion { expected: u16, actual: u16 },
    EmptyGroupId,
    GroupMismatch { expected: String, actual: String },
    AlreadyAdmitted { nullifier: Digest32 },
    Verify(VerifyError),
    Encode(String),
    Decode(String),
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for MessagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMessageVersion { expected, actual } => {
                write!(
                    f,
                    "invalid proof message version: expected {expected}, got {actual}"
                )
            }
            Self::InvalidEnvelopeVersion { expected, actual } => {
                write!(
                    f,
                    "invalid embedded envelope version: expected {expected}, got {actual}"
                )
            }
            Self::InvalidAdmissionBookVersion { expected, actual } => {
                write!(
                    f,
                    "invalid admission book version: expected {expected}, got {actual}"
                )
            }
            Self::EmptyGroupId => f.write_str("group_id is required"),
            Self::GroupMismatch { expected, actual } => {
                write!(
                    f,
                    "message group mismatch: expected {expected}, got {actual}"
                )
            }
            Self::AlreadyAdmitted { nullifier } => {
                write!(
                    f,
                    "context nullifier already admitted: {}",
                    nullifier.to_hex()
                )
            }
            Self::Verify(error) => {
                write!(f, "message envelope rejected [{}]: {error}", error.code())
            }
            Self::Encode(error) => write!(f, "failed to encode message: {error}"),
            Self::Decode(error) => write!(f, "failed to decode message: {error}"),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for MessagingError {}

impl MessagingError {
    pub fn code(&self) -> AttestationErrorCode {
        match self {
            Self::Verify(error) => error.code(),
            Self::AlreadyAdmitted { .. } => AttestationErrorCode::DuplicateNullifier,
            Self::Decode(_)
            | Self::InvalidMessageVersion { .. }
            | Self::InvalidEnvelopeVersion { .. } => AttestationErrorCode::MessagingReceiveFailed,
            Self::Encode(_) | Self::Io { .. } => AttestationErrorCode::MessagingPublishFailed,
            Self::EmptyGroupId
            | Self::GroupMismatch { .. }
            | Self::InvalidAdmissionBookVersion { .. } => {
                AttestationErrorCode::MessagingReceiveFailed
            }
        }
    }
}

pub trait ProofMessageTransport {
    fn publish(&self, message: &ProofMessage, destination: &Path) -> Result<(), MessagingError>;
    fn receive(&self, source: &Path) -> Result<ProofMessage, MessagingError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalFileTransport;

impl ProofMessageTransport for LocalFileTransport {
    fn publish(&self, message: &ProofMessage, destination: &Path) -> Result<(), MessagingError> {
        write_message(destination, message)
    }

    fn receive(&self, source: &Path) -> Result<ProofMessage, MessagingError> {
        read_message(source)
    }
}

pub fn build_local_message(
    envelope: BalanceAttestationEnvelope,
    metadata: ProofMessageMetadata,
) -> Result<ProofMessage, MessagingError> {
    if metadata.group_id.trim().is_empty() {
        return Err(MessagingError::EmptyGroupId);
    }

    let message = ProofMessage {
        version: PROOF_MESSAGE_VERSION,
        transport: ProofMessageTransportKind::LocalJson,
        message_id: metadata.message_id,
        group_id: metadata.group_id,
        sender: metadata.sender,
        recipient: metadata.recipient,
        created_at_unix: metadata.created_at_unix,
        envelope,
    };
    validate_message_shape(&message)?;
    Ok(message)
}

pub fn validate_message_shape(message: &ProofMessage) -> Result<(), MessagingError> {
    if message.version != PROOF_MESSAGE_VERSION {
        return Err(MessagingError::InvalidMessageVersion {
            expected: PROOF_MESSAGE_VERSION,
            actual: message.version,
        });
    }
    if message.group_id.trim().is_empty() {
        return Err(MessagingError::EmptyGroupId);
    }
    if message.envelope.version != ENVELOPE_VERSION {
        return Err(MessagingError::InvalidEnvelopeVersion {
            expected: ENVELOPE_VERSION,
            actual: message.envelope.version,
        });
    }
    Ok(())
}

pub fn encode_message_pretty(message: &ProofMessage) -> Result<String, MessagingError> {
    validate_message_shape(message)?;
    serde_json::to_string_pretty(message).map_err(|error| MessagingError::Encode(error.to_string()))
}

pub fn decode_message(bytes: &[u8]) -> Result<ProofMessage, MessagingError> {
    let message: ProofMessage =
        serde_json::from_slice(bytes).map_err(|error| MessagingError::Decode(error.to_string()))?;
    validate_message_shape(&message)?;
    Ok(message)
}

pub fn write_message(path: &Path, message: &ProofMessage) -> Result<(), MessagingError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| MessagingError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = encode_message_pretty(message)?;
    fs::write(path, json).map_err(|source| MessagingError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn read_message(path: &Path) -> Result<ProofMessage, MessagingError> {
    let bytes = fs::read(path).map_err(|source| MessagingError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    decode_message(&bytes)
}

pub fn verify_message(
    message: &ProofMessage,
    expected: &ExpectedGate,
) -> Result<(), MessagingError> {
    validate_message_shape(message)?;
    verify_envelope(&message.envelope, expected).map_err(MessagingError::Verify)
}

impl AdmissionBook {
    pub fn new(group_id: impl Into<String>) -> Result<Self, MessagingError> {
        let group_id = group_id.into();
        if group_id.trim().is_empty() {
            return Err(MessagingError::EmptyGroupId);
        }
        Ok(Self {
            version: ADMISSION_BOOK_VERSION,
            group_id,
            admissions: Vec::new(),
        })
    }

    pub fn validate_shape(&self) -> Result<(), MessagingError> {
        if self.version != ADMISSION_BOOK_VERSION {
            return Err(MessagingError::InvalidAdmissionBookVersion {
                expected: ADMISSION_BOOK_VERSION,
                actual: self.version,
            });
        }
        if self.group_id.trim().is_empty() {
            return Err(MessagingError::EmptyGroupId);
        }
        Ok(())
    }

    pub fn member_count(&self) -> usize {
        self.admissions.len()
    }

    pub fn contains_nullifier(&self, nullifier: Digest32) -> bool {
        self.admissions
            .iter()
            .any(|record| record.context_nullifier == nullifier)
    }

    pub fn admit_verified_message(
        &mut self,
        message: &ProofMessage,
        expected: &ExpectedGate,
        admitted_at_unix: u64,
    ) -> Result<AdmissionRecord, MessagingError> {
        self.validate_shape()?;
        validate_message_shape(message)?;

        if message.group_id != self.group_id {
            return Err(MessagingError::GroupMismatch {
                expected: self.group_id.clone(),
                actual: message.group_id.clone(),
            });
        }

        verify_message(message, expected)?;

        let nullifier = message.envelope.journal.context_nullifier;
        if self.contains_nullifier(nullifier) {
            return Err(MessagingError::AlreadyAdmitted { nullifier });
        }

        let record = AdmissionRecord {
            session_token: format!("session-{}", self.admissions.len()),
            message_id: message.message_id.clone(),
            sender: message.sender.clone(),
            presenter_id: message.envelope.journal.presenter_id,
            context_id: message.envelope.journal.context_id,
            context_nullifier: nullifier,
            admitted_at_unix,
        };
        self.admissions.push(record.clone());
        Ok(record)
    }
}

pub fn read_or_init_admission_book(
    path: &Path,
    group_id: &str,
) -> Result<AdmissionBook, MessagingError> {
    if !path.exists() {
        return AdmissionBook::new(group_id.to_owned());
    }

    let json = fs::read_to_string(path).map_err(|source| MessagingError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let book: AdmissionBook =
        serde_json::from_str(&json).map_err(|error| MessagingError::Decode(error.to_string()))?;
    book.validate_shape()?;
    if book.group_id != group_id {
        return Err(MessagingError::GroupMismatch {
            expected: book.group_id.clone(),
            actual: group_id.to_owned(),
        });
    }
    Ok(book)
}

pub fn write_admission_book(path: &Path, book: &AdmissionBook) -> Result<(), MessagingError> {
    book.validate_shape()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| MessagingError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(book)
        .map_err(|error| MessagingError::Encode(error.to_string()))?;
    fs::write(path, json).map_err(|source| MessagingError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn validate_admission_book_unique(book: &AdmissionBook) -> Result<(), MessagingError> {
    let mut seen = HashSet::new();
    for record in &book.admissions {
        if !seen.insert(record.context_nullifier) {
            return Err(MessagingError::AlreadyAdmitted {
                nullifier: record.context_nullifier,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use attestation_core::{
        BalanceAttestationJournal, ContextBindingParams, HexBytes, ProofSystem,
    };

    fn digest(seed: u8) -> Digest32 {
        Digest32([seed; 32])
    }

    fn envelope() -> BalanceAttestationEnvelope {
        let journal = BalanceAttestationJournal::new(
            10,
            digest(0xAA),
            ContextBindingParams {
                chain_id: digest(0x10),
                circuit_image_id: digest(0x20),
                verifier_id: digest(0x30),
                gate_id: digest(0x40),
                threshold: 10,
            }
            .context_id(),
            digest(0x55),
            digest(0x66),
            digest(0x30),
            digest(0x20),
            0,
            1,
        );
        BalanceAttestationEnvelope {
            version: ENVELOPE_VERSION,
            proof_system: ProofSystem::Risc0,
            image_id: journal.circuit_image_id,
            journal,
            receipt: HexBytes::new(vec![1, 2, 3]),
            presenter_pubkey: HexBytes::new(vec![7; 32]),
            presentation_challenge: digest(0x44),
            presenter_signature: HexBytes::new(vec![8; 64]),
        }
    }

    fn metadata() -> ProofMessageMetadata {
        ProofMessageMetadata {
            message_id: "message-1".to_owned(),
            group_id: "demo-chat".to_owned(),
            sender: "alice".to_owned(),
            recipient: Some("host".to_owned()),
            created_at_unix: 123,
        }
    }

    #[test]
    fn message_round_trips_as_json() {
        let message = build_local_message(envelope(), metadata()).unwrap();
        let json = encode_message_pretty(&message).unwrap();
        let decoded = decode_message(json.as_bytes()).unwrap();
        assert_eq!(decoded, message);
        assert_eq!(decoded.transport, ProofMessageTransportKind::LocalJson);
    }

    #[test]
    fn rejects_empty_group() {
        let mut metadata = metadata();
        metadata.group_id.clear();
        let error = build_local_message(envelope(), metadata).unwrap_err();
        assert!(matches!(error, MessagingError::EmptyGroupId));
    }

    #[test]
    fn admission_book_rejects_group_mismatch_before_verify() {
        let message = build_local_message(envelope(), metadata()).unwrap();
        let mut book = AdmissionBook::new("other-chat").unwrap();
        let expected = ExpectedGate {
            context_id: message.envelope.journal.context_id,
            threshold: message.envelope.journal.threshold,
            presentation_challenge: message.envelope.presentation_challenge,
        };
        let error = book
            .admit_verified_message(&message, &expected, 456)
            .unwrap_err();
        assert!(matches!(error, MessagingError::GroupMismatch { .. }));
    }

    #[test]
    fn detects_duplicate_nullifiers_in_existing_book() {
        let nullifier = digest(0x88);
        let record = AdmissionRecord {
            session_token: "session-0".to_owned(),
            message_id: "message-1".to_owned(),
            sender: "alice".to_owned(),
            presenter_id: digest(0x01),
            context_id: digest(0x02),
            context_nullifier: nullifier,
            admitted_at_unix: 1,
        };
        let book = AdmissionBook {
            version: ADMISSION_BOOK_VERSION,
            group_id: "demo-chat".to_owned(),
            admissions: vec![record.clone(), record],
        };
        let error = validate_admission_book_unique(&book).unwrap_err();
        assert!(
            matches!(error, MessagingError::AlreadyAdmitted { nullifier: n } if n == nullifier)
        );
    }
}
