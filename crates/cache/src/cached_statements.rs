use anyhow::Result;
use rusqlite::{Connection, Params};

/// Runs `sql` with `params` through the connection's statement cache, so a
/// statement run for every row of a write is compiled once.
pub(crate) fn execute_cached(
    connection: &Connection,
    sql: &str,
    params: impl Params,
) -> Result<usize> {
    Ok(connection.prepare_cached(sql)?.execute(params)?)
}
