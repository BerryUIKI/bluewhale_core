//! Sync entities, operations, conflicts, and the local sync store port.
//!
//! The sync engine lives in `core-sync`; this module defines the storage-neutral
//! types it exchanges with the local database. The server never sees these
//! types: operations are encrypted and signed before they reach a transport.

use std::fmt;

use jiff::Timestamp;
use uuid::Uuid;

use crate::account::{Account, AccountGroup};
use crate::category::{Category, CategoryGroup};
use crate::counterparty::Counterparty;
use crate::exchange_rate::ExchangeRate;
use crate::note_preset::TransactionNotePreset;
use crate::profile::LedgerProfile;
use crate::reconciliation::ReconciliationFlag;
use crate::repository::RepositoryError;
use crate::settings::LedgerSettings;
use crate::tag::LedgerTag;
use crate::transaction::Transaction;

/// The entity kind carried by a operation envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SyncEntityType {
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
    /// The ledger profile.
    LedgerProfile,
    /// The ledger settings.
    LedgerSettings,
    /// A transaction note preset.
    NotePreset,
    /// A reconciliation flag.
    ReconciliationFlag,
    /// A tag.
    Tag,
    /// A transaction.
    Transaction,
}

impl SyncEntityType {
    /// Every entity type in sync protocol version 1.
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

    /// Returns the protocol value.
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

    /// Parses a protocol value.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "account" => Some(Self::Account),
            "account_group" => Some(Self::AccountGroup),
            "category" => Some(Self::Category),
            "category_group" => Some(Self::CategoryGroup),
            "counterparty" => Some(Self::Counterparty),
            "exchange_rate" => Some(Self::ExchangeRate),
            "ledger_profile" => Some(Self::LedgerProfile),
            "ledger_settings" => Some(Self::LedgerSettings),
            "note_preset" => Some(Self::NotePreset),
            "reconciliation_flag" => Some(Self::ReconciliationFlag),
            "tag" => Some(Self::Tag),
            "transaction" => Some(Self::Transaction),
            _ => None,
        }
    }
}

impl fmt::Display for SyncEntityType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The mutation kind carried by an operation envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncOperationType {
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

impl SyncOperationType {
    /// Returns the protocol value.
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

    /// Parses a protocol value.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "delete" => Some(Self::Delete),
            "profile_replace" => Some(Self::ProfileReplace),
            "rate_upsert" => Some(Self::RateUpsert),
            "restore" => Some(Self::Restore),
            "settings_replace" => Some(Self::SettingsReplace),
            "upsert" => Some(Self::Upsert),
            _ => None,
        }
    }
}

impl fmt::Display for SyncOperationType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One entity aggregate in the sync payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEntity {
    /// An account.
    Account(Account),
    /// An account group.
    AccountGroup(AccountGroup),
    /// A leaf category.
    Category(Category),
    /// A category group.
    CategoryGroup(CategoryGroup),
    /// A counterparty.
    Counterparty(Counterparty),
    /// An exchange-rate revision.
    ExchangeRate(ExchangeRate),
    /// The ledger profile.
    LedgerProfile(LedgerProfile),
    /// The ledger settings.
    LedgerSettings(LedgerSettings),
    /// A transaction note preset.
    NotePreset(TransactionNotePreset),
    /// A reconciliation flag.
    ReconciliationFlag(ReconciliationFlag),
    /// A tag.
    Tag(LedgerTag),
    /// A transaction.
    Transaction(Transaction),
}

impl SyncEntity {
    /// Returns the entity kind.
    #[must_use]
    pub fn entity_type(&self) -> SyncEntityType {
        match self {
            Self::Account(_) => SyncEntityType::Account,
            Self::AccountGroup(_) => SyncEntityType::AccountGroup,
            Self::Category(_) => SyncEntityType::Category,
            Self::CategoryGroup(_) => SyncEntityType::CategoryGroup,
            Self::Counterparty(_) => SyncEntityType::Counterparty,
            Self::ExchangeRate(_) => SyncEntityType::ExchangeRate,
            Self::LedgerProfile(_) => SyncEntityType::LedgerProfile,
            Self::LedgerSettings(_) => SyncEntityType::LedgerSettings,
            Self::NotePreset(_) => SyncEntityType::NotePreset,
            Self::ReconciliationFlag(_) => SyncEntityType::ReconciliationFlag,
            Self::Tag(_) => SyncEntityType::Tag,
            Self::Transaction(_) => SyncEntityType::Transaction,
        }
    }

    /// Returns the entity identifier.
    #[must_use]
    pub fn entity_id(&self) -> Uuid {
        match self {
            Self::Account(entity) => entity.id().as_uuid(),
            Self::AccountGroup(entity) => entity.id().as_uuid(),
            Self::Category(entity) => entity.id().as_uuid(),
            Self::CategoryGroup(entity) => entity.id().as_uuid(),
            Self::Counterparty(entity) => entity.id().as_uuid(),
            Self::ExchangeRate(entity) => entity.id().as_uuid(),
            Self::LedgerProfile(entity) => entity.id().as_uuid(),
            Self::LedgerSettings(entity) => entity.id().as_uuid(),
            Self::NotePreset(entity) => entity.id().as_uuid(),
            Self::ReconciliationFlag(entity) => entity.id().as_uuid(),
            Self::Tag(entity) => entity.id().as_uuid(),
            Self::Transaction(entity) => entity.id().as_uuid(),
        }
    }
}

