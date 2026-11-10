use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use crate::database_connection::{default_database_path, open_database, prepare_data_directory};

/// The SQLite connection holding cached GitHub data, one typed column per
/// field; each topic's reads and writes sit in its own file: pull requests
/// and lists in `pull_request_storage.rs`, changed files in
/// `file_set_storage.rs`, expiry in `cache_cleanup.rs`. Every write runs in one statement or one transaction, so the
/// database is consistent after any failure.
pub struct GitHubCache {
    connection: Connection,
}

impl GitHubCache {
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

    /// The connection, for the storage of other cached topics in sibling files.
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The connection for writes that need a transaction.
    pub(crate) fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}
