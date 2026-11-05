use anyhow::Result;
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{PanelWidths, UiStateStore};

impl UiStateStore {
    /// The last saved panel widths.
    pub fn panel_widths(&self) -> Result<Option<PanelWidths>> {
        Ok(self
            .connection()
            .query_row(
                "SELECT sidebar_width, side_panel_width FROM panel_widths WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok(PanelWidths {
                        sidebar: row.get(0)?,
                        side_panel: row.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    /// Saves the sidebar's width; the side panel keeps its saved width (or
    /// the default when none is saved).
    pub fn put_sidebar_width(&self, width: f32) -> Result<()> {
        self.connection().execute(
            "INSERT INTO panel_widths (id, sidebar_width, side_panel_width) VALUES (?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET sidebar_width = excluded.sidebar_width",
            params![SINGLE_ROW_ID, width, PanelWidths::default().side_panel],
        )?;
        Ok(())
    }

    /// Saves the side panel's width; the sidebar keeps its saved width (or
    /// the default when none is saved).
    pub fn put_side_panel_width(&self, width: f32) -> Result<()> {
        self.connection().execute(
            "INSERT INTO panel_widths (id, sidebar_width, side_panel_width) VALUES (?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET side_panel_width = excluded.side_panel_width",
            params![SINGLE_ROW_ID, PanelWidths::default().sidebar, width],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each width saves on its own, the other keeping its value or the default.
    #[test]
    fn panel_widths_save_separately() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.panel_widths().unwrap(), None);

        store.put_side_panel_width(420.).unwrap();
        assert_eq!(
            store.panel_widths().unwrap(),
            Some(PanelWidths {
                side_panel: 420.,
                ..PanelWidths::default()
            })
        );

        store.put_sidebar_width(250.).unwrap();
        assert_eq!(
            store.panel_widths().unwrap(),
            Some(PanelWidths {
                sidebar: 250.,
                side_panel: 420.
            })
        );
    }

    /// The schema rejects a zero width instead of storing it.
    #[test]
    fn schema_rejects_zero_width() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert!(store.put_sidebar_width(0.).is_err());
    }
}
