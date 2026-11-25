//! HTTP request and response documents for the `/v1` sync API.
//!
//! The shapes mirror `$defs` in `contracts/sync/v1/sync.schema.json`; the
//! endpoint behavior is described by `contracts/sync/v1/openapi.yaml`. The
//! server stores encrypted envelopes and routing metadata only, so none of
//! these documents contains ledger names, amounts, notes, or plaintext.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::MAX_OPERATIONS_PER_BATCH;
use crate::envelope::{EnvelopeError, OperationEnvelope};

/// A request for an email magic link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagicLinkRequest {
    /// The account email address.
    pub email: String,
}

/// A magic-link token exchange request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    /// The token delivered by email.
    pub token: String,
}

/// A refresh-token rotation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    /// The single-use refresh token.
    pub refresh_token: String,
}

/// A non-financial account summary returned by authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountSummary {
    /// The account identity.
    pub account_id: Uuid,
    /// The account email address.
    pub email: String,
}

/// A session returned by verification or refresh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthSession {
    /// The short-lived access token.
    pub access_token: String,
    /// The rotating refresh token.
    pub refresh_token: String,
    /// The access-token lifetime in seconds.
    pub expires_in: u32,
    /// The account summary.
    pub account: AccountSummary,
}

/// The client platform a device runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    /// A Windows client.
    Windows,
    /// An Android client.
    Android,
}

/// The lifecycle state of a registered device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    /// The device may authenticate and push operations.
    Active,
    /// The device is revoked; its tokens and operations are rejected.
    Revoked,
}

/// A device registration request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRegistrationRequest {
    /// The client-generated device identity.
    pub device_id: Uuid,
    /// A user-visible device name.
    pub name: String,
    /// The client platform.
    pub platform: Platform,
    /// The device Ed25519 public key as raw 32-byte base64url.
    pub signing_public_key: String,
    /// The device X25519 public key as raw 32-byte base64url.
    pub agreement_public_key: String,
}

/// Registered device metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    /// The device identity.
    pub device_id: Uuid,
    /// The user-visible device name.
    pub name: String,
    /// The client platform.
    pub platform: Platform,
    /// The device Ed25519 public key as raw 32-byte base64url.
    pub signing_public_key: String,
    /// The device X25519 public key as raw 32-byte base64url.
    pub agreement_public_key: String,
    /// The lifecycle state.
    pub status: DeviceStatus,
    /// When the device was registered.
    pub registered_at: String,
    /// When the device was revoked, or `null` while it is active.
    pub revoked_at: Option<String>,
}

/// The device list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceList {
    /// The account's devices, including revoked devices.
    pub devices: Vec<Device>,
}

/// An opaque ledger data-key envelope for one device and key version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyEnvelope {
    /// The device the key is wrapped for.
    pub device_id: Uuid,
    /// The ledger data-key version.
    pub key_version: u32,
    /// The opaque wrapped key as base64url. The server never interprets it.
    pub body: String,
}

/// A ledger registration request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerRegistrationRequest {
    /// The client-generated ledger identity, equal to the settings record
    /// identifier.
    pub ledger_id: Uuid,
    /// The initial key envelopes.
    pub key_envelopes: Vec<KeyEnvelope>,
}

/// Ledger routing metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerSummary {
    /// The ledger identity.
    pub ledger_id: Uuid,
    /// The latest ledger data-key version.
    pub latest_key_version: u32,
    /// The latest operation sequence, or `0` when the log is empty.
    pub latest_seq: u64,
}

/// The ledger list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerList {
    /// The account's ledgers.
    pub ledgers: Vec<LedgerSummary>,
}

/// A key-envelope update for one key version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerKeyUpdateRequest {
    /// The key version whose envelope set is replaced.
    pub key_version: u32,
    /// The complete envelope set for the key version.
    pub envelopes: Vec<KeyEnvelope>,
}

/// A batch push request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushRequest {
    /// The operations to append, in order.
    pub ops: Vec<OperationEnvelope>,
}

