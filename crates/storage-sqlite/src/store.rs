//! SQLite store: one shared connection behind a mutex, plus embedded migrations.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::Connection;

/// Embedded schema migrations, applied in version order.
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
    Migration {
        version: 5,
        sql: include_str!("migrations/0005_create_decision_candidates.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("migrations/0006_create_assessments.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("migrations/0008_create_decisions.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("migrations/0009_add_adapter_version.sql"),
    },
    Migration {
        version: 10,
        sql: include_str!("migrations/0010_create_decision_relations.sql"),
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
#[derive(Clone)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Opens (creating if needed) the database at `path`, configures it and
    /// applies migrations.
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
    pub(crate) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Applies the connection pragmas required by the spec.
fn configure(connection: &Connection) -> rusqlite::Result<()> {
    let _journal_mode: String =
        connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

/// Returns the data directory from [`application::AppPaths::from_env`], creating it.
pub fn default_data_dir() -> PathBuf {
    let directory = application::AppPaths::from_env().data_dir;
    let _ = std::fs::create_dir_all(&directory);
    directory
}

/// Returns the database path from [`application::AppPaths::from_env`].
pub fn default_db_path() -> PathBuf {
    application::AppPaths::from_env().database
}

/// Applies every migration that is not recorded yet, in order and transactionally.
fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    apply_migrations(connection, MIGRATIONS)
}

/// Applies `migrations` forward-only, each in its own transaction.
fn apply_migrations(connection: &Connection, migrations: &[Migration]) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    for migration in migrations {
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
    use super::{
        apply_migrations, configure, default_data_dir, default_db_path, Migration, SqliteStore,
        MIGRATIONS,
    };
    use rusqlite::Connection;

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

    fn table_exists(connection: &Connection, name: &str) -> bool {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .expect("query sqlite_master")
            > 0
    }

    #[test]
    fn a_failing_migration_rolls_back_and_the_connection_stays_usable() {
        let root = temporary_directory("rollback");
        let database = root.join("app.db");
        let connection = Connection::open(&database).expect("open raw connection");
        configure(&connection).expect("configure");

        let migrations = [
            Migration {
                version: 1,
                sql: "CREATE TABLE good (id TEXT PRIMARY KEY);",
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE bad (id TEXT PRIMARY KEY); CREATE TABLE bad (id TEXT);",
            },
        ];
        assert!(apply_migrations(&connection, &migrations).is_err());

        let version_one: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .expect("count v1");
        assert_eq!(version_one, 1, "the good migration stays applied");
        let version_two: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
                [],
                |row| row.get(0),
            )
            .expect("count v2");
        assert_eq!(version_two, 0, "the failed version must not be recorded");
        assert!(
            !table_exists(&connection, "bad"),
            "the failed migration's table must be rolled back"
        );

        let corrected = [
            Migration {
                version: 1,
                sql: "CREATE TABLE good (id TEXT PRIMARY KEY);",
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE fixed (id TEXT PRIMARY KEY);",
            },
        ];
        apply_migrations(&connection, &corrected).expect("corrected migration applies");
        assert!(table_exists(&connection, "fixed"));
        let version_two: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
                [],
                |row| row.get(0),
            )
            .expect("count v2 after fix");
        assert_eq!(version_two, 1);

        let _ = std::fs::remove_dir_all(&root);
    }
}
