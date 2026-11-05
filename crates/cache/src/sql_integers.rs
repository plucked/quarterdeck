use anyhow::{Context as _, Result};
use rusqlite::Row;

/// Reads a non-negative integer column; a negative value is a damaged row and
/// an error, never a wrapped number.
pub(crate) fn read_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

/// Converts a GitHub id, number or count to SQLite's signed integer, failing
/// instead of wrapping when it does not fit.
pub(crate) fn sql_integer(value: u64) -> Result<i64> {
    i64::try_from(value).context("value exceeds SQLite's integer range")
}
