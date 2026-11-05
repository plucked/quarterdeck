use anyhow::{Result, anyhow};
use rusqlite::{OptionalExtension, params};

use crate::ui_state_store::SINGLE_ROW_ID;
use crate::{CacheTtl, UiStateStore};

impl UiStateStore {
    /// The last saved cache TTL.
    pub fn cache_ttl(&self) -> Result<Option<CacheTtl>> {
        let hours: Option<u32> = self
            .connection()
            .query_row(
                "SELECT ttl_hours FROM cache_settings WHERE id = ?1",
                [SINGLE_ROW_ID],
                |row| row.get(0),
            )
            .optional()?;
        // The CHECK constraint makes other values impossible short of a hand edit.
        hours
            .map(|hours| {
                CacheTtl::from_hours(hours)
                    .ok_or_else(|| anyhow!("unknown cache TTL of {hours} hours"))
            })
            .transpose()
    }

    /// Saves the cache TTL, replacing the previous one.
    pub fn put_cache_ttl(&self, ttl: CacheTtl) -> Result<()> {
        self.connection().execute(
            "INSERT INTO cache_settings (id, ttl_hours) VALUES (?1, ?2)
             ON CONFLICT (id) DO UPDATE SET ttl_hours = excluded.ttl_hours",
            params![SINGLE_ROW_ID, ttl.hours()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A saved TTL reads back, and a fresh database has none.
    #[test]
    fn cache_ttl_round_trips() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.cache_ttl().unwrap(), None);
        store.put_cache_ttl(CacheTtl::ThreeDays).unwrap();
        assert_eq!(store.cache_ttl().unwrap(), Some(CacheTtl::ThreeDays));
    }
}
