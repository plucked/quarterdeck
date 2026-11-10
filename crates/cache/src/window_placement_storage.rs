use anyhow::{Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{DisplayUuid, UiStateStore, WindowMode, WindowPlacement};

/// The columns of the `window_placement` row, read before decoding.
struct StoredWindowPlacement {
    mode: String,
    display_uuid: Option<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl UiStateStore {
    /// The last saved window placement.
    pub fn window_placement(&self) -> Result<Option<WindowPlacement>> {
        let row = self
            .connection()
            .query_row(
                "SELECT mode, display_uuid, x, y, width, height FROM window_placement WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| {
                    Ok(StoredWindowPlacement {
                        mode: row.get(0)?,
                        display_uuid: row.get(1)?,
                        x: row.get(2)?,
                        y: row.get(3)?,
                        width: row.get(4)?,
                        height: row.get(5)?,
                    })
                },
            )
            .optional()?;
        let Some(stored) = row else {
            return Ok(None);
        };

        // The CHECK constraint makes an unknown mode impossible short of a hand edit.
        let mode = WindowMode::from_sql_text(&stored.mode)
            .ok_or_else(|| anyhow!("unknown window mode {:?}", stored.mode))?;
        Ok(Some(WindowPlacement {
            mode,
            display: stored.display_uuid.map(DisplayUuid::new),
            x: stored.x,
            y: stored.y,
            width: stored.width,
            height: stored.height,
        }))
    }

    /// Saves the window placement, replacing the previous one.
    pub fn put_window_placement(&self, placement: &WindowPlacement) -> Result<()> {
        self.connection().execute(
            "INSERT INTO window_placement (id, mode, display_uuid, x, y, width, height)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (id) DO UPDATE SET
                mode = excluded.mode, display_uuid = excluded.display_uuid,
                x = excluded.x, y = excluded.y,
                width = excluded.width, height = excluded.height",
            params![
                SINGLE_ROW_ID,
                placement.mode.as_sql_text(),
                placement.display.as_ref().map(DisplayUuid::as_str),
                placement.x,
                placement.y,
                placement.width,
                placement.height
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh database has no saved window placement.
    #[test]
    fn fresh_database_has_no_window_placement() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert!(store.window_placement().unwrap().is_none());
    }

    /// The latest window placement replaces the previous one.
    #[test]
    fn window_placement_keeps_latest() {
        let store = UiStateStore::open_in_memory().unwrap();
        let first = WindowPlacement {
            mode: WindowMode::Windowed,
            display: Some(DisplayUuid::new("37D8832A-2D66-02CA-B9F7-8F30A301B230")),
            x: 10.,
            y: 20.,
            width: 800.,
            height: 600.,
        };
        let second = WindowPlacement {
            mode: WindowMode::Maximized,
            display: None,
            ..first.clone()
        };
        store.put_window_placement(&first).unwrap();
        store.put_window_placement(&second).unwrap();
        assert_eq!(store.window_placement().unwrap(), Some(second));
    }

    /// The schema rejects a zero-size window instead of storing it.
    #[test]
    fn schema_rejects_empty_window() {
        let store = UiStateStore::open_in_memory().unwrap();
        let empty = WindowPlacement {
            mode: WindowMode::Windowed,
            display: None,
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
        };
        assert!(store.put_window_placement(&empty).is_err());
    }
}
