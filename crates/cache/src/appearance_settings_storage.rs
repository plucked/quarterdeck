use anyhow::{Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{ColorTheme, UiStateStore};

impl UiStateStore {
    /// Whether folder colors follow changed-file counts; none before appearance is saved.
    pub fn folder_heatmap(&self) -> Result<Option<bool>> {
        Ok(self
            .connection()
            .query_row(
                "SELECT folder_heatmap FROM appearance_settings WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Saves folder coloring independently of the selected theme.
    pub fn put_folder_heatmap(&self, enabled: bool) -> Result<()> {
        self.connection().execute(
            "INSERT INTO appearance_settings (id, color_theme, folder_heatmap) VALUES (?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET folder_heatmap = excluded.folder_heatmap",
            params![SINGLE_ROW_ID, ColorTheme::default().as_sql_text(), enabled],
        )?;
        Ok(())
    }

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
            "INSERT INTO appearance_settings (id, color_theme, folder_heatmap) VALUES (?1, ?2, 1)
             ON CONFLICT (id) DO UPDATE SET color_theme = excluded.color_theme",
            params![SINGLE_ROW_ID, theme.as_sql_text()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Either appearance preference can be saved first without resetting the other.
    #[test]
    fn folder_heatmap_and_theme_save_independently() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.folder_heatmap().unwrap(), None);

        store.put_folder_heatmap(true).unwrap();
        assert_eq!(store.folder_heatmap().unwrap(), Some(true));
        assert_eq!(store.color_theme().unwrap(), Some(ColorTheme::default()));

        store.put_color_theme(ColorTheme::Vellum).unwrap();
        assert_eq!(store.folder_heatmap().unwrap(), Some(true));
        store.put_folder_heatmap(false).unwrap();
        assert_eq!(store.folder_heatmap().unwrap(), Some(false));
        assert_eq!(store.color_theme().unwrap(), Some(ColorTheme::Vellum));
        store.put_color_theme(ColorTheme::Ledger).unwrap();
        assert_eq!(store.folder_heatmap().unwrap(), Some(false));

        let store = UiStateStore::open_in_memory().unwrap();
        store.put_color_theme(ColorTheme::Tide).unwrap();
        assert_eq!(store.folder_heatmap().unwrap(), Some(true));
        store.put_folder_heatmap(false).unwrap();
        assert_eq!(store.color_theme().unwrap(), Some(ColorTheme::Tide));
    }

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
