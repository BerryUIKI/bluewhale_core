//! Encrypted ledger snapshots: creation, verification, and restore.
//!
//! A snapshot is a consistent export document plus its entity revisions,
//! encrypted with the ledger data key and hashed with SHA-256. The server
//! stores the blob and metadata; it cannot decrypt it. Restore imports the
//! entities into a fresh ledger database and sets the cursor to the snapshot's
//! server sequence.

use std::collections::BTreeMap;

use core_application::{
    EXPORT_VERSION, ExportDocument, ExportLedgerUseCase, entities_from_document,
};
use core_crypto::aead;
use core_domain::{LedgerExportRepository, SyncEntityType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::engine::{SyncEngine, SyncError};

/// The snapshot payload schema version.
pub const SNAPSHOT_VERSION: u32 = 1;

const SNAPSHOT_FORMAT_VERSION: u8 = 1;
const SNAPSHOT_AAD_DOMAIN: &[u8] = b"bluewhale/snapshot/v1";

/// Snapshot metadata stored by the server and used to verify the blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMetadata {
    /// The ledger sequence the snapshot covers.
    pub server_seq: u64,
    /// The ledger data-key version.
    pub key_version: u32,
    /// The SHA-256 of the encrypted blob.
    pub sha256: [u8; 32],
    /// The encrypted blob size in bytes.
    pub size: u64,
}

impl SnapshotMetadata {
    /// Returns the SHA-256 as lowercase hexadecimal.
    #[must_use]
    pub fn sha256_hex(&self) -> String {
        hex(&self.sha256)
    }
}

