//! Signed operation envelopes and the entity/operation compatibility matrix.
//!
//! The wire shape is `$defs.operationEnvelope` in
//! `contracts/sync/v1/sync.schema.json`. The server validates the envelope and
//! signature, assigns a per-ledger sequence, and stores the ciphertext without
//! reading the payload.

use std::fmt;

use core_domain::{SyncEntityType, SyncOperation, SyncOperationType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{MAX_OPERATION_BYTES, MAX_PROTOCOL_INTEGER, PAYLOAD_VERSION, PROTOCOL_VERSION};

/// The entity kind an operation mutates.
///
/// The encrypted payload carries the full entity aggregate; the envelope's
/// entity type lets the server and the pulling client route the operation
/// without decrypting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// An account.
    Account,
    /// An account group.
    AccountGroup,
    /// A leaf category.
    Category,
    /// A category group.
    CategoryGroup,
    /// A counterparty.
    Counterparty,
    /// An exchange-rate revision.
    ExchangeRate,
    /// The single ledger profile.
    LedgerProfile,
    /// The single ledger settings record.
    LedgerSettings,
    /// A transaction note preset.
    NotePreset,
    /// A reconciliation flag.
    ReconciliationFlag,
    /// A descriptive tag.
    Tag,
    /// A transaction, including opening balances and transfers.
    Transaction,
}

impl EntityType {
    /// Every entity type in protocol version 1.
    pub const ALL: [Self; 12] = [
        Self::Account,
        Self::AccountGroup,
        Self::Category,
        Self::CategoryGroup,
        Self::Counterparty,
        Self::ExchangeRate,
        Self::LedgerProfile,
        Self::LedgerSettings,
        Self::NotePreset,
        Self::ReconciliationFlag,
        Self::Tag,
        Self::Transaction,
    ];

    /// Returns the contract's enum value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::AccountGroup => "account_group",
            Self::Category => "category",
            Self::CategoryGroup => "category_group",
            Self::Counterparty => "counterparty",
            Self::ExchangeRate => "exchange_rate",
            Self::LedgerProfile => "ledger_profile",
            Self::LedgerSettings => "ledger_settings",
            Self::NotePreset => "note_preset",
            Self::ReconciliationFlag => "reconciliation_flag",
            Self::Tag => "tag",
            Self::Transaction => "transaction",
        }
    }

    /// Returns the operations the contract permits for this entity type.
    #[must_use]
    pub const fn allowed_operations(self) -> &'static [OperationType] {
        match self {
            Self::Transaction => &[
                OperationType::Upsert,
                OperationType::Delete,
                OperationType::Restore,
            ],
            Self::ExchangeRate => &[OperationType::RateUpsert],
            Self::LedgerSettings => &[OperationType::SettingsReplace],
            Self::LedgerProfile => &[OperationType::ProfileReplace],
            Self::Account
            | Self::AccountGroup
            | Self::Category
            | Self::CategoryGroup
            | Self::Counterparty
            | Self::NotePreset
            | Self::ReconciliationFlag
            | Self::Tag => &[OperationType::Upsert],
        }
    }

    /// Returns whether the operation is permitted for this entity type.
    #[must_use]
    pub fn allows(self, operation: OperationType) -> bool {
        self.allowed_operations().contains(&operation)
    }

    /// Converts to the storage-neutral sync entity type.
    #[must_use]
    pub const fn to_sync(self) -> SyncEntityType {
        match self {
            Self::Account => SyncEntityType::Account,
            Self::AccountGroup => SyncEntityType::AccountGroup,
            Self::Category => SyncEntityType::Category,
            Self::CategoryGroup => SyncEntityType::CategoryGroup,
            Self::Counterparty => SyncEntityType::Counterparty,
            Self::ExchangeRate => SyncEntityType::ExchangeRate,
            Self::LedgerProfile => SyncEntityType::LedgerProfile,
            Self::LedgerSettings => SyncEntityType::LedgerSettings,
            Self::NotePreset => SyncEntityType::NotePreset,
            Self::ReconciliationFlag => SyncEntityType::ReconciliationFlag,
            Self::Tag => SyncEntityType::Tag,
            Self::Transaction => SyncEntityType::Transaction,
        }
    }

    /// Converts from the storage-neutral sync entity type.
    #[must_use]
    pub const fn from_sync(value: SyncEntityType) -> Self {
        match value {
            SyncEntityType::Account => Self::Account,
            SyncEntityType::AccountGroup => Self::AccountGroup,
            SyncEntityType::Category => Self::Category,
            SyncEntityType::CategoryGroup => Self::CategoryGroup,
            SyncEntityType::Counterparty => Self::Counterparty,
            SyncEntityType::ExchangeRate => Self::ExchangeRate,
            SyncEntityType::LedgerProfile => Self::LedgerProfile,
            SyncEntityType::LedgerSettings => Self::LedgerSettings,
            SyncEntityType::NotePreset => Self::NotePreset,
            SyncEntityType::ReconciliationFlag => Self::ReconciliationFlag,
            SyncEntityType::Tag => Self::Tag,
            SyncEntityType::Transaction => Self::Transaction,
        }
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The kind of mutation an operation applies.
///
/// Every permitted operation carries the full entity state in its payload.
/// `Delete` and `Restore` carry the transaction tombstone state; `Upsert`,
/// `SettingsReplace`, `ProfileReplace`, and `RateUpsert` are full-state
/// replacements for their entity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Soft-delete a transaction.
    Delete,
    /// Replace the ledger profile.
    ProfileReplace,
    /// Add or supersede an exchange-rate revision.
    RateUpsert,
    /// Restore a soft-deleted transaction.
    Restore,
    /// Replace the ledger settings.
    SettingsReplace,
    /// Create or replace an entity aggregate.
    Upsert,
}

