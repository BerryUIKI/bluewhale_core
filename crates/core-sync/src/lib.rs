//! Sync protocol v1 types and validation.
//!
//! The public contract is `contracts/sync/v1`: `sync.schema.json` defines the
//! operation envelope and every HTTP document, `openapi.yaml` describes the
//! `/v1` API, and `fixtures/` holds conformance cases with a SHA-256 manifest.
//! The types in this crate mirror that contract.
//!
//! Scope: the operation envelope and the unencrypted HTTP documents. The
//! encrypted payload schema identified by `payload_version` and the
//! outbox/conflict engine land with the client sync engine; cryptography lands
//! with `core-crypto`. This crate performs no I/O.

pub mod api;
pub mod engine;
pub mod envelope;
pub mod snapshot;

pub use api::*;
pub use engine::{
    PullOutcome, PushOutcome, SyncEngine, SyncError, SyncKeys, SyncOutcome, SyncTransport,
    SyncTransportError, sha256_hex,
};
pub use envelope::*;
pub use snapshot::{EncryptedSnapshot, SNAPSHOT_VERSION, SnapshotMetadata};

/// The sync protocol version written and accepted by this build.
///
/// A consumer rejects envelopes and documents from versions it does not
/// support; a breaking change increments this value and publishes a new
/// contract directory.
pub const PROTOCOL_VERSION: u32 = 1;

/// The decrypted payload schema version used by protocol version 1.
pub const PAYLOAD_VERSION: u32 = 1;

/// The maximum number of operations in one push batch.
pub const MAX_OPERATIONS_PER_BATCH: usize = 500;

/// The maximum serialized size of one operation, in bytes.
pub const MAX_OPERATION_BYTES: usize = 1_048_576;

/// The maximum serialized size of one push batch, in bytes.
pub const MAX_BATCH_BYTES: usize = 8_388_608;

/// The largest integer permitted in a protocol document.
///
/// This is `2^53 - 1`, the exact integer range of IEEE 754 doubles and
/// therefore of RFC 8785 canonical JSON. Keeping every protocol integer within
/// this range avoids precision loss in the signature input and in consumers
/// that decode JSON numbers as doubles.
pub const MAX_PROTOCOL_INTEGER: u64 = 9_007_199_254_740_991;
