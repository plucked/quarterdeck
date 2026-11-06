use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use rusqlite::Connection;

use crate::schema_migrations::apply_schema_migrations;

/// The data folder of release builds.
const APP_DIRECTORY: &str = "Quarterdeck";
/// The data folder of development (debug) builds, so a development build a
/// schema migration ahead never upgrades the database the installed
/// release uses (which would then refuse it as written by a newer version).
const DEVELOPMENT_APP_DIRECTORY: &str = "Quarterdeck Dev";
/// Where the app kept its data before it was named Quarterdeck.
const FORMER_APP_DIRECTORY: &str = "quick-check";
const DATABASE_FILE: &str = "cache.sqlite";

/// `~/Library/Application Support/Quarterdeck/cache.sqlite` on macOS
/// (`Quarterdeck Dev/` in a debug build): the one database file every store
/// in this crate opens its own connection to.
pub(crate) fn default_database_path() -> Result<PathBuf> {
    let data_directory = dirs::data_dir().context("no data directory")?;
    Ok(data_directory.join(build_directory()).join(DATABASE_FILE))
}

/// Gets the data folder ready before the default database opens; the
/// stores call it first. A release build moves the data kept under the
/// app's former name (`quick-check`) once; a debug build starts its own
/// folder, once, from a copy of the release build's database, so viewed
/// marks, notes and settings carry over.
pub(crate) fn prepare_data_directory() -> Result<()> {
    let data_directory = dirs::data_dir().context("no data directory")?;
    if cfg!(debug_assertions) {
        seed_development_directory(&data_directory)
    } else {
        move_former_directory(&data_directory)
    }
}

/// The data folder of this build.
fn build_directory() -> &'static str {
    if cfg!(debug_assertions) {
        DEVELOPMENT_APP_DIRECTORY
    } else {
        APP_DIRECTORY
    }
}

/// Copies the release database into the development folder when that has
/// none yet. `VACUUM INTO` writes one consistent file, whatever the release
/// app is doing; the copy is migrated when the development build opens it,
/// the release database never is.
fn seed_development_directory(data_directory: &Path) -> Result<()> {
    let release = data_directory.join(APP_DIRECTORY).join(DATABASE_FILE);
    let development = data_directory.join(DEVELOPMENT_APP_DIRECTORY);
    let target = development.join(DATABASE_FILE);
    if target.exists() || !release.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(&development)?;
    let source = Connection::open(&release)?;
    source
        .execute("VACUUM INTO ?1", [target.to_string_lossy()])
        .with_context(|| format!("copying {} to {}", release.display(), target.display()))?;
    Ok(())
}

/// Renames the former data directory to the current one, so the cache and
/// UI state (viewed marks, notes, layout) survive the rename; nothing to do
/// when the current one exists already or there is no former one.
fn move_former_directory(data_directory: &Path) -> Result<()> {
    let current = data_directory.join(APP_DIRECTORY);
    let former = data_directory.join(FORMER_APP_DIRECTORY);
    if current.exists() || !former.exists() {
        return Ok(());
    }
    std::fs::rename(&former, &current)
        .with_context(|| format!("moving {} to {}", former.display(), current.display()))
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
        let directory = std::env::temp_dir().join(format!("qd-cache-test-{}", std::process::id()));
        let path = directory.join(DATABASE_FILE);
        open_database(&path).unwrap();
        open_database(&path).unwrap();
        std::fs::remove_dir_all(directory).ok();
    }

    /// The default database sits in the app's own data directory; tests
    /// are debug builds, so in the development one.
    #[test]
    fn default_path_is_app_scoped() {
        let path = default_database_path().unwrap();
        assert!(path.ends_with("Quarterdeck Dev/cache.sqlite"));
    }

    /// The development folder starts from a copy of the release database,
    /// once; later changes to either stay apart.
    #[test]
    fn seeds_the_development_directory_once() {
        // Set up: a release database holding one row.
        let data = std::env::temp_dir().join(format!("qd-seed-test-{}", std::process::id()));
        let release = data.join(APP_DIRECTORY).join(DATABASE_FILE);
        std::fs::create_dir_all(release.parent().unwrap()).unwrap();
        let connection = Connection::open(&release).unwrap();
        connection
            .execute_batch("CREATE TABLE marks (value TEXT); INSERT INTO marks VALUES ('viewed');")
            .unwrap();

        // Act: seed, change the release database, seed again.
        seed_development_directory(&data).unwrap();
        connection.execute("DELETE FROM marks", []).unwrap();
        seed_development_directory(&data).unwrap();

        // Expect: the copy kept the row from the first seed.
        let copy =
            Connection::open(data.join(DEVELOPMENT_APP_DIRECTORY).join(DATABASE_FILE)).unwrap();
        let count: i64 = copy
            .query_row("SELECT count(*) FROM marks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        std::fs::remove_dir_all(data).ok();
    }

    /// The former data directory moves to the current name once; a current
    /// one is never replaced.
    #[test]
    fn moves_the_former_directory() {
        let data = std::env::temp_dir().join(format!("qd-rename-test-{}", std::process::id()));
        std::fs::create_dir_all(data.join(FORMER_APP_DIRECTORY)).unwrap();
        std::fs::write(data.join(FORMER_APP_DIRECTORY).join(DATABASE_FILE), "old").unwrap();

        move_former_directory(&data).unwrap();
        assert!(!data.join(FORMER_APP_DIRECTORY).exists());
        assert!(data.join(APP_DIRECTORY).join(DATABASE_FILE).exists());

        // A former directory appearing again leaves the current one alone.
        std::fs::create_dir_all(data.join(FORMER_APP_DIRECTORY)).unwrap();
        move_former_directory(&data).unwrap();
        assert!(data.join(FORMER_APP_DIRECTORY).exists());
        std::fs::remove_dir_all(data).ok();
    }
}
