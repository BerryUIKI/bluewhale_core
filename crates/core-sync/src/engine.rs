//! The client sync engine: outbox, push/pull, revisions, and conflicts.
//!
//! Local changes are recorded atomically with their outbox operation and
//! revision. Push drains pending operations; pull verifies and decrypts each
//! operation, applies it with server-order and revision checks, records
//! conflicts, and advances the cursor.
//!
//! The engine is transport-neutral: [`SyncTransport`] is implemented by the
//! HTTP adapter in the clients and by the in-memory test server.

use std::collections::BTreeMap;
use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use core_application::{
    ApplicationError, Clock, IdSource, SyncPayloadError, decode_entity, encode_entity,
};
use core_crypto::{
    CryptoError, DeviceSignature, DeviceSigningKey, DeviceVerifyingKey, KeyVersion, SymmetricKey,
    aead,
};
use core_domain::{
    Account, RepositoryError, SyncConflict, SyncConflictResolution, SyncEntity, SyncEntityType,
    SyncOperation, SyncOperationType, SyncRepository, Transaction,
};
use uuid::Uuid;

use crate::api::{BatchError, PullResponse, PushRequest, PushResponse};
use crate::envelope::{EntityType, EnvelopeError, OperationEnvelope, OperationType};
use crate::{
    MAX_BATCH_BYTES, MAX_OPERATION_BYTES, MAX_OPERATIONS_PER_BATCH, PAYLOAD_VERSION,
    PROTOCOL_VERSION,
};

/// The device signing keys, verified device keys, and ledger data keys used by
/// the engine.
#[derive(Debug)]
pub struct SyncKeys {
    /// The ledger identity.
    pub ledger_id: Uuid,
    /// This device's identity.
    pub device_id: Uuid,
    /// This device's Ed25519 signing key.
    pub signing_key: DeviceSigningKey,
    /// Every authorized device's verifying key, including this device's.
    pub verifying_keys: BTreeMap<Uuid, DeviceVerifyingKey>,
    /// Ledger data keys by key version.
    pub ledger_keys: BTreeMap<u32, SymmetricKey>,
    /// The key version used for new operations.
    pub current_key_version: KeyVersion,
}

/// A transport failure. Retryable failures leave the local state untouched and
/// the operation can be retried later with the same envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncTransportError {
    message: String,
    retryable: bool,
}

impl SyncTransportError {
    /// Creates a transport error.
    #[must_use]
    pub fn new(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            retryable,
        }
    }

    /// Returns the message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns whether the request may be retried.
    #[must_use]
    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for SyncTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SyncTransportError {}

/// The operations the engine needs from a server transport.
pub trait SyncTransport {
    /// Pushes one batch.
    ///
    /// # Errors
    ///
    /// Returns [`SyncTransportError`] for network, authentication, or API
    /// failures.
    fn push(
        &self,
        ledger_id: Uuid,
        request: &PushRequest,
    ) -> Result<PushResponse, SyncTransportError>;

    /// Pulls operations after a sequence.
    ///
    /// # Errors
    ///
    /// Returns [`SyncTransportError`] for network, authentication, or API
    /// failures.
    fn pull(
        &self,
        ledger_id: Uuid,
        after_seq: u64,
        limit: u32,
    ) -> Result<PullResponse, SyncTransportError>;

    /// Stores the device cursor.
    ///
    /// # Errors
    ///
    /// Returns [`SyncTransportError`] for network, authentication, or API
    /// failures.
    fn acknowledge(&self, ledger_id: Uuid, seq: u64) -> Result<(), SyncTransportError>;
}

