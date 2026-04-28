# Error Codes

This is the initial deterministic error-code plan. Names and numbers should be
shared by the CLI, off-chain verifier, Basecamp backend, and LEZ verifier
program.

The exact Rust enum can be introduced in `attestation-core`.

## Code Ranges

| Range | Area |
| --- | --- |
| `BA1xx` | Envelope and input shape |
| `BA2xx` | Proof and verification |
| `BA3xx` | Wallet, commitment, and sequencer |
| `BA4xx` | Messaging |
| `BA5xx` | LEZ verifier program state |
| `BA9xx` | Internal or unsupported operation |

## Draft Codes

| Code | Name | Meaning |
| --- | --- | --- |
| `BA100` | `InvalidEnvelopeVersion` | The proof envelope version is unsupported. |
| `BA101` | `InvalidProofSystem` | The envelope is not a supported RISC Zero proof. |
| `BA102` | `InvalidImageId` | The receipt image id does not match the expected circuit. |
| `BA103` | `MalformedEnvelope` | Required envelope fields are missing or malformed. |
| `BA104` | `MalformedJournal` | The public journal cannot be decoded. |
| `BA200` | `InvalidReceipt` | RISC Zero receipt verification failed. |
| `BA201` | `ThresholdMismatch` | The supplied threshold does not match the proof journal. |
| `BA202` | `ContextMismatch` | The supplied context does not match the proof journal. |
| `BA203` | `PresenterMismatch` | The presenter trying to use the proof does not match the journal. |
| `BA204` | `InvalidPresenterSignature` | The presenter challenge signature is missing or invalid. |
| `BA205` | `StaleCommitmentRoot` | The commitment root is outside the accepted freshness policy. |
| `BA206` | `DuplicateNullifier` | The context nullifier has already been used. |
| `BA207` | `GateThresholdNotMet` | The proof is valid but not for the gate threshold. |
| `BA300` | `PrivateAccountNotFound` | The private account is not present in local wallet storage. |
| `BA301` | `WalletSyncRequired` | Local private state is missing or stale. |
| `BA302` | `CommitmentProofUnavailable` | Sequencer returned no membership proof for the commitment. |
| `BA303` | `SequencerRpcFailed` | JSON-RPC call to the sequencer failed. |
| `BA304` | `CommitmentMismatch` | Local commitment reconstruction did not match expected LEZ output. |
| `BA400` | `MessagingPublishFailed` | Proof envelope could not be sent over Messaging. |
| `BA401` | `MessagingReceiveFailed` | Proof envelope could not be received or decoded. |
| `BA402` | `MessagingChallengeExpired` | The off-chain verifier challenge expired. |
| `BA500` | `GateAlreadyInitialized` | A gate state account is already initialized. |
| `BA501` | `GateNotInitialized` | A gated action was requested before gate initialization. |
| `BA502` | `UnauthorizedPresenterAccount` | The on-chain presenter account is not authorized. |
| `BA503` | `InvalidGateAccount` | The supplied gate state account does not match the context. |
| `BA900` | `UnsupportedRuntimePath` | A required runtime path is not available in the current LEZ version. |
| `BA901` | `InternalError` | Unexpected internal failure. |

## Logging Rule

Errors must not include private witness values in their messages. In particular,
do not print `npk`, nullifier secret keys, viewing keys, raw private account
data, exact private balance, membership proof siblings, or receipt bytes in
normal failure output. Receipt logging is allowed only behind an explicit
`--debug-receipt` flag.