impl OperationType {
    /// Every operation type in protocol version 1.
    pub const ALL: [Self; 6] = [
        Self::Delete,
        Self::ProfileReplace,
        Self::RateUpsert,
        Self::Restore,
        Self::SettingsReplace,
        Self::Upsert,
    ];

    /// Returns the contract's enum value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::ProfileReplace => "profile_replace",
            Self::RateUpsert => "rate_upsert",
            Self::Restore => "restore",
            Self::SettingsReplace => "settings_replace",
            Self::Upsert => "upsert",
        }
    }

    /// Converts to the storage-neutral sync operation type.
    #[must_use]
    pub const fn to_sync(self) -> SyncOperationType {
        match self {
            Self::Delete => SyncOperationType::Delete,
            Self::ProfileReplace => SyncOperationType::ProfileReplace,
            Self::RateUpsert => SyncOperationType::RateUpsert,
            Self::Restore => SyncOperationType::Restore,
            Self::SettingsReplace => SyncOperationType::SettingsReplace,
            Self::Upsert => SyncOperationType::Upsert,
        }
    }

    /// Converts from the storage-neutral sync operation type.
    #[must_use]
    pub const fn from_sync(value: SyncOperationType) -> Self {
        match value {
            SyncOperationType::Delete => Self::Delete,
            SyncOperationType::ProfileReplace => Self::ProfileReplace,
            SyncOperationType::RateUpsert => Self::RateUpsert,
            SyncOperationType::Restore => Self::Restore,
            SyncOperationType::SettingsReplace => Self::SettingsReplace,
            SyncOperationType::Upsert => Self::Upsert,
        }
    }
}

impl fmt::Display for OperationType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A signed, end-to-end encrypted operation.
///
/// Field order is part of the emitted shape and follows the contract. The
/// server verifies the signature over the RFC 8785 canonical JSON of the
/// envelope with `signature` omitted, and never reads the encrypted payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationEnvelope {
    /// The sync protocol version. This build writes [`PROTOCOL_VERSION`].
    pub protocol_version: u32,
    /// The operation identity, a UUIDv7 unique within the ledger.
    pub op_id: Uuid,
    /// The ledger identity, a UUIDv7 equal to the settings record identifier.
    pub ledger_id: Uuid,
    /// The registered device that produced the operation.
    pub device_id: Uuid,
    /// The entity kind the operation mutates.
    pub entity_type: EntityType,
    /// The identity of the entity the operation mutates.
    pub entity_id: Uuid,
    /// The mutation kind, constrained by [`EntityType::allowed_operations`].
    pub op_type: OperationType,
    /// The entity revision the edit was based on; `0` when creating the entity.
    pub base_revision: u64,
    /// Informational client time as UTC epoch milliseconds.
    pub client_timestamp: i64,
    /// A client-generated key reused on every retry; the server deduplicates
    /// by it.
    pub idempotency_key: Uuid,
    /// The ledger data key version used to encrypt the payload.
    pub key_version: u32,
    /// The 12-byte AEAD nonce, unpadded base64url.
    pub nonce: String,
    /// The encrypted payload, unpadded base64url.
    pub ciphertext: String,
    /// The decrypted payload schema version. This build writes
    /// [`PAYLOAD_VERSION`].
    pub payload_version: u32,
    /// The 64-byte Ed25519 signature over the RFC 8785 canonical JSON of this
    /// envelope with `signature` omitted, unpadded base64url.
    pub signature: String,
}

