//! Deterministic test doubles for application tests.

use std::cell::Cell;

use jiff::Timestamp;
use uuid::Uuid;

use crate::ports::{Clock, IdSource};

/// A clock that always returns the same instant.
pub(crate) struct FixedClock(Timestamp);

impl FixedClock {
    /// Creates the clock.
    pub(crate) fn new(instant: Timestamp) -> Self {
        Self(instant)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

/// An identifier source that returns increasing UUIDs from one.
pub(crate) struct SequenceIds {
    next: Cell<u128>,
}

impl SequenceIds {
    /// Creates the source.
    pub(crate) fn new() -> Self {
        Self { next: Cell::new(1) }
    }
}

impl IdSource for SequenceIds {
    fn next_uuid(&self) -> Uuid {
        let value = self.next.get();
        self.next.set(value.saturating_add(1));
        Uuid::from_u128(value)
    }
}
