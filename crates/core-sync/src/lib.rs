//! Sync protocol types and operation-log handling for Bluewhale.
//!
//! Owns sync envelopes, cursors, conflict records, and the application of
//! remote operations to the local store. Financial conflicts are preserved
//! for review; they are never resolved by silent last-write-wins.
