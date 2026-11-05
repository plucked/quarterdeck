use anyhow::{Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::diff_option_sql_text::{
    highlight_as_sql_text, highlight_from_sql_text, whitespace_as_sql_text,
    whitespace_from_sql_text,
};
use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{DiffLayout, DiffViewSettings, LineWrapping, UiStateStore};

/// The stored text of each setting, read before decoding.
struct StoredSettings {
    layout: String,
    highlight: String,
    whitespace: String,
    wrapping: String,
}

impl UiStateStore {
    /// The last saved diff view settings.
    pub fn diff_view_settings(&self) -> Result<Option<DiffViewSettings>> {
        let stored = self
            .connection()
            .query_row(
                "SELECT layout, highlight, whitespace, wrapping FROM diff_view_settings WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok(StoredSettings {
                        layout: row.get(0)?,
                        highlight: row.get(1)?,
                        whitespace: row.get(2)?,
                        wrapping: row.get(3)?,
                    })
                },
            )
            .optional()?;
        let Some(stored) = stored else {
            return Ok(None);
        };

        // The CHECK constraints make unknown values impossible short of a hand edit.
        let unknown = |column: &str, text: &str| anyhow!("unknown diff {column} {text:?}");
        Ok(Some(DiffViewSettings {
            layout: DiffLayout::from_sql_text(&stored.layout)
                .ok_or_else(|| unknown("layout", &stored.layout))?,
            highlight: highlight_from_sql_text(&stored.highlight)
                .ok_or_else(|| unknown("highlight", &stored.highlight))?,
            whitespace: whitespace_from_sql_text(&stored.whitespace)
                .ok_or_else(|| unknown("whitespace", &stored.whitespace))?,
            wrapping: LineWrapping::from_sql_text(&stored.wrapping)
                .ok_or_else(|| unknown("wrapping", &stored.wrapping))?,
        }))
    }

    /// Saves the diff view settings, replacing the previous ones.
    pub fn put_diff_view_settings(&self, settings: &DiffViewSettings) -> Result<()> {
        self.connection().execute(
            "INSERT INTO diff_view_settings (id, layout, highlight, whitespace, wrapping)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET
                layout = excluded.layout, highlight = excluded.highlight,
                whitespace = excluded.whitespace, wrapping = excluded.wrapping",
            params![
                SINGLE_ROW_ID,
                settings.layout.as_sql_text(),
                highlight_as_sql_text(settings.highlight),
                whitespace_as_sql_text(settings.whitespace),
                settings.wrapping.as_sql_text()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::{ChangeHighlight, WhitespaceMode};

    use super::*;

    /// Saved settings read back as saved, and a fresh database has none.
    #[test]
    fn diff_view_settings_round_trip() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.diff_view_settings().unwrap(), None);
        let settings = DiffViewSettings {
            layout: DiffLayout::Split,
            highlight: ChangeHighlight::Characters,
            whitespace: WhitespaceMode::Ignore,
            wrapping: LineWrapping::Wrap,
        };
        store.put_diff_view_settings(&settings).unwrap();
        assert_eq!(store.diff_view_settings().unwrap(), Some(settings));
    }
}