impl PushRequest {
    /// Validates the batch count and every operation envelope.
    ///
    /// The caller remains responsible for the serialized byte limits described
    /// by [`crate::MAX_BATCH_BYTES`] and [`crate::MAX_OPERATION_BYTES`]; they
    /// depend on the encoded request body and are enforced by the transport and
    /// the server.
    ///
    /// # Errors
    ///
    /// Returns [`BatchError::Empty`] for an empty batch,
    /// [`BatchError::TooManyOperations`] above [`MAX_OPERATIONS_PER_BATCH`],
    /// and [`BatchError::Operation`] for the first invalid envelope.
    pub fn validate(&self) -> Result<(), BatchError> {
        if self.ops.is_empty() {
            return Err(BatchError::Empty);
        }
        if self.ops.len() > MAX_OPERATIONS_PER_BATCH {
            return Err(BatchError::TooManyOperations {
                maximum: MAX_OPERATIONS_PER_BATCH,
                found: self.ops.len(),
            });
        }
        for envelope in &self.ops {
            envelope.validate().map_err(BatchError::Operation)?;
        }
        Ok(())
    }
}

/// The server's assignment for one accepted operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedOperation {
    /// The operation identity from the envelope.
    pub op_id: Uuid,
    /// The per-ledger sequence assigned to the operation.
    pub seq: u64,
}

/// The batch push response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushResponse {
    /// One entry per pushed operation, in request order.
    pub accepted: Vec<AcceptedOperation>,
    /// The ledger's latest sequence after the push.
    pub latest_seq: u64,
}

/// An operation stored in the ledger log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredOperation {
    /// The per-ledger sequence.
    pub seq: u64,
    /// When the server accepted the operation.
    pub received_at: String,
    /// The signed envelope exactly as pushed.
    pub envelope: OperationEnvelope,
}

/// The pull response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PullResponse {
    /// The operations in ascending sequence order.
    pub ops: Vec<StoredOperation>,
    /// The ledger's latest sequence.
    pub latest_seq: u64,
    /// Whether further operations are available for the next pull.
    pub has_more: bool,
}

/// The device cursor acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AckRequest {
    /// The highest sequence the device has applied.
    pub seq: u64,
}

/// A snapshot upload reservation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotUploadRequest {
    /// The ledger sequence the snapshot covers.
    pub seq: u64,
    /// The SHA-256 of the encrypted snapshot, lowercase hexadecimal.
    pub sha256: String,
    /// The encrypted snapshot size in bytes.
    pub size: u64,
    /// The ledger data-key version used to encrypt the snapshot.
    pub key_version: u32,
}

/// The snapshot upload reservation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotUploadResponse {
    /// The snapshot identity.
    pub snapshot_id: Uuid,
    /// A short-lived object-storage upload URL.
    pub upload_url: String,
    /// When the upload URL expires.
    pub expires_at: String,
}

/// Snapshot metadata and a download URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// The snapshot identity.
    pub snapshot_id: Uuid,
    /// The ledger sequence the snapshot covers.
    pub seq: u64,
    /// The SHA-256 of the encrypted snapshot, lowercase hexadecimal.
    pub sha256: String,
    /// The encrypted snapshot size in bytes.
    pub size: u64,
    /// The ledger data-key version used to encrypt the snapshot.
    pub key_version: u32,
    /// When the snapshot metadata was created.
    pub created_at: String,
    /// A short-lived object-storage download URL.
    pub download_url: String,
    /// When the download URL expires.
    pub expires_at: String,
}

/// A stable machine-readable API error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The request conflicts with stored state.
    Conflict,
    /// The device has been revoked.
    DeviceRevoked,
    /// The caller may not perform the request.
    Forbidden,
    /// The server failed to process the request.
    Internal,
    /// The request is malformed or invalid.
    InvalidRequest,
    /// The addressed entity does not exist.
    NotFound,
    /// The request exceeds a size limit.
    PayloadTooLarge,
    /// The request is rate limited.
    RateLimited,
    /// The access token is missing, expired, or invalid.
    Unauthorized,
    /// The request uses an unsupported protocol version.
    UnsupportedProtocolVersion,
}

/// The API error document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiError {
    /// The stable error code.
    pub code: ErrorCode,
    /// A diagnostic message for logs and support; never localized.
    pub message: String,
    /// Whether the same request may be retried with backoff.
    pub retryable: bool,
    /// Optional structured context; field names are stable per error code.
    pub details: Map<String, Value>,
}

