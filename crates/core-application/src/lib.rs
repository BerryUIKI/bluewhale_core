//! Application use cases for Bluewhale.
//!
//! Orchestrates domain operations, transaction boundaries, and validation
//! flow. Depends on `core-domain`; persistence is injected through repository
//! traits so this crate stays independent of SQLite and of clients.
