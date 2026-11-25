//! SQLCipher connection setup, pragmas, and forward-only migrations.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use core_domain::RepositoryError;
use rusqlite::Connection;
use rusqlite::backup::Backup;

/// The current local schema version.
pub const SCHEMA_VERSION: i64 = 2;

const APPLICATION_ID: i64 = 0x4257_4831;
const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial.sql");
const SYNC_SCHEMA: &str = include_str!("../migrations/0002_sync.sql");

/// Forward-only migrations in version order.
const MIGRATIONS: &[(i64, &str)] = &[(1, INITIAL_SCHEMA), (2, SYNC_SCHEMA)];

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
        initialize(&connection, key, Some(path))?;
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
        initialize(&connection, key, None)?;
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

fn initialize(
    connection: &Connection,
    key: &str,
    path: Option<&Path>,
) -> Result<(), RepositoryError> {
    if key.is_empty() {
        return Err(RepositoryError::new("database key must not be empty"));
    }
    apply_key(connection, key)?;
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
    migrate(connection, key, path)
}

fn apply_key(connection: &Connection, key: &str) -> Result<(), RepositoryError> {
    let escaped = key.replace('\'', "''");
    connection
        .execute_batch(&format!("PRAGMA key = '{escaped}';"))
        .map_err(to_repository_error)
}

fn migrate(connection: &Connection, key: &str, path: Option<&Path>) -> Result<(), RepositoryError> {
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
        if current > 0
            && let Some(path) = path
        {
            backup_before_migration(connection, path, current, key)?;
        }
        for (version, schema) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
            let transaction = connection
                .unchecked_transaction()
                .map_err(to_repository_error)?;
            transaction
                .execute_batch(schema)
                .map_err(to_repository_error)?;
            transaction
                .execute(
                    "INSERT INTO schema_migration (version, applied_at) VALUES (?1, ?2)",
                    rusqlite::params![version, now_milliseconds()],
                )
                .map_err(to_repository_error)?;
            transaction.commit().map_err(to_repository_error)?;
        }
    }
    Ok(())
}

/// Copies a consistent database image beside the original and verifies it
/// before any migration runs.
///
/// The backup path is `<database>.backup-v<version>`. A failed attempt removes
/// the partial file; a successful backup is kept after the migration.
fn backup_before_migration(
    source: &Connection,
    path: &Path,
    version: i64,
    key: &str,
) -> Result<(), RepositoryError> {
    let backup_path = PathBuf::from(format!("{}.backup-v{version}", path.display()));
    let created = !backup_path.exists();
    if created {
        let result = copy_database(source, &backup_path, key);
        if result.is_err() {
            let _ = fs::remove_file(&backup_path);
        }
        result?;
    }
    verify_database(&backup_path, key)
}

fn copy_database(
    source: &Connection,
    backup_path: &Path,
    key: &str,
) -> Result<(), RepositoryError> {
    let mut destination = Connection::open(backup_path).map_err(to_repository_error)?;
    apply_key(&destination, key)?;
    let backup = Backup::new(source, &mut destination).map_err(to_repository_error)?;
    backup
        .run_to_completion(32, Duration::from_millis(10), None)
        .map_err(to_repository_error)
}

fn verify_database(path: &Path, key: &str) -> Result<(), RepositoryError> {
    let connection = Connection::open(path).map_err(to_repository_error)?;
    apply_key(&connection, key)?;
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(to_repository_error)?;
    if integrity == "ok" {
        Ok(())
    } else {
        Err(RepositoryError::new(format!(
            "migration backup failed integrity check: {integrity}"
        )))
    }
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