/// A violation of the batch push limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchError {
    /// The batch contains no operations.
    Empty,
    /// The batch contains more operations than the protocol permits.
    TooManyOperations {
        /// The maximum number of operations.
        maximum: usize,
        /// The number of operations in the request.
        found: usize,
    },
    /// An operation envelope failed validation.
    Operation(EnvelopeError),
}

impl fmt::Display for BatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("push batch is empty"),
            Self::TooManyOperations { maximum, found } => write!(
                formatter,
                "push batch contains {found} operations; at most {maximum} are allowed"
            ),
            Self::Operation(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for BatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Operation(error) => Some(error),
            Self::Empty | Self::TooManyOperations { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PAYLOAD_VERSION, PROTOCOL_VERSION};

    fn envelope() -> OperationEnvelope {
        OperationEnvelope {
            protocol_version: PROTOCOL_VERSION,
            op_id: Uuid::parse_str("0199c0f0-0000-7000-8000-000000000010").unwrap(),
            ledger_id: Uuid::parse_str("0199c0f0-0000-7000-8000-000000000001").unwrap(),
            device_id: Uuid::parse_str("0199c0f0-0000-7000-8000-000000000002").unwrap(),
            entity_type: crate::EntityType::Transaction,
            entity_id: Uuid::parse_str("0199c0f0-0000-7000-8000-000000000020").unwrap(),
            op_type: crate::OperationType::Upsert,
            base_revision: 4,
            client_timestamp: 1_795_000_000_000,
            idempotency_key: Uuid::parse_str("0199c0f0-0000-7000-8000-000000000011").unwrap(),
            key_version: 1,
            nonce: "AAECAwQFBgcICQoL".to_string(),
            ciphertext: "Ymx1ZXdoYWxlIHN5bnRoZXRpYyBjaXBoZXJ0ZXh0".to_string(),
            payload_version: PAYLOAD_VERSION,
            signature: "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-Pw".to_string(),
        }
    }

    #[test]
    fn a_valid_batch_is_accepted() {
        let request = PushRequest {
            ops: vec![envelope(), envelope()],
        };
        request.validate().unwrap();
    }

    #[test]
    fn batch_bounds_are_enforced() {
        assert_eq!(
            PushRequest { ops: Vec::new() }.validate(),
            Err(BatchError::Empty)
        );

        let request = PushRequest {
            ops: vec![envelope(); MAX_OPERATIONS_PER_BATCH + 1],
        };
        assert_eq!(
            request.validate(),
            Err(BatchError::TooManyOperations {
                maximum: MAX_OPERATIONS_PER_BATCH,
                found: MAX_OPERATIONS_PER_BATCH + 1
            })
        );
    }

    #[test]
    fn an_invalid_envelope_fails_the_batch() {
        let mut invalid = envelope();
        invalid.protocol_version = 2;
        let request = PushRequest {
            ops: vec![envelope(), invalid],
        };
        assert!(matches!(request.validate(), Err(BatchError::Operation(_))));
    }

    #[test]
    fn the_error_document_round_trips() {
        let error = ApiError {
            code: ErrorCode::RateLimited,
            message: "Too many requests; retry after the indicated delay.".to_string(),
            retryable: true,
            details: Map::from_iter([("retry_after_seconds".to_string(), Value::from(30))]),
        };
        let json = serde_json::to_string(&error).unwrap();
        let parsed: ApiError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, error);
        assert!(json.contains("\"rate_limited\""));
    }

    #[test]
    fn device_status_uses_the_contract_names() {
        assert_eq!(
            serde_json::to_value(DeviceStatus::Revoked).unwrap(),
            Value::from("revoked")
        );
        assert_eq!(
            serde_json::to_value(Platform::Android).unwrap(),
            Value::from("android")
        );
    }

    #[test]
    fn batch_errors_display_a_message() {
        let errors = [
            BatchError::Empty,
            BatchError::TooManyOperations {
                maximum: MAX_OPERATIONS_PER_BATCH,
                found: MAX_OPERATIONS_PER_BATCH + 1,
            },
            BatchError::Operation(EnvelopeError::InvalidIdentifier { field: "op_id" }),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty(), "{error:?}");
        }
        let operation = BatchError::Operation(EnvelopeError::InvalidIdentifier { field: "op_id" });
        assert!(std::error::Error::source(&operation).is_some());
        assert!(std::error::Error::source(&BatchError::Empty).is_none());
    }
}
