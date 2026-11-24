//! Injected ports for time and identifiers.

use jiff::Timestamp;
use uuid::Uuid;

/// Supplies the current instant to a use case.
pub trait Clock {
    /// Returns the current UTC instant.
    fn now(&self) -> Timestamp;
}

/// Supplies new identifiers to a use case.
pub trait IdSource {
    /// Returns a new UUID. Clients provide UUIDv7 values.
    fn next_uuid(&self) -> Uuid;
}
