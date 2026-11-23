//! SQLite persistence for Bluewhale.
//!
//! Owns the local schema, forward-only migrations, and `rusqlite` repository
//! implementations. Depends on `core-domain`; `core-domain` must never depend
//! on this crate.