/// A sync engine failure.
#[derive(Debug)]
pub enum SyncError {
    /// The local sync store failed.
    Repository(RepositoryError),
    /// A payload could not be encoded or decoded.
    Payload(SyncPayloadError),
    /// A cryptographic operation failed.
    Crypto(CryptoError),
    /// An envelope failed protocol validation.
    Envelope(EnvelopeError),
    /// The transport failed.
    Transport(SyncTransportError),
    /// A batch failed protocol validation.
    Batch(BatchError),
    /// The application export use case failed.
    Application(ApplicationError),
    /// The operation's device has no known verifying key.
    UnknownDevice(Uuid),
    /// The operation's key version is not available locally.
    UnknownKeyVersion(u32),
    /// The revision chain has a gap for the entity.
    RevisionGap {
        /// The entity kind.
        entity_type: SyncEntityType,
        /// The entity identity.
        entity_id: Uuid,
        /// The local revision.
        local_revision: u64,
        /// The remote base revision.
        remote_base_revision: u64,
    },
    /// An operation or entity identifier is invalid.
    InvalidIdentifier(&'static str),
    /// A binary field is not canonical base64url of the required length.
    InvalidEncoding(&'static str),
    /// The client clock returned a negative epoch time.
    InvalidTimestamp,
    /// The operation type is not valid for the entity.
    InvalidOperation(&'static str),
    /// The payload did not describe the envelope's entity.
    InvalidPayload(&'static str),
    /// One operation exceeds the protocol size limit.
    OperationTooLarge {
        /// The serialized size.
        size: usize,
        /// The maximum size.
        maximum: usize,
    },
    /// The batch exceeds the protocol size limit.
    BatchTooLarge {
        /// The serialized size.
        size: usize,
        /// The maximum size.
        maximum: usize,
    },
    /// The server response violates the protocol.
    Protocol(&'static str),
    /// The conflict does not exist.
    ConflictNotFound(Uuid),
    /// The conflict is already resolved.
    ConflictAlreadyResolved(Uuid),
    /// A snapshot failed validation.
    Snapshot(&'static str),
}

impl fmt::Display for SyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(error) => write!(formatter, "repository: {error}"),
            Self::Payload(error) => write!(formatter, "payload: {error}"),
            Self::Crypto(error) => write!(formatter, "crypto: {error}"),
            Self::Envelope(error) => write!(formatter, "envelope: {error}"),
            Self::Transport(error) => write!(formatter, "transport: {error}"),
            Self::Batch(error) => write!(formatter, "batch: {error}"),
            Self::Application(error) => write!(formatter, "application: {error}"),
            Self::UnknownDevice(device_id) => write!(formatter, "unknown device: {device_id}"),
            Self::UnknownKeyVersion(version) => write!(formatter, "unknown key version: {version}"),
            Self::RevisionGap {
                entity_type,
                entity_id,
                local_revision,
                remote_base_revision,
            } => write!(
                formatter,
                "revision gap for {entity_type} {entity_id}: local {local_revision}, remote base {remote_base_revision}"
            ),
            Self::InvalidIdentifier(field) => write!(formatter, "invalid identifier: {field}"),
            Self::InvalidEncoding(field) => write!(formatter, "invalid encoding: {field}"),
            Self::InvalidTimestamp => formatter.write_str("invalid client timestamp"),
            Self::InvalidOperation(message) => write!(formatter, "invalid operation: {message}"),
            Self::InvalidPayload(message) => write!(formatter, "invalid payload: {message}"),
            Self::OperationTooLarge { size, maximum } => {
                write!(formatter, "operation is {size} bytes; maximum {maximum}")
            }
            Self::BatchTooLarge { size, maximum } => {
                write!(formatter, "batch is {size} bytes; maximum {maximum}")
            }
            Self::Protocol(message) => write!(formatter, "protocol: {message}"),
            Self::ConflictNotFound(id) => write!(formatter, "conflict not found: {id}"),
            Self::ConflictAlreadyResolved(id) => {
                write!(formatter, "conflict already resolved: {id}")
            }
            Self::Snapshot(message) => write!(formatter, "snapshot: {message}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<RepositoryError> for SyncError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

impl From<SyncPayloadError> for SyncError {
    fn from(error: SyncPayloadError) -> Self {
        Self::Payload(error)
    }
}

impl From<CryptoError> for SyncError {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<EnvelopeError> for SyncError {
    fn from(error: EnvelopeError) -> Self {
        Self::Envelope(error)
    }
}

impl From<SyncTransportError> for SyncError {
    fn from(error: SyncTransportError) -> Self {
        Self::Transport(error)
    }
}

impl From<BatchError> for SyncError {
    fn from(error: BatchError) -> Self {
        Self::Batch(error)
    }
}

impl From<ApplicationError> for SyncError {
    fn from(error: ApplicationError) -> Self {
        Self::Application(error)
    }
}

/// The number of operations accepted by one push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PushOutcome {
    /// Operations pushed.
    pub pushed: usize,
    /// The server's latest sequence.
    pub latest_seq: u64,
}

/// The result of one pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PullOutcome {
    /// Operations applied.
    pub applied: usize,
    /// Operations skipped because they were already reflected locally.
    pub skipped: usize,
    /// Conflict records created.
    pub conflicts: usize,
}

impl PullOutcome {
    fn merge(&mut self, other: Self) {
        self.applied += other.applied;
        self.skipped += other.skipped;
        self.conflicts += other.conflicts;
    }
}

/// The result of a full sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncOutcome {
    /// The push result.
    pub push: PushOutcome,
    /// The merged pull result.
    pub pull: PullOutcome,
    /// The cursor after the sync.
    pub cursor: u64,
}

/// The client sync engine.
pub struct SyncEngine<'a> {
    store: &'a dyn SyncRepository,
    keys: SyncKeys,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> SyncEngine<'a> {
    /// Creates an engine and checks that its current key version and own
    /// verifying key are available.
    ///
    /// # Errors
    ///
    /// Returns [`SyncError::UnknownKeyVersion`] or
    /// [`SyncError::UnknownDevice`] when the key material is incomplete.
    pub fn new(
        store: &'a dyn SyncRepository,
        keys: SyncKeys,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Result<Self, SyncError> {
        if !keys
            .ledger_keys
            .contains_key(&keys.current_key_version.get())
        {
            return Err(SyncError::UnknownKeyVersion(keys.current_key_version.get()));
        }
        if !keys.verifying_keys.contains_key(&keys.device_id) {
            return Err(SyncError::UnknownDevice(keys.device_id));
        }
        Ok(Self {
            store,
            keys,
            clock,
            ids,
        })
    }

    /// Returns the local sync store.
    #[must_use]
    pub fn store(&self) -> &dyn SyncRepository {
        self.store
    }

    pub(crate) fn keys(&self) -> &SyncKeys {
        &self.keys
    }

    pub(crate) fn clock(&self) -> &dyn Clock {
        self.clock
    }

    /// Returns the local cursor.
    ///
    /// # Errors
    ///
    /// Returns [`SyncError::Repository`] when the store fails.
    pub fn cursor(&self) -> Result<u64, SyncError> {
        Ok(self.store.cursor()?)
    }

    /// Records a local entity change, encrypts and signs it, and enqueues it in
    /// the outbox atomically with the entity and its new revision.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] for an invalid operation, missing key material,
    /// encoding or cryptographic failure, or a store failure.
    pub fn record_local_change(
        &self,
        entity: &SyncEntity,
        op_type: SyncOperationType,
    ) -> Result<SyncOperation, SyncError> {
        validate_operation_for_entity(entity, op_type)?;
        let entity_type = entity.entity_type();
        let entity_id = entity.entity_id();
        let base_revision = self.store.entity_revision(entity_type, entity_id)?;
        let revision = base_revision
            .checked_add(1)
            .ok_or(SyncError::InvalidOperation("revision overflow"))?;

        let payload = encode_entity(entity)?;
        let key_version = self.keys.current_key_version;
        let key = self
            .keys
            .ledger_keys
            .get(&key_version.get())
            .ok_or(SyncError::UnknownKeyVersion(key_version.get()))?;
        let associated_data = operation_associated_data(
            self.keys.ledger_id,
            entity_type,
            entity_id,
            op_type,
            base_revision,
            key_version.get(),
        );
        let (nonce, ciphertext) = aead::seal_with_random_nonce(key, &associated_data, &payload)?;

        let op_id = self.next_operation_id()?;
        let idempotency_key = self.ids.next_uuid();
        let client_timestamp = self.clock.now().as_millisecond();
        if client_timestamp < 0 {
            return Err(SyncError::InvalidTimestamp);
        }

        let mut envelope = OperationEnvelope {
            protocol_version: PROTOCOL_VERSION,
            op_id,
            ledger_id: self.keys.ledger_id,
            device_id: self.keys.device_id,
            entity_type: EntityType::from_sync(entity_type),
            entity_id,
            op_type: OperationType::from_sync(op_type),
            base_revision,
            client_timestamp,
            idempotency_key,
            key_version: key_version.get(),
            nonce: encode_base64(&nonce),
            ciphertext: encode_base64(&ciphertext),
            payload_version: PAYLOAD_VERSION,
            signature: String::new(),
        };
        let signature = self.keys.signing_key.sign(&envelope.signing_bytes());
        envelope.signature = encode_base64(&signature.to_bytes());
        envelope.validate()?;

        let operation = envelope.to_sync_operation(None);
        self.store
            .record_local_change(entity, &operation, revision)?;
        Ok(operation)
    }

    /// Pushes one batch of pending outbox operations.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] for a store, protocol, size, or transport
    /// failure. A transport failure leaves the outbox intact.
    pub fn push(&self, transport: &dyn SyncTransport) -> Result<PushOutcome, SyncError> {
        let pending = self.store.pending_operations(MAX_OPERATIONS_PER_BATCH)?;
        if pending.is_empty() {
            return Ok(PushOutcome::default());
        }

        let mut envelopes = Vec::with_capacity(pending.len());
        for operation in &pending {
            let envelope = OperationEnvelope::from_sync_operation(operation);
            envelope.validate()?;
            let size = serialized_size(&envelope)?;
            if size > MAX_OPERATION_BYTES {
                return Err(SyncError::OperationTooLarge {
                    size,
                    maximum: MAX_OPERATION_BYTES,
                });
            }
            envelopes.push(envelope);
        }

        let request = PushRequest { ops: envelopes };
        request.validate()?;
        let batch_size = serialized_size(&request)?;
        if batch_size > MAX_BATCH_BYTES {
            return Err(SyncError::BatchTooLarge {
                size: batch_size,
                maximum: MAX_BATCH_BYTES,
            });
        }

        let response = transport.push(self.keys.ledger_id, &request)?;
        if response.accepted.len() != pending.len() {
            return Err(SyncError::Protocol(
                "push response does not accept every operation",
            ));
        }
        let mut accepted = Vec::with_capacity(pending.len());
        for (operation, accepted_operation) in pending.iter().zip(&response.accepted) {
            if accepted_operation.op_id != operation.op_id {
                return Err(SyncError::Protocol(
                    "push response order does not match the request",
                ));
            }
            if accepted_operation.seq == 0 {
                return Err(SyncError::Protocol("server sequence must be positive"));
            }
            accepted.push((operation.op_id, accepted_operation.seq));
        }
        self.store.mark_operations_sent(&accepted)?;
        Ok(PushOutcome {
            pushed: pending.len(),
            latest_seq: response.latest_seq,
        })
    }

    /// Pulls and applies one page of remote operations.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] for a store, protocol, verification,
    /// decryption, payload, or transport failure. The cursor only advances over
    /// operations that were applied or safely skipped.
    pub fn pull(
        &self,
        transport: &dyn SyncTransport,
        limit: u32,
    ) -> Result<PullOutcome, SyncError> {
        if limit == 0 || limit > 500 {
            return Err(SyncError::Protocol("pull limit must be 1-500"));
        }
        let cursor = self.store.cursor()?;
        let response = transport.pull(self.keys.ledger_id, cursor, limit)?;
        if response.ops.is_empty() && response.has_more {
            return Err(SyncError::Protocol(
                "server reported more operations but returned none",
            ));
        }

        let mut outcome = PullOutcome::default();
        for stored in &response.ops {
            if stored.seq == 0 {
                return Err(SyncError::Protocol("server sequence must be positive"));
            }
            if stored.seq <= self.store.cursor()? {
                return Err(SyncError::Protocol(
                    "server returned an operation at or before the cursor",
                ));
            }
            match self.apply_remote(stored)? {
                Applied::Applied { conflicts } => {
                    outcome.applied += 1;
                    outcome.conflicts += conflicts;
                }
                Applied::Skipped => outcome.skipped += 1,
            }
            self.store.save_cursor(stored.seq)?;
        }
        Ok(outcome)
    }

    /// Pushes pending work, pulls until the server has no more operations, and
    /// acknowledges the cursor.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] for a push, pull, or acknowledgment failure.
    pub fn sync(&self, transport: &dyn SyncTransport) -> Result<SyncOutcome, SyncError> {
        let mut push = PushOutcome::default();
        for _ in 0..10 {
            let outcome = self.push(transport)?;
            push.pushed += outcome.pushed;
            push.latest_seq = push.latest_seq.max(outcome.latest_seq);
            if outcome.pushed == 0 {
                break;
            }
        }
        let mut pull = PullOutcome::default();
        for _ in 0..100 {
            let outcome = self.pull(transport, 500)?;
            pull.merge(outcome);
            if outcome.applied == 0 && outcome.skipped == 0 {
                break;
            }
        }
        let cursor = self.store.cursor()?;
        if cursor > 0 {
            transport.acknowledge(self.keys.ledger_id, cursor)?;
        }
        Ok(SyncOutcome { push, pull, cursor })
    }

    /// Returns unresolved and resolved conflicts in detection order.
    ///
    /// # Errors
    ///
    /// Returns [`SyncError::Repository`] when the store fails.
    pub fn conflicts(&self) -> Result<Vec<SyncConflict>, SyncError> {
        Ok(self.store.conflicts()?)
    }

    /// Resolves a conflict.
    ///
    /// Choosing the local aggregate records a new local change on top of the
    /// current revision. Choosing the remote aggregate only marks the conflict
    /// resolved.
    ///
    /// # Errors
    ///
    /// Returns [`SyncError::ConflictNotFound`],
    /// [`SyncError::ConflictAlreadyResolved`], or a store, payload, or crypto
    /// failure.
    pub fn resolve_conflict(
        &self,
        conflict_id: Uuid,
        resolution: SyncConflictResolution,
    ) -> Result<Option<SyncOperation>, SyncError> {
        let conflict = self
            .store
            .conflicts()?
            .into_iter()
            .find(|conflict| conflict.id == conflict_id)
            .ok_or(SyncError::ConflictNotFound(conflict_id))?;
        if conflict.resolution.is_some() {
            return Err(SyncError::ConflictAlreadyResolved(conflict_id));
        }
        let now = self.clock.now();
        if resolution == SyncConflictResolution::Remote {
            self.store.resolve_conflict(conflict_id, resolution, now)?;
            return Ok(None);
        }

        let entity = decode_entity(conflict.entity_type, &conflict.local_payload)?;
        let op_type = natural_operation(&entity);
        let operation = self.record_local_change(&entity, op_type)?;
        self.store.resolve_conflict(conflict_id, resolution, now)?;
        Ok(Some(operation))
    }

    fn next_operation_id(&self) -> Result<Uuid, SyncError> {
        let op_id = self.ids.next_uuid();
        if op_id.get_version_num() != 7 {
            return Err(SyncError::InvalidIdentifier("op_id"));
        }
        Ok(op_id)
    }

    fn apply_remote(&self, stored: &crate::api::StoredOperation) -> Result<Applied, SyncError> {
        let envelope = &stored.envelope;
        envelope.validate()?;
        let verifying_key = self
            .keys
            .verifying_keys
            .get(&envelope.device_id)
            .ok_or(SyncError::UnknownDevice(envelope.device_id))?;
        let signature_bytes = decode_base64_array::<64>(&envelope.signature, "signature")?;
        let signature = DeviceSignature::from_bytes(&signature_bytes);
        verifying_key.verify(&envelope.signing_bytes(), &signature)?;

        let entity_type = envelope.entity_type.to_sync();
        let last_server_seq = self
            .store
            .entity_server_seq(entity_type, envelope.entity_id)?;
        let is_own = envelope.device_id == self.keys.device_id;

        if stored.seq < last_server_seq {
            return Ok(Applied::Skipped);
        }
        if stored.seq == last_server_seq && !is_own {
            return Ok(Applied::Skipped);
        }

        let key = self
            .keys
            .ledger_keys
            .get(&envelope.key_version)
            .ok_or(SyncError::UnknownKeyVersion(envelope.key_version))?;
        let nonce = decode_base64_array::<12>(&envelope.nonce, "nonce")?;
        let ciphertext = decode_base64(&envelope.ciphertext, "ciphertext")?;
        let associated_data = operation_associated_data(
            envelope.ledger_id,
            entity_type,
            envelope.entity_id,
            envelope.op_type.to_sync(),
            envelope.base_revision,
            envelope.key_version,
        );
        let plaintext = aead::open(key, &nonce, &associated_data, &ciphertext)?;
        let remote_entity = decode_entity(entity_type, &plaintext)?;
        if remote_entity.entity_type() != entity_type
            || remote_entity.entity_id() != envelope.entity_id
        {
            return Err(SyncError::InvalidPayload(
                "payload does not match the envelope entity",
            ));
        }

        let local_revision = self
            .store
            .entity_revision(entity_type, envelope.entity_id)?;
        let local_entity = self.store.load_entity(entity_type, envelope.entity_id)?;

        if is_own {
            let remote_revision = envelope
                .base_revision
                .checked_add(1)
                .ok_or(SyncError::InvalidOperation("revision overflow"))?;
            let unchanged = local_entity.as_ref() == Some(&remote_entity);
            let operation = envelope.to_sync_operation(Some(stored.seq));
            self.store
                .apply_remote_change(&remote_entity, &operation, remote_revision, None)?;
            let now = self.clock.now();
            for conflict in self.store.conflicts()? {
                if conflict.resolution.is_none()
                    && conflict.entity_type == entity_type
                    && conflict.entity_id == envelope.entity_id
                {
                    self.store
                        .resolve_conflict(conflict.id, SyncConflictResolution::Local, now)?;
                }
            }
            return Ok(if unchanged {
                Applied::Skipped
            } else {
                Applied::Applied { conflicts: 0 }
            });
        }

        if envelope.base_revision > local_revision {
            return Err(SyncError::RevisionGap {
                entity_type,
                entity_id: envelope.entity_id,
                local_revision,
                remote_base_revision: envelope.base_revision,
            });
        }

        if envelope.base_revision == local_revision {
            let revision = envelope
                .base_revision
                .checked_add(1)
                .ok_or(SyncError::InvalidOperation("revision overflow"))?;
            let operation = envelope.to_sync_operation(Some(stored.seq));
            self.store
                .apply_remote_change(&remote_entity, &operation, revision, None)?;
            return Ok(Applied::Applied { conflicts: 0 });
        }

        let Some(local_entity) = local_entity else {
            return Err(SyncError::InvalidPayload(
                "local entity is missing for a concurrent revision",
            ));
        };
        let entity_to_apply = merge_concurrent(remote_entity, &local_entity)?;
        let conflict = if is_financial_conflict(entity_type) {
            let local_payload = encode_entity(&local_entity)?;
            Some(SyncConflict {
                id: self.ids.next_uuid(),
                entity_type,
                entity_id: envelope.entity_id,
                local_payload,
                remote_op_id: envelope.op_id,
                local_revision,
                remote_base_revision: envelope.base_revision,
                detected_at: self.clock.now(),
                resolution: None,
                resolved_at: None,
            })
        } else {
            None
        };
        let revision = envelope
            .base_revision
            .checked_add(1)
            .ok_or(SyncError::InvalidOperation("revision overflow"))?;
        let operation = envelope.to_sync_operation(Some(stored.seq));
        self.store.apply_remote_change(
            &entity_to_apply,
            &operation,
            revision,
            conflict.as_ref(),
        )?;
        Ok(Applied::Applied {
            conflicts: usize::from(conflict.is_some()),
        })
    }
}

enum Applied {
    Applied { conflicts: usize },
    Skipped,
}

fn validate_operation_for_entity(
    entity: &SyncEntity,
    op_type: SyncOperationType,
) -> Result<(), SyncError> {
    match entity {
        SyncEntity::LedgerSettings(_) => {
            if op_type == SyncOperationType::SettingsReplace {
                Ok(())
            } else {
                Err(SyncError::InvalidOperation(
                    "ledger settings require settings_replace",
                ))
            }
        }
        SyncEntity::LedgerProfile(_) => {
            if op_type == SyncOperationType::ProfileReplace {
                Ok(())
            } else {
                Err(SyncError::InvalidOperation(
                    "ledger profile requires profile_replace",
                ))
            }
        }
        SyncEntity::ExchangeRate(_) => {
            if op_type == SyncOperationType::RateUpsert {
                Ok(())
            } else {
                Err(SyncError::InvalidOperation(
                    "exchange rate requires rate_upsert",
                ))
            }
        }
        SyncEntity::Transaction(transaction) => {
            if op_type == SyncOperationType::Delete {
                if transaction.is_deleted() {
                    Ok(())
                } else {
                    Err(SyncError::InvalidOperation(
                        "delete requires a soft-deleted transaction",
                    ))
                }
            } else if transaction.is_deleted() {
                Err(SyncError::InvalidOperation(
                    "a deleted transaction requires the delete operation",
                ))
            } else {
                Ok(())
            }
        }
        _ => {
            if op_type == SyncOperationType::Upsert {
                Ok(())
            } else {
                Err(SyncError::InvalidOperation(
                    "entity requires the upsert operation",
                ))
            }
        }
    }
}

fn natural_operation(entity: &SyncEntity) -> SyncOperationType {
    match entity {
        SyncEntity::LedgerSettings(_) => SyncOperationType::SettingsReplace,
        SyncEntity::LedgerProfile(_) => SyncOperationType::ProfileReplace,
        SyncEntity::ExchangeRate(_) => SyncOperationType::RateUpsert,
        SyncEntity::Transaction(transaction) if transaction.is_deleted() => {
            SyncOperationType::Delete
        }
        _ => SyncOperationType::Upsert,
    }
}

fn is_financial_conflict(entity_type: SyncEntityType) -> bool {
    entity_type == SyncEntityType::Transaction
}

fn merge_concurrent(remote: SyncEntity, local: &SyncEntity) -> Result<SyncEntity, SyncError> {
    match (remote, local) {
        (SyncEntity::Account(remote), SyncEntity::Account(local)) => {
            let mut options = remote.options().clone();
            options
                .tag_ids
                .extend(local.options().tag_ids.iter().copied());
            Account::new(
                remote.id(),
                remote.name(),
                remote.asset_type(),
                remote.currency().clone(),
                options,
                remote.created_at(),
                remote.updated_at(),
            )
            .map(SyncEntity::Account)
            .map_err(|_| SyncError::InvalidOperation("merged account is invalid"))
        }
        (SyncEntity::Transaction(remote), SyncEntity::Transaction(local)) => {
            let mut fields = remote.fields().clone();
            fields
                .tag_ids
                .extend(local.fields().tag_ids.iter().copied());
            Transaction::new(
                remote.id(),
                remote.kind(),
                remote.amount().clone(),
                remote.occurred_at(),
                remote.account_id(),
                fields,
                remote.created_at(),
                remote.updated_at(),
            )
            .map(SyncEntity::Transaction)
            .map_err(|_| SyncError::InvalidOperation("merged transaction is invalid"))
        }
        (remote, _) => Ok(remote),
    }
}

/// The AEAD associated data for an operation payload.
///
/// The envelope signature already covers these fields; binding them again in
/// the ciphertext prevents a valid payload from being moved to another
/// envelope.
fn operation_associated_data(
    ledger_id: Uuid,
    entity_type: SyncEntityType,
    entity_id: Uuid,
    op_type: SyncOperationType,
    base_revision: u64,
    key_version: u32,
) -> Vec<u8> {
    let mut associated_data = Vec::with_capacity(128);
    associated_data.extend_from_slice(b"bluewhale/operation-payload/v1");
    associated_data.extend_from_slice(ledger_id.as_bytes());
    associated_data.extend_from_slice(entity_type.as_str().as_bytes());
    associated_data.push(0);
    associated_data.extend_from_slice(entity_id.as_bytes());
    associated_data.extend_from_slice(op_type.as_str().as_bytes());
    associated_data.push(0);
    associated_data.extend_from_slice(&base_revision.to_be_bytes());
    associated_data.extend_from_slice(&key_version.to_be_bytes());
    associated_data
}

fn serialized_size<T: serde::Serialize>(value: &T) -> Result<usize, SyncError> {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .map_err(|_| SyncError::Protocol("serialization failed"))
}

fn encode_base64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_base64(value: &str, field: &'static str) -> Result<Vec<u8>, SyncError> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| SyncError::InvalidEncoding(field))
}

fn decode_base64_array<const N: usize>(
    value: &str,
    field: &'static str,
) -> Result<[u8; N], SyncError> {
    decode_base64(value, field)?
        .try_into()
        .map_err(|_| SyncError::InvalidEncoding(field))
}

/// Returns the SHA-256 digest of a byte slice as lowercase hexadecimal.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}
