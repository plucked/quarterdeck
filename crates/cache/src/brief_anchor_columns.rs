use anyhow::{Context as _, Result, anyhow};
use qd_studio::CodeAnchor;
use rusqlite::Row;

use crate::brief_sql_text::{side_as_sql_text, side_from_sql_text};

/// A code anchor as the four nullable columns a brief stores it in: the
/// path, the side as stored text, the first and the last line. All four are
/// NULL for no anchor; a row with only some of them does not read.
pub(crate) struct AnchorColumns {
    pub path: Option<String>,
    pub side: Option<String>,
    pub first_line: Option<i64>,
    pub last_line: Option<i64>,
}

impl AnchorColumns {
    /// The columns of an optional anchor.
    pub fn of(anchor: Option<&CodeAnchor>) -> Self {
        match anchor {
            Some(anchor) => Self {
                path: Some(anchor.path.clone()),
                side: Some(side_as_sql_text(anchor.side).to_owned()),
                first_line: Some(i64::from(anchor.first_line)),
                last_line: Some(i64::from(anchor.last_line)),
            },
            None => Self {
                path: None,
                side: None,
                first_line: None,
                last_line: None,
            },
        }
    }

    /// Reads the four columns from `row`, starting at column `first`.
    pub fn read(row: &Row<'_>, first: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            path: row.get(first)?,
            side: row.get(first + 1)?,
            first_line: row.get(first + 2)?,
            last_line: row.get(first + 3)?,
        })
    }

    /// The anchor: `None` when all four columns are NULL, an error when only
    /// some are, the side is unknown or a line does not fit.
    pub fn into_anchor(self) -> Result<Option<CodeAnchor>> {
        match (self.path, self.side, self.first_line, self.last_line) {
            (Some(path), Some(side), Some(first), Some(last)) => Ok(Some(CodeAnchor {
                path,
                side: side_from_sql_text(&side).context("an unknown anchor side")?,
                first_line: u32::try_from(first)?,
                last_line: u32::try_from(last)?,
            })),
            (None, None, None, None) => Ok(None),
            _ => Err(anyhow!("an anchor with some of its columns missing")),
        }
    }
}
