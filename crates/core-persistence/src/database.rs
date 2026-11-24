//! SQLCipher connection setup, pragmas, and migrations.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use core_domain::RepositoryError;
use rusqlite::Connection;

/// The current local schema version.
pub const SCHEMA_VERSION: i64 = 1;

const APPLICATION_ID: i64 = 0x4257_4831;
const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial.sql");

/// A SQLCipher database guarded for use by the repository implementation.
#[derive(Debug)]
pub struct LedgerDatabase {
    connection: Mutex<Connection>,
}

impl LedgerDatabase {
    /// Opens or creates an encrypted ledger database.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] when the key is empty, the database
    /// cannot be opened, the key is wrong, or a migration fails.
    pub fn open(path: &Path, key: &str) -> Result<Self, RepositoryError> {
        let connection = Connection::open(path).map_err(to_repository_error)?;
        initialize(&connection, key)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Opens an encrypted in-memory ledger database.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] when the key is empty or initialization
    /// fails.
    pub fn open_in_memory(key: &str) -> Result<Self, RepositoryError> {
        let connection = Connection::open_in_memory().map_err(to_repository_error)?;
        initialize(&connection, key)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Locks the connection for one repository operation.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] when the lock is poisoned.
    pub fn connection(&self) -> Result<MutexGuard<'_, Connection>, RepositoryError> {
        self.connection
            .lock()
            .map_err(|_| RepositoryError::new("database lock poisoned"))
    }

    /// Returns the applied schema version.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] when the query fails.
    pub fn schema_version(&self) -> Result<i64, RepositoryError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migration",
                [],
                |row| row.get(0),
            )
            .map_err(to_repository_error)
    }
}

fn initialize(connection: &Connection, key: &str) -> Result<(), RepositoryError> {
    if key.is_empty() {
        return Err(RepositoryError::new("database key must not be empty"));
    }
    let escaped = key.replace('\'', "''");
    connection
        .execute_batch(&format!("PRAGMA key = '{escaped}';"))
        .map_err(to_repository_error)?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(to_repository_error)?;
    connection
        .pragma_update(None, "application_id", APPLICATION_ID)
        .map_err(to_repository_error)?;
    migrate(connection)
}

fn migrate(connection: &Connection) -> Result<(), RepositoryError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migration (
                 version INTEGER PRIMARY KEY,
                 applied_at INTEGER NOT NULL
             );",
        )
        .map_err(to_repository_error)?;
    let current: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migration",
            [],
            |row| row.get(0),
        )
        .map_err(to_repository_error)?;
    if current < SCHEMA_VERSION {
        let transaction = connection
            .unchecked_transaction()
            .map_err(to_repository_error)?;
        transaction
            .execute_batch(INITIAL_SCHEMA)
            .map_err(to_repository_error)?;
        transaction
            .execute(
                "INSERT INTO schema_migration (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![SCHEMA_VERSION, now_milliseconds()],
            )
            .map_err(to_repository_error)?;
        transaction.commit().map_err(to_repository_error)?;
    }
    Ok(())
}

fn now_milliseconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Converts a rusqlite error into the domain repository error.
pub(crate) fn to_repository_error(error: rusqlite::Error) -> RepositoryError {
    RepositoryError::new(error.to_string())
}
