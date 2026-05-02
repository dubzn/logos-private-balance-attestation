use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AttestationErrorCode {
    InvalidEnvelopeVersion,
    InvalidProofSystem,
    InvalidImageId,
    MalformedEnvelope,
    MalformedJournal,
    InvalidReceipt,
    ThresholdMismatch,
    ContextMismatch,
    PresenterMismatch,
    InvalidPresenterSignature,
    StaleCommitmentRoot,
    DuplicateNullifier,
    GateThresholdNotMet,
    PrivateAccountNotFound,
    WalletSyncRequired,
    CommitmentProofUnavailable,
    SequencerRpcFailed,
    CommitmentMismatch,
    MessagingPublishFailed,
    MessagingReceiveFailed,
    MessagingChallengeExpired,
    GateAlreadyInitialized,
    GateNotInitialized,
    UnauthorizedPresenterAccount,
    InvalidGateAccount,
    UnsupportedRuntimePath,
    InternalError,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationError {
    code: AttestationErrorCode,
    detail: Option<String>,
}

impl AttestationErrorCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidEnvelopeVersion => "BA100",
            Self::InvalidProofSystem => "BA101",
            Self::InvalidImageId => "BA102",
            Self::MalformedEnvelope => "BA103",
            Self::MalformedJournal => "BA104",
            Self::InvalidReceipt => "BA200",
            Self::ThresholdMismatch => "BA201",
            Self::ContextMismatch => "BA202",
            Self::PresenterMismatch => "BA203",
            Self::InvalidPresenterSignature => "BA204",
            Self::StaleCommitmentRoot => "BA205",
            Self::DuplicateNullifier => "BA206",
            Self::GateThresholdNotMet => "BA207",
            Self::PrivateAccountNotFound => "BA300",
            Self::WalletSyncRequired => "BA301",
            Self::CommitmentProofUnavailable => "BA302",
            Self::SequencerRpcFailed => "BA303",
            Self::CommitmentMismatch => "BA304",
            Self::MessagingPublishFailed => "BA400",
            Self::MessagingReceiveFailed => "BA401",
            Self::MessagingChallengeExpired => "BA402",
            Self::GateAlreadyInitialized => "BA500",
            Self::GateNotInitialized => "BA501",
            Self::UnauthorizedPresenterAccount => "BA502",
            Self::InvalidGateAccount => "BA503",
            Self::UnsupportedRuntimePath => "BA900",
            Self::InternalError => "BA901",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::InvalidEnvelopeVersion => "InvalidEnvelopeVersion",
            Self::InvalidProofSystem => "InvalidProofSystem",
            Self::InvalidImageId => "InvalidImageId",
            Self::MalformedEnvelope => "MalformedEnvelope",
            Self::MalformedJournal => "MalformedJournal",
            Self::InvalidReceipt => "InvalidReceipt",
            Self::ThresholdMismatch => "ThresholdMismatch",
            Self::ContextMismatch => "ContextMismatch",
            Self::PresenterMismatch => "PresenterMismatch",
            Self::InvalidPresenterSignature => "InvalidPresenterSignature",
            Self::StaleCommitmentRoot => "StaleCommitmentRoot",
            Self::DuplicateNullifier => "DuplicateNullifier",
            Self::GateThresholdNotMet => "GateThresholdNotMet",
            Self::PrivateAccountNotFound => "PrivateAccountNotFound",
            Self::WalletSyncRequired => "WalletSyncRequired",
            Self::CommitmentProofUnavailable => "CommitmentProofUnavailable",
            Self::SequencerRpcFailed => "SequencerRpcFailed",
            Self::CommitmentMismatch => "CommitmentMismatch",
            Self::MessagingPublishFailed => "MessagingPublishFailed",
            Self::MessagingReceiveFailed => "MessagingReceiveFailed",
            Self::MessagingChallengeExpired => "MessagingChallengeExpired",
            Self::GateAlreadyInitialized => "GateAlreadyInitialized",
            Self::GateNotInitialized => "GateNotInitialized",
            Self::UnauthorizedPresenterAccount => "UnauthorizedPresenterAccount",
            Self::InvalidGateAccount => "InvalidGateAccount",
            Self::UnsupportedRuntimePath => "UnsupportedRuntimePath",
            Self::InternalError => "InternalError",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "BA100" => Self::InvalidEnvelopeVersion,
            "BA101" => Self::InvalidProofSystem,
            "BA102" => Self::InvalidImageId,
            "BA103" => Self::MalformedEnvelope,
            "BA104" => Self::MalformedJournal,
            "BA200" => Self::InvalidReceipt,
            "BA201" => Self::ThresholdMismatch,
            "BA202" => Self::ContextMismatch,
            "BA203" => Self::PresenterMismatch,
            "BA204" => Self::InvalidPresenterSignature,
            "BA205" => Self::StaleCommitmentRoot,
            "BA206" => Self::DuplicateNullifier,
            "BA207" => Self::GateThresholdNotMet,
            "BA300" => Self::PrivateAccountNotFound,
            "BA301" => Self::WalletSyncRequired,
            "BA302" => Self::CommitmentProofUnavailable,
            "BA303" => Self::SequencerRpcFailed,
            "BA304" => Self::CommitmentMismatch,
            "BA400" => Self::MessagingPublishFailed,
            "BA401" => Self::MessagingReceiveFailed,
            "BA402" => Self::MessagingChallengeExpired,
            "BA500" => Self::GateAlreadyInitialized,
            "BA501" => Self::GateNotInitialized,
            "BA502" => Self::UnauthorizedPresenterAccount,
            "BA503" => Self::InvalidGateAccount,
            "BA900" => Self::UnsupportedRuntimePath,
            "BA901" => Self::InternalError,
            _ => return None,
        })
    }
}

impl AttestationError {
    pub fn new(code: AttestationErrorCode) -> Self {
        Self { code, detail: None }
    }

    pub fn with_detail(code: AttestationErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: Some(detail.into()),
        }
    }

    pub fn code(&self) -> AttestationErrorCode {
        self.code
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

impl fmt::Display for AttestationErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.code(), self.name())
    }
}

impl fmt::Display for AttestationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Some(detail) => write!(formatter, "{}: {}", self.code, detail),
            None => write!(formatter, "{}", self.code),
        }
    }
}

impl std::error::Error for AttestationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_codes_both_ways() {
        assert_eq!(
            AttestationErrorCode::from_code("BA204"),
            Some(AttestationErrorCode::InvalidPresenterSignature)
        );
        assert_eq!(AttestationErrorCode::DuplicateNullifier.code(), "BA206");
        assert_eq!(
            AttestationErrorCode::UnsupportedRuntimePath.to_string(),
            "BA900 UnsupportedRuntimePath"
        );
    }
}
