use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use crate::database_connection::{default_database_path, open_database, prepare_data_directory};

/// The single row id of the one-row UI state tables.
pub(crate) const SINGLE_ROW_ID: i64 = 1;

/// A connection to the database for the UI state the app restores after a
/// relaunch; each topic's reads and writes sit in its own `…_storage.rs`
/// file (window placement, review queue view, diff view settings, viewed
/// files, …). Separate from [`crate::GitHubCache`]'s connection so UI writes
/// never wait behind a cache write; WAL lets both share the file.
///
/// Reads return `Ok(None)` when nothing was saved yet; every write is one
/// statement or one transaction.
pub struct UiStateStore {
    connection: Connection,
}

impl UiStateStore {
    /// Opens (creating if needed) the default database file.
    pub fn open_default() -> Result<Self> {
        prepare_data_directory()?;
        Self::open(&default_database_path()?)
    }

    /// Opens (creating if needed) the database at `path` and migrates it.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            connection: open_database(path)?,
        })
    }

    /// A fresh, migrated database that lives only as long as the value.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        Ok(Self {
            connection: crate::database_connection::open_in_memory_database()?,
        })
    }

    /// The connection, for the storage of other UI state topics in sibling files.
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }
}
