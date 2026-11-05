use anyhow::{Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{ColorTheme, UiStateStore};

impl UiStateStore {
    /// The last saved color theme.
    pub fn color_theme(&self) -> Result<Option<ColorTheme>> {
        let name: Option<String> = self
            .connection()
            .query_row(
                "SELECT color_theme FROM appearance_settings WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| row.get(0),
            )
            .optional()?;
        // The CHECK constraint makes other values impossible short of a hand edit.
        name.map(|name| {
            ColorTheme::from_sql_text(&name).ok_or_else(|| anyhow!("unknown color theme {name}"))
        })
        .transpose()
    }

    /// Saves the color theme, replacing the previous one.
    pub fn put_color_theme(&self, theme: ColorTheme) -> Result<()> {
        self.connection().execute(
            "INSERT INTO appearance_settings (id, color_theme) VALUES (?1, ?2)
             ON CONFLICT (id) DO UPDATE SET color_theme = excluded.color_theme",
            params![SINGLE_ROW_ID, theme.sql_text()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A saved theme reads back, and a fresh database has none.
    #[test]
    fn color_theme_round_trips() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.color_theme().unwrap(), None);
        store.put_color_theme(ColorTheme::Vellum).unwrap();
        assert_eq!(store.color_theme().unwrap(), Some(ColorTheme::Vellum));
    }

    /// Every theme the app offers fits the table's CHECK constraint.
    #[test]
    fn every_theme_can_be_saved() {
        let store = UiStateStore::open_in_memory().unwrap();
        for theme in ColorTheme::ALL {
            store.put_color_theme(theme).unwrap();
            assert_eq!(store.color_theme().unwrap(), Some(theme));
        }
    }
}
