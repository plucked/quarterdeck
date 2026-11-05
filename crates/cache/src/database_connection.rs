use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use rusqlite::Connection;

use crate::schema_migrations::apply_schema_migrations;

const APP_DIRECTORY: &str = "quick-check";
const DATABASE_FILE: &str = "cache.sqlite";

/// `~/Library/Application Support/quick-check/cache.sqlite` on macOS: the one
/// database file every store in this crate opens its own connection to.
pub(crate) fn default_database_path() -> Result<PathBuf> {
    let data_directory = dirs::data_dir().context("no data directory")?;
    Ok(data_directory.join(APP_DIRECTORY).join(DATABASE_FILE))
}

/// Opens (creating if needed) the database at `path`, ready for use.
pub(crate) fn open_database(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    prepare_connection(Connection::open(path)?)
}

/// A fresh, migrated database that lives only as long as the connection.
#[cfg(test)]
pub(crate) fn open_in_memory_database() -> Result<Connection> {
    prepare_connection(Connection::open_in_memory()?)
}

/// Turns on WAL (so several connections can share the file) and foreign
/// keys, then brings the schema up to date.
fn prepare_connection(mut connection: Connection) -> Result<Connection> {
    connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    apply_schema_migrations(&mut connection)?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reopening a migrated file applies no migration twice.
    #[test]
    fn migrations_are_idempotent() {
        let directory = std::env::temp_dir().join(format!("qc-cache-test-{}", std::process::id()));
        let path = directory.join(DATABASE_FILE);
        open_database(&path).unwrap();
        open_database(&path).unwrap();
        std::fs::remove_dir_all(directory).ok();
    }

    /// The default database sits in the app's own data directory.
    #[test]
    fn default_path_is_app_scoped() {
        let path = default_database_path().unwrap();
        assert!(path.ends_with("quick-check/cache.sqlite"));
    }
}