impl OperationEnvelope {
    /// Validates the envelope against the protocol v1 rules that a JSON Schema
    /// cannot express completely.
    ///
    /// This checks the protocol and payload versions, the entity/operation
    /// matrix, the UUID version of the operation, ledger, and entity
    /// identifiers, the numeric ranges, and the encodings of the nonce,
    /// ciphertext, and signature. It does not verify the signature; that is the
    /// cryptographic layer's job.
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError`] for the first rule that fails.
    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(EnvelopeError::UnsupportedProtocolVersion {
                supported: PROTOCOL_VERSION,
                found: self.protocol_version,
            });
        }
        if self.payload_version != PAYLOAD_VERSION {
            return Err(EnvelopeError::UnsupportedPayloadVersion {
                supported: PAYLOAD_VERSION,
                found: self.payload_version,
            });
        }
        if !self.entity_type.allows(self.op_type) {
            return Err(EnvelopeError::OperationNotAllowed {
                entity_type: self.entity_type,
                op_type: self.op_type,
            });
        }

        for (field, identifier) in [
            ("op_id", self.op_id),
            ("ledger_id", self.ledger_id),
            ("entity_id", self.entity_id),
        ] {
            if identifier.get_version_num() != 7 {
                return Err(EnvelopeError::InvalidIdentifier { field });
            }
        }

        if self.base_revision > MAX_PROTOCOL_INTEGER {
            return Err(EnvelopeError::OutOfRange {
                field: "base_revision",
            });
        }
        if self.client_timestamp < 0 || self.client_timestamp as u64 > MAX_PROTOCOL_INTEGER {
            return Err(EnvelopeError::OutOfRange {
                field: "client_timestamp",
            });
        }
        if self.key_version == 0 {
            return Err(EnvelopeError::OutOfRange {
                field: "key_version",
            });
        }

        if decoded_base64url_len(&self.nonce) != Some(12) {
            return Err(EnvelopeError::InvalidEncoding { field: "nonce" });
        }
        if decoded_base64url_len(&self.signature) != Some(64) {
            return Err(EnvelopeError::InvalidEncoding { field: "signature" });
        }
        match decoded_base64url_len(&self.ciphertext) {
            Some(length) if (1..=MAX_OPERATION_BYTES).contains(&length) => {}
            _ => {
                return Err(EnvelopeError::InvalidEncoding {
                    field: "ciphertext",
                });
            }
        }

        Ok(())
    }

    /// Returns the RFC 8785 canonical JSON used as the Ed25519 signing input.
    ///
    /// The object keys are sorted, the encoding is compact, and the
    /// `signature` field is omitted. Sign these bytes with the device signing
    /// key and verify them with the registered public key.
    #[must_use]
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut output = String::with_capacity(512);
        let mut first = true;
        output.push('{');

        push_member_name(&mut output, &mut first, "base_revision");
        output.push_str(&self.base_revision.to_string());
        push_member_name(&mut output, &mut first, "ciphertext");
        push_json_string(&mut output, &self.ciphertext);
        push_member_name(&mut output, &mut first, "client_timestamp");
        output.push_str(&self.client_timestamp.to_string());
        push_member_name(&mut output, &mut first, "device_id");
        push_json_string(&mut output, &self.device_id.to_string());
        push_member_name(&mut output, &mut first, "entity_id");
        push_json_string(&mut output, &self.entity_id.to_string());
        push_member_name(&mut output, &mut first, "entity_type");
        push_json_string(&mut output, self.entity_type.as_str());
        push_member_name(&mut output, &mut first, "idempotency_key");
        push_json_string(&mut output, &self.idempotency_key.to_string());
        push_member_name(&mut output, &mut first, "key_version");
        output.push_str(&self.key_version.to_string());
        push_member_name(&mut output, &mut first, "ledger_id");
        push_json_string(&mut output, &self.ledger_id.to_string());
        push_member_name(&mut output, &mut first, "nonce");
        push_json_string(&mut output, &self.nonce);
        push_member_name(&mut output, &mut first, "op_id");
        push_json_string(&mut output, &self.op_id.to_string());
        push_member_name(&mut output, &mut first, "op_type");
        push_json_string(&mut output, self.op_type.as_str());
        push_member_name(&mut output, &mut first, "payload_version");
        output.push_str(&self.payload_version.to_string());
        push_member_name(&mut output, &mut first, "protocol_version");
        output.push_str(&self.protocol_version.to_string());

        output.push('}');
        output.into_bytes()
    }

    /// Builds an envelope from a stored sync operation.
    #[must_use]
    pub fn from_sync_operation(operation: &SyncOperation) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            op_id: operation.op_id,
            ledger_id: operation.ledger_id,
            device_id: operation.device_id,
            entity_type: EntityType::from_sync(operation.entity_type),
            entity_id: operation.entity_id,
            op_type: OperationType::from_sync(operation.op_type),
            base_revision: operation.base_revision,
            client_timestamp: operation.client_timestamp,
            idempotency_key: operation.idempotency_key,
            key_version: operation.key_version,
            nonce: operation.nonce.clone(),
            ciphertext: operation.ciphertext.clone(),
            payload_version: operation.payload_version,
            signature: operation.signature.clone(),
        }
    }

    /// Converts the envelope to a stored sync operation.
    #[must_use]
    pub fn to_sync_operation(&self, server_seq: Option<u64>) -> SyncOperation {
        SyncOperation {
            op_id: self.op_id,
            ledger_id: self.ledger_id,
            device_id: self.device_id,
            entity_type: self.entity_type.to_sync(),
            entity_id: self.entity_id,
            op_type: self.op_type.to_sync(),
            base_revision: self.base_revision,
            client_timestamp: self.client_timestamp,
            idempotency_key: self.idempotency_key,
            key_version: self.key_version,
            nonce: self.nonce.clone(),
            ciphertext: self.ciphertext.clone(),
            payload_version: self.payload_version,
            signature: self.signature.clone(),
            server_seq,
        }
    }
}

/// A violation of the protocol v1 envelope rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    /// The envelope declares a protocol version this build does not support.
    UnsupportedProtocolVersion {
        /// The version this build writes and accepts.
        supported: u32,
        /// The version found in the envelope.
        found: u32,
    },
    /// The envelope declares a payload version this build does not support.
    UnsupportedPayloadVersion {
        /// The payload version this build writes and accepts.
        supported: u32,
        /// The version found in the envelope.
        found: u32,
    },
    /// The operation is not defined for the entity type.
    OperationNotAllowed {
        /// The entity kind.
        entity_type: EntityType,
        /// The rejected operation kind.
        op_type: OperationType,
    },
    /// An identifier is not a UUIDv7.
    InvalidIdentifier {
        /// The field name.
        field: &'static str,
    },
    /// A numeric field is outside its permitted range.
    OutOfRange {
        /// The field name.
        field: &'static str,
    },
    /// A binary field is not canonical unpadded base64url of the required
    /// length.
    InvalidEncoding {
        /// The field name.
        field: &'static str,
    },
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion { supported, found } => write!(
                formatter,
                "unsupported protocol version: found {found}, supported {supported}"
            ),
            Self::UnsupportedPayloadVersion { supported, found } => write!(
                formatter,
                "unsupported payload version: found {found}, supported {supported}"
            ),
            Self::OperationNotAllowed {
                entity_type,
                op_type,
            } => write!(
                formatter,
                "operation {op_type} is not allowed for entity {entity_type}"
            ),
            Self::InvalidIdentifier { field } => {
                write!(formatter, "invalid identifier: {field} must be a UUIDv7")
            }
            Self::OutOfRange { field } => write!(formatter, "field out of range: {field}"),
            Self::InvalidEncoding { field } => write!(formatter, "invalid encoding: {field}"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Appends a comma-separated object member name and the colon.
fn push_member_name(output: &mut String, first: &mut bool, name: &str) {
    if !*first {
        output.push(',');
    }
    *first = false;
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
}

/// Appends a JSON string with the escaping required by RFC 8785.
fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\u{0c}' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            control if control <= '\u{1f}' => {
                let code = u32::from(control);
                output.push_str("\\u00");
                output.push(char::from_digit(code >> 4, 16).expect("a hex digit"));
                output.push(char::from_digit(code & 0x0f, 16).expect("a hex digit"));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

/// Returns the decoded length of unpadded base64url text, or `None` when the
/// text is not canonical base64url.
fn decoded_base64url_len(value: &str) -> Option<usize> {
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return None;
    }
    let padding = match value.len() % 4 {
        0 => 0,
        2 => 1,
        3 => 2,
        _ => return None,
    };
    Some(value.len() / 4 * 3 + padding)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OP_ID: &str = "0199c0f0-0000-7000-8000-000000000010";
    const LEDGER_ID: &str = "0199c0f0-0000-7000-8000-000000000001";
    const DEVICE_ID: &str = "0199c0f0-0000-7000-8000-000000000002";
    const ENTITY_ID: &str = "0199c0f0-0000-7000-8000-000000000020";
    const IDEMPOTENCY_KEY: &str = "0199c0f0-0000-7000-8000-000000000011";
    const NONCE: &str = "AAECAwQFBgcICQoL";
    const CIPHERTEXT: &str = "Ymx1ZXdoYWxlIHN5bnRoZXRpYyBjaXBoZXJ0ZXh0";
    const SIGNATURE: &str =
        "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-Pw";

    fn identifier(value: &str) -> Uuid {
        Uuid::parse_str(value).unwrap()
    }

    fn envelope() -> OperationEnvelope {
        OperationEnvelope {
            protocol_version: PROTOCOL_VERSION,
            op_id: identifier(OP_ID),
            ledger_id: identifier(LEDGER_ID),
            device_id: identifier(DEVICE_ID),
            entity_type: EntityType::Transaction,
            entity_id: identifier(ENTITY_ID),
            op_type: OperationType::Upsert,
            base_revision: 4,
            client_timestamp: 1_795_000_000_000,
            idempotency_key: identifier(IDEMPOTENCY_KEY),
            key_version: 1,
            nonce: NONCE.to_string(),
            ciphertext: CIPHERTEXT.to_string(),
            payload_version: PAYLOAD_VERSION,
            signature: SIGNATURE.to_string(),
        }
    }

    #[test]
    fn a_valid_envelope_is_accepted() {
        envelope().validate().unwrap();
    }

    #[test]
    fn protocol_and_payload_versions_are_enforced() {
        let mut candidate = envelope();
        candidate.protocol_version = 2;
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::UnsupportedProtocolVersion {
                supported: PROTOCOL_VERSION,
                found: 2
            })
        );

        let mut candidate = envelope();
        candidate.payload_version = 2;
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::UnsupportedPayloadVersion {
                supported: PAYLOAD_VERSION,
                found: 2
            })
        );
    }

    #[test]
    fn operation_matrix_matches_the_contract() {
        for entity_type in EntityType::ALL {
            for op_type in [
                OperationType::Delete,
                OperationType::ProfileReplace,
                OperationType::RateUpsert,
                OperationType::Restore,
                OperationType::SettingsReplace,
                OperationType::Upsert,
            ] {
                let mut candidate = envelope();
                candidate.entity_type = entity_type;
                candidate.op_type = op_type;
                let expected = entity_type.allows(op_type);
                assert_eq!(
                    candidate.validate().is_ok(),
                    expected,
                    "{entity_type} with {op_type}"
                );
            }
        }
    }

    #[test]
    fn identifiers_must_be_version_seven() {
        for field in ["op_id", "ledger_id", "entity_id"] {
            let mut candidate = envelope();
            let version_four = identifier("0199c0f0-0000-4000-8000-000000000010");
            match field {
                "op_id" => candidate.op_id = version_four,
                "ledger_id" => candidate.ledger_id = version_four,
                _ => candidate.entity_id = version_four,
            }
            assert_eq!(
                candidate.validate(),
                Err(EnvelopeError::InvalidIdentifier { field })
            );
        }
    }

    #[test]
    fn numeric_fields_are_bounded() {
        let mut candidate = envelope();
        candidate.client_timestamp = -1;
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::OutOfRange {
                field: "client_timestamp"
            })
        );

        let mut candidate = envelope();
        candidate.client_timestamp = MAX_PROTOCOL_INTEGER as i64 + 1;
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::OutOfRange {
                field: "client_timestamp"
            })
        );

        let mut candidate = envelope();
        candidate.key_version = 0;
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::OutOfRange {
                field: "key_version"
            })
        );

        let mut candidate = envelope();
        candidate.base_revision = MAX_PROTOCOL_INTEGER + 1;
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::OutOfRange {
                field: "base_revision"
            })
        );
    }

    #[test]
    fn encodings_are_checked() {
        let mut candidate = envelope();
        candidate.nonce = "short".to_string();
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::InvalidEncoding { field: "nonce" })
        );

        let mut candidate = envelope();
        candidate.signature = format!("{}==", &SIGNATURE[..84]);
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::InvalidEncoding { field: "signature" })
        );

        let mut candidate = envelope();
        candidate.ciphertext = String::new();
        assert_eq!(
            candidate.validate(),
            Err(EnvelopeError::InvalidEncoding {
                field: "ciphertext"
            })
        );
    }

    #[test]
    fn serde_uses_the_contract_names() {
        let value = serde_json::to_value(envelope()).unwrap();
        assert_eq!(value["protocol_version"], 1);
        assert_eq!(value["entity_type"], "transaction");
        assert_eq!(value["op_type"], "upsert");
        assert_eq!(value["payload_version"], 1);
        let parsed: OperationEnvelope = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, envelope());
    }

    #[test]
    fn enum_values_match_the_contract() {
        for entity_type in EntityType::ALL {
            assert_eq!(
                serde_json::to_value(entity_type).unwrap(),
                serde_json::Value::from(entity_type.as_str())
            );
            assert_eq!(entity_type.to_string(), entity_type.as_str());
        }
        let operation_names: Vec<&str> = OperationType::ALL
            .iter()
            .map(|operation| operation.as_str())
            .collect();
        assert_eq!(
            operation_names,
            [
                "delete",
                "profile_replace",
                "rate_upsert",
                "restore",
                "settings_replace",
                "upsert"
            ]
        );
        for operation in OperationType::ALL {
            assert_eq!(
                serde_json::to_value(operation).unwrap(),
                serde_json::Value::from(operation.as_str())
            );
            assert_eq!(operation.to_string(), operation.as_str());
        }
    }

    #[test]
    fn validation_errors_display_a_message() {
        let errors = [
            EnvelopeError::UnsupportedProtocolVersion {
                supported: 1,
                found: 2,
            },
            EnvelopeError::UnsupportedPayloadVersion {
                supported: 1,
                found: 2,
            },
            EnvelopeError::OperationNotAllowed {
                entity_type: EntityType::Tag,
                op_type: OperationType::Delete,
            },
            EnvelopeError::InvalidIdentifier { field: "op_id" },
            EnvelopeError::OutOfRange {
                field: "key_version",
            },
            EnvelopeError::InvalidEncoding { field: "nonce" },
        ];
        for error in errors {
            assert!(!error.to_string().is_empty(), "{error:?}");
        }
    }

    #[test]
    fn signing_bytes_are_canonical() {
        let bytes = envelope().signing_bytes();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(
            text,
            concat!(
                r#"{"base_revision":4,"#,
                r#""ciphertext":"Ymx1ZXdoYWxlIHN5bnRoZXRpYyBjaXBoZXJ0ZXh0","#,
                r#""client_timestamp":1795000000000,"#,
                r#""device_id":"0199c0f0-0000-7000-8000-000000000002","#,
                r#""entity_id":"0199c0f0-0000-7000-8000-000000000020","#,
                r#""entity_type":"transaction","#,
                r#""idempotency_key":"0199c0f0-0000-7000-8000-000000000011","#,
                r#""key_version":1,"#,
                r#""ledger_id":"0199c0f0-0000-7000-8000-000000000001","#,
                r#""nonce":"AAECAwQFBgcICQoL","#,
                r#""op_id":"0199c0f0-0000-7000-8000-000000000010","#,
                r#""op_type":"upsert","#,
                r#""payload_version":1,"#,
                r#""protocol_version":1}"#
            )
        );
        assert!(!text.contains("signature"));
    }

    #[test]
    fn signing_bytes_escape_json_strings() {
        let mut candidate = envelope();
        candidate.ciphertext = "a\"b\\c\n\u{01}".to_string();
        let text = String::from_utf8(candidate.signing_bytes()).unwrap();
        assert!(text.contains(r#""ciphertext":"a\"b\\c\n\u0001""#), "{text}");
    }
}
