/// How the main window was shown. Stored in `window_placement.mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    Maximized,
    Fullscreen,
}

impl WindowMode {
    const ALL: [Self; 3] = [Self::Windowed, Self::Maximized, Self::Fullscreen];

    /// The value stored in SQLite; the table's CHECK constraint lists the same values.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Maximized => "maximized",
            Self::Fullscreen => "fullscreen",
        }
    }

    /// Decodes a stored value; `None` for anything else.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_sql_text() == text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every mode decodes from the text it is stored as.
    #[test]
    fn every_mode_round_trips_through_sql_text() {
        for mode in WindowMode::ALL {
            assert_eq!(WindowMode::from_sql_text(mode.as_sql_text()), Some(mode));
        }
    }

    /// An unknown stored value decodes to `None` instead of a guess.
    #[test]
    fn unknown_text_decodes_to_none() {
        assert_eq!(WindowMode::from_sql_text("minimized"), None);
    }
}
