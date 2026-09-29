//! SQLite store: one shared connection behind a mutex, plus embedded migrations.
//!
//! Spec §9 requires a single write queue and WAL, and spec §11 requires foreign
//! keys. This store keeps exactly one `Connection` behind `Arc<Mutex<..>>` and
//! shares it across the Project and Job repositories, so every write is
//! serialized. Per the ticket decision, reads go through the same connection in
//! Gate 1: short read-only connections arrive with the local API in Gate 2, and
//! speculating them now would violate the "no abstraction without a second case"
//! rule.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::Connection;

/// Embedded schema migrations, applied in version order.
///
/// Each file is named `NNNN_description.sql`; the leading number is the
/// `schema_migrations.version`. The top-level `migrations/` tree remains
/// reserved for a future migration tool and is intentionally not read here.
pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("migrations/0001_create_projects.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("migrations/0002_create_jobs.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("migrations/0003_create_captures.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("migrations/0004_create_adapter_checkpoints.sql"),
    },
];

/// A single embedded schema migration.
pub(crate) struct Migration {
    /// Monotonic version recorded in `schema_migrations`.
    pub(crate) version: i64,
    /// DDL applied exactly once, in a transaction.
    pub(crate) sql: &'static str,
}

/// Failure modes while opening, configuring or migrating the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// The SQLite file could not be opened, configured, or its parent created.
    Open(String),
    /// The embedded migrations could not be applied.
    Migration(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open(message) => write!(formatter, "could not open the database: {message}"),
            Self::Migration(message) => {
                write!(formatter, "could not migrate the database: {message}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// Shared SQLite store that implements the application storage ports.
///
/// The clone is cheap (`Arc`): every clone writes through the same connection
/// and therefore the same single write queue.
#[derive(Clone)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Opens (creating if needed) the database at `path`, configures it and
    /// applies migrations.
    ///
    /// The parent directory is created with `create_dir_all` when missing, so
    /// opening a canonical path inside a fresh tree works without the caller
    /// assembling the path or creating directories by hand. A failure to create
    /// the parent is reported as [`StorageError::Open`]. A path without a parent
    /// component (a bare file name) is opened as given.
    ///
    /// Configuration enables WAL (spec §9), foreign keys (spec §11) and a busy
    /// timeout so an external writer cannot make a transient open fail.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| StorageError::Open(error.to_string()))?;
            }
        }
        let connection =
            Connection::open(path).map_err(|error| StorageError::Open(error.to_string()))?;
        configure(&connection).map_err(|error| StorageError::Open(error.to_string()))?;
        migrate(&connection).map_err(|error| StorageError::Migration(error.to_string()))?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Locks the shared connection, recovering from a poisoned mutex.
    ///
    /// A handler panic is caught by the job worker, but it happens while the
    /// repository lock is held, so the mutex can be poisoned. The SQLite
    /// connection itself remains valid and consistent, so poisoning is
    /// recovered rather than propagated (RUST-001: this is not an `unwrap`).
    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Applies the connection pragmas required by the spec.
fn configure(connection: &Connection) -> rusqlite::Result<()> {
    // WAL lets readers proceed while a write is committed (spec §9).
    let _journal_mode: String =
        connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

/// Returns the directory where xemnas keeps its data.
///
/// `XEMNAS_DATA_DIR` wins when set to a non-empty value; otherwise the
/// platform data directory `%LOCALAPPDATA%\xemnas` is used (falling back to the
/// system temporary directory when `LOCALAPPDATA` is unavailable). The returned
/// directory is created if necessary; creation failures are ignored because the
/// signature cannot report them.
pub fn default_data_dir() -> PathBuf {
    let directory = match std::env::var_os("XEMNAS_DATA_DIR") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("xemnas"),
    };
    let _ = std::fs::create_dir_all(&directory);
    directory
}

/// Returns the canonical database path used by the desktop app.
///
/// This is `default_data_dir()/state/app.db`, the `app-data/state/app.db`
/// location documented in `docs/stack-e-arquitetura-rust-gpui.md`. The
/// `state` parent directory is created by [`SqliteStore::open`], so callers
/// only need to pass this path. `XEMNAS_DATA_DIR` therefore overrides the whole
/// path exactly as it overrides [`default_data_dir`].
pub fn default_db_path() -> PathBuf {
    default_data_dir().join("state").join("app.db")
}

/// Applies every migration that is not recorded yet, in order and transactionally.
fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    for migration in MIGRATIONS {
        let applied: i64 = connection.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            [migration.version],
            |row| row.get(0),
        )?;
        if applied > 0 {
            continue;
        }

        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) \
             VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            [migration.version],
        )?;
        transaction.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{default_data_dir, default_db_path, SqliteStore, MIGRATIONS};

    fn temporary_directory(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let directory = std::env::temp_dir().join(format!(
            "xemnas-store-{tag}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create temporary directory");
        directory
    }

    #[test]
    fn default_data_dir_respects_environment_override() {
        let expected = temporary_directory("data-dir");
        std::env::set_var("XEMNAS_DATA_DIR", &expected);
        let actual = default_data_dir();
        let database = default_db_path();
        std::env::remove_var("XEMNAS_DATA_DIR");
        assert_eq!(actual, expected);
        assert_eq!(database, expected.join("state").join("app.db"));
    }

    #[test]
    fn open_creates_missing_parent_directory() {
        let root = temporary_directory("parent");
        let database = root.join("state").join("app.db");
        assert!(!root.join("state").exists());

        let store = SqliteStore::open(&database).expect("open with missing parent");
        assert!(root.join("state").is_dir());
        let connection = store.lock();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .expect("count projects");
        assert_eq!(count, 0, "a fresh database must have no projects");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn fresh_database_applies_every_migration_and_reopen_is_idempotent() {
        let root = temporary_directory("migrations");
        let database = root.join("app.db");

        {
            let store = SqliteStore::open(&database).expect("first open");
            let connection = store.lock();
            let migrations: i64 = connection
                .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                    row.get(0)
                })
                .expect("count migrations");
            assert_eq!(migrations, MIGRATIONS.len() as i64);
            let projects: i64 = connection
                .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
                .expect("count projects");
            assert_eq!(projects, 0);
            let jobs: i64 = connection
                .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
                .expect("count jobs");
            assert_eq!(jobs, 0);
        }

        let store = SqliteStore::open(&database).expect("second open");
        let connection = store.lock();
        let migrations: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations after reopen");
        assert_eq!(migrations, MIGRATIONS.len() as i64);
        let distinct: i64 = connection
            .query_row(
                "SELECT COUNT(DISTINCT version) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .expect("count distinct migrations");
        assert_eq!(distinct, MIGRATIONS.len() as i64);

        let _ = std::fs::remove_dir_all(&root);
    }
}