/// One operation stored in the local outbox or the ledger log.
///
/// The fields mirror the protocol envelope. `server_seq` is `None` until the
/// server accepts the operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOperation {
    /// The operation identity.
    pub op_id: Uuid,
    /// The ledger identity.
    pub ledger_id: Uuid,
    /// The device that produced the operation.
    pub device_id: Uuid,
    /// The entity kind.
    pub entity_type: SyncEntityType,
    /// The entity identity.
    pub entity_id: Uuid,
    /// The mutation kind.
    pub op_type: SyncOperationType,
    /// The revision the edit was based on.
    pub base_revision: u64,
    /// Informational client time as UTC epoch milliseconds.
    pub client_timestamp: i64,
    /// The retry key.
    pub idempotency_key: Uuid,
    /// The ledger data-key version.
    pub key_version: u32,
    /// The AEAD nonce, unpadded base64url.
    pub nonce: String,
    /// The encrypted payload, unpadded base64url.
    pub ciphertext: String,
    /// The payload schema version.
    pub payload_version: u32,
    /// The Ed25519 signature, unpadded base64url.
    pub signature: String,
    /// The server sequence, once accepted.
    pub server_seq: Option<u64>,
}

/// A concurrent edit preserved for user review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConflict {
    /// The conflict identity.
    pub id: Uuid,
    /// The entity kind.
    pub entity_type: SyncEntityType,
    /// The entity identity.
    pub entity_id: Uuid,
    /// The losing local aggregate, serialized as a sync payload.
    pub local_payload: Vec<u8>,
    /// The remote operation that won.
    pub remote_op_id: Uuid,
    /// The local revision at detection.
    pub local_revision: u64,
    /// The remote operation's base revision.
    pub remote_base_revision: u64,
    /// When the conflict was detected.
    pub detected_at: Timestamp,
    /// The user's resolution, when resolved.
    pub resolution: Option<SyncConflictResolution>,
    /// When the conflict was resolved.
    pub resolved_at: Option<Timestamp>,
}

/// The user's chosen side of a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncConflictResolution {
    /// Keep the losing local aggregate and enqueue it as a new edit.
    Local,
    /// Keep the server-order remote aggregate.
    Remote,
}

impl SyncConflictResolution {
    /// Returns the stored value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }

    /// Parses a stored value.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "local" => Some(Self::Local),
            "remote" => Some(Self::Remote),
            _ => None,
        }
    }
}

/// The storage port used by the client sync engine.
///
/// Local changes, revisions, and outbox rows are written atomically. Remote
/// applies and conflict records are also atomic. The port is implemented by
/// SQLCipher persistence and by the in-memory test ledger.
pub trait SyncRepository {
    /// Returns the highest applied server sequence.
    fn cursor(&self) -> Result<u64, RepositoryError>;

    /// Stores the highest applied server sequence.
    fn save_cursor(&self, seq: u64) -> Result<(), RepositoryError>;

    /// Returns pending outbox operations in creation order.
    fn pending_operations(&self, limit: usize) -> Result<Vec<SyncOperation>, RepositoryError>;

    /// Marks accepted operations with their server sequences.
    fn mark_operations_sent(&self, accepted: &[(Uuid, u64)]) -> Result<(), RepositoryError>;

    /// Returns whether an operation is already known locally.
    fn is_operation_known(&self, op_id: Uuid) -> Result<bool, RepositoryError>;

    /// Returns the entity revision, or `0` when the entity is unknown.
    fn entity_revision(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<u64, RepositoryError>;

    /// Returns the server sequence of the latest known version of the entity,
    /// or `0` when none is known.
    fn entity_server_seq(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<u64, RepositoryError>;

    /// Loads an entity aggregate.
    fn load_entity(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<Option<SyncEntity>, RepositoryError>;

    /// Atomically saves a local entity, enqueues its operation, and records the
    /// new revision.
    fn record_local_change(
        &self,
        entity: &SyncEntity,
        operation: &SyncOperation,
        revision: u64,
    ) -> Result<(), RepositoryError>;

    /// Atomically applies a remote entity, records the new revision, marks the
    /// operation applied, and stores a conflict when one was detected.
    fn apply_remote_change(
        &self,
        entity: &SyncEntity,
        operation: &SyncOperation,
        revision: u64,
        conflict: Option<&SyncConflict>,
    ) -> Result<(), RepositoryError>;

    /// Saves an entity and its revision without creating an operation, for
    /// snapshot restore. The server sequence marks the snapshot's position in
    /// the ledger log.
    fn restore_entity(
        &self,
        entity: &SyncEntity,
        revision: u64,
        server_seq: u64,
    ) -> Result<(), RepositoryError>;

    /// Returns every tracked entity revision, for snapshot creation.
    fn entity_revisions(&self) -> Result<Vec<(SyncEntityType, Uuid, u64)>, RepositoryError>;

    /// Returns conflicts in detection order.
    fn conflicts(&self) -> Result<Vec<SyncConflict>, RepositoryError>;

    /// Marks a conflict resolved.
    fn resolve_conflict(
        &self,
        conflict_id: Uuid,
        resolution: SyncConflictResolution,
        at: Timestamp,
    ) -> Result<(), RepositoryError>;
}