/// An encrypted snapshot ready for upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedSnapshot {
    /// The metadata to register with the server.
    pub metadata: SnapshotMetadata,
    /// The encrypted blob.
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotPayload {
    snapshot_version: u32,
    ledger_id: Uuid,
    server_seq: u64,
    schema_version: u32,
    document: ExportDocument,
    revisions: Vec<SnapshotRevision>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotRevision {
    entity_type: String,
    entity_id: Uuid,
    revision: u64,
}

impl SyncEngine<'_> {
    /// Creates an encrypted snapshot at the current cursor.
    ///
    /// The caller supplies the local schema version; it is recorded in the
    /// snapshot and checked on restore.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] when the export fails, a key is missing, a
    /// cryptographic operation fails, or the store fails.
    pub fn create_snapshot(
        &self,
        source: &dyn LedgerExportRepository,
        app_version: &str,
        schema_version: u32,
    ) -> Result<EncryptedSnapshot, SyncError> {
        if !self.store().pending_operations(1)?.is_empty() {
            return Err(SyncError::Snapshot("snapshot requires a drained outbox"));
        }
        let server_seq = self.store().cursor()?;
        let document = ExportLedgerUseCase::new(source, self.clock()).export(app_version)?;
        let revisions = self
            .store()
            .entity_revisions()?
            .into_iter()
            .map(|(entity_type, entity_id, revision)| SnapshotRevision {
                entity_type: entity_type.as_str().to_string(),
                entity_id,
                revision,
            })
            .collect();
        let payload = SnapshotPayload {
            snapshot_version: SNAPSHOT_VERSION,
            ledger_id: self.keys().ledger_id,
            server_seq,
            schema_version,
            document,
            revisions,
        };
        let plaintext = serde_json::to_vec(&payload)
            .map_err(|_| SyncError::Snapshot("snapshot serialization failed"))?;

        let key_version = self.keys().current_key_version;
        let key = self
            .keys()
            .ledger_keys
            .get(&key_version.get())
            .ok_or(SyncError::UnknownKeyVersion(key_version.get()))?;
        let associated_data = associated_data(self.keys().ledger_id, server_seq, key_version.get());
        let (nonce, ciphertext) = aead::seal_with_random_nonce(key, &associated_data, &plaintext)?;

        let mut bytes = Vec::with_capacity(1 + nonce.len() + ciphertext.len());
        bytes.push(SNAPSHOT_FORMAT_VERSION);
        bytes.extend_from_slice(&nonce);
        bytes.extend_from_slice(&ciphertext);
        let sha256 = digest(&bytes);
        Ok(EncryptedSnapshot {
            metadata: SnapshotMetadata {
                server_seq,
                key_version: key_version.get(),
                sha256,
                size: bytes.len() as u64,
            },
            bytes,
        })
    }

    /// Verifies, decrypts, and restores a snapshot into the local store.
    ///
    /// The store should be empty or freshly initialized. The cursor is set to
    /// the snapshot sequence after every entity is restored.
    ///
    /// # Errors
    ///
    /// Returns a [`SyncError`] for a hash, size, format, key, decryption,
    /// payload, schema, ledger, or store failure.
    pub fn restore_snapshot(
        &self,
        metadata: &SnapshotMetadata,
        bytes: &[u8],
        schema_version: u32,
    ) -> Result<u64, SyncError> {
        if metadata.size != bytes.len() as u64 {
            return Err(SyncError::Snapshot("snapshot size mismatch"));
        }
        if digest(bytes) != metadata.sha256 {
            return Err(SyncError::Snapshot("snapshot hash mismatch"));
        }
        if bytes.len() < 1 + aead::NONCE_LEN + aead::TAG_LEN || bytes[0] != SNAPSHOT_FORMAT_VERSION
        {
            return Err(SyncError::Snapshot("unsupported snapshot format"));
        }
        let nonce: [u8; aead::NONCE_LEN] = bytes[1..1 + aead::NONCE_LEN]
            .try_into()
            .map_err(|_| SyncError::Snapshot("invalid snapshot nonce"))?;
        let ciphertext = &bytes[1 + aead::NONCE_LEN..];

        let key = self
            .keys()
            .ledger_keys
            .get(&metadata.key_version)
            .ok_or(SyncError::UnknownKeyVersion(metadata.key_version))?;
        let associated_data = associated_data(
            self.keys().ledger_id,
            metadata.server_seq,
            metadata.key_version,
        );
        let plaintext = aead::open(key, &nonce, &associated_data, ciphertext)?;
        let payload: SnapshotPayload = serde_json::from_slice(&plaintext)
            .map_err(|_| SyncError::Snapshot("snapshot payload is invalid"))?;

        if payload.snapshot_version != SNAPSHOT_VERSION {
            return Err(SyncError::Snapshot("unsupported snapshot version"));
        }
        if payload.ledger_id != self.keys().ledger_id {
            return Err(SyncError::Snapshot("snapshot belongs to another ledger"));
        }
        if payload.document.export_version != EXPORT_VERSION
            || payload.document.ledger_id != payload.ledger_id.to_string()
        {
            return Err(SyncError::Snapshot(
                "snapshot document does not match its ledger",
            ));
        }
        if payload.server_seq != metadata.server_seq {
            return Err(SyncError::Snapshot(
                "snapshot sequence does not match its metadata",
            ));
        }
        if payload.schema_version != schema_version {
            return Err(SyncError::Snapshot("unsupported local schema version"));
        }

        let revisions: BTreeMap<(SyncEntityType, Uuid), u64> = payload
            .revisions
            .into_iter()
            .map(|revision| {
                let entity_type = SyncEntityType::parse(&revision.entity_type)
                    .ok_or(SyncError::Snapshot("unknown snapshot entity type"))?;
                Ok(((entity_type, revision.entity_id), revision.revision))
            })
            .collect::<Result<_, SyncError>>()?;

        for entity in entities_from_document(&payload.document)? {
            let revision = revisions
                .get(&(entity.entity_type(), entity.entity_id()))
                .copied()
                .unwrap_or(0);
            self.store()
                .restore_entity(&entity, revision, payload.server_seq)?;
        }
        self.store().save_cursor(payload.server_seq)?;
        Ok(payload.server_seq)
    }
}

fn associated_data(ledger_id: Uuid, server_seq: u64, key_version: u32) -> Vec<u8> {
    let mut associated_data = Vec::with_capacity(SNAPSHOT_AAD_DOMAIN.len() + 16 + 8 + 4);
    associated_data.extend_from_slice(SNAPSHOT_AAD_DOMAIN);
    associated_data.extend_from_slice(ledger_id.as_bytes());
    associated_data.extend_from_slice(&server_seq.to_be_bytes());
    associated_data.extend_from_slice(&key_version.to_be_bytes());
    associated_data
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}
