//! Turns a fully migrated database back into one that only ran the
//! migrations up to a version, so upgrade tests can replay the rest.
//!
//! The schema of that version is computed by applying the migration files to
//! an in-memory database; everything a later migration created (triggers,
//! views, indexes, tables and columns added to earlier tables) is removed. New
//! migrations therefore never require these tests to be edited.

use rusqlite::Connection;

fn names(db: &Connection, kind: &str) -> Vec<String> {
    let mut statement = db
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1 AND name NOT LIKE 'sqlite_%'")
        .expect("prepare schema query");
    statement
        .query_map([kind], |row| row.get(0))
        .expect("schema query")
        .map(|name| name.expect("name"))
        .collect()
}

fn columns(db: &Connection, table: &str) -> Vec<String> {
    let mut statement = db
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .expect("prepare columns");
    statement
        .query_map([], |row| row.get(0))
        .expect("columns")
        .map(|name| name.expect("column"))
        .collect()
}

fn is_virtual(db: &Connection, table: &str) -> bool {
    db.query_row(
        "SELECT sql LIKE 'CREATE VIRTUAL TABLE%' FROM sqlite_master WHERE name = ?1",
        [table],
        |row| row.get(0),
    )
    .unwrap_or(false)
}

/// The schema after migrations `1..=version`, in memory.
fn schema_at(version: i64) -> Connection {
    let early = Connection::open_in_memory().expect("in-memory database");
    let mut files: Vec<_> =
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/migrations"))
            .expect("migrations directory")
            .map(|entry| entry.expect("entry").path())
            .collect();
    files.sort();
    for file in files {
        let name = file
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let number: i64 = name[..4].parse().expect("numbered migration");
        if number <= version {
            early
                .execute_batch(&std::fs::read_to_string(&file).expect("read migration"))
                .expect("apply migration");
        }
    }
    early
}

/// Rewinds `connection` to the schema of `version` and forgets the later
/// migrations, keeping the rows of the tables that already existed.
pub fn rewind(connection: &Connection, version: i64) {
    let early = schema_at(version);
    let keep_tables = names(&early, "table");
    let keep_triggers = names(&early, "trigger");
    let keep_views = names(&early, "view");
    connection
        .execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("foreign keys off");
    for trigger in names(connection, "trigger") {
        if !keep_triggers.contains(&trigger) {
            connection
                .execute_batch(&format!("DROP TRIGGER \"{trigger}\";"))
                .expect("drop trigger");
        }
    }
    for view in names(connection, "view") {
        if !keep_views.contains(&view) {
            connection
                .execute_batch(&format!("DROP VIEW \"{view}\";"))
                .expect("drop view");
        }
    }
    let keep_indexes = names(&early, "index");
    for index in names(connection, "index") {
        if !keep_indexes.contains(&index) {
            connection
                .execute_batch(&format!("DROP INDEX IF EXISTS \"{index}\";"))
                .expect("drop index");
        }
    }
    // Virtual tables take their shadow tables with them: drop until stable.
    loop {
        let extra: Vec<String> = names(connection, "table")
            .into_iter()
            .filter(|table| !keep_tables.contains(table) && table != "schema_migrations")
            .collect();
        let Some(table) = extra.first() else { break };
        let _ = connection.execute_batch(&format!("DROP TABLE IF EXISTS \"{table}\";"));
        assert!(
            !names(connection, "table").contains(table),
            "could not drop {table}"
        );
    }
    for table in &keep_tables {
        let original = columns(&early, table);
        if columns(connection, table) != original && is_virtual(&early, table) {
            // A virtual table (FTS5) cannot drop a column: rebuild it with
            // the earlier definition, keeping the rows of the shared columns.
            let shared = original
                .iter()
                .map(|column| format!("\"{column}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let definition: String = early
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("virtual table definition");
            connection
                .execute_batch(&format!(
                    "CREATE TEMP TABLE rewind_rows AS SELECT {shared} FROM \"{table}\";\n\
                     DROP TABLE \"{table}\";\n\
                     {definition};\n\
                     INSERT INTO \"{table}\" ({shared}) SELECT {shared} FROM rewind_rows;\n\
                     DROP TABLE rewind_rows;"
                ))
                .expect("rebuild virtual table");
            continue;
        }
        for column in columns(connection, table) {
            if !original.contains(&column) {
                connection
                    .execute_batch(&format!(
                        "ALTER TABLE \"{table}\" DROP COLUMN \"{column}\";"
                    ))
                    .expect("drop added column");
            }
        }
    }
    connection
        .execute(
            "DELETE FROM schema_migrations WHERE version > ?1",
            [version],
        )
        .expect("forget later migrations");
}
