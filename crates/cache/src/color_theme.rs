/// The app's color theme, picked in Settings → Appearance. Stored as its
/// name in `appearance_settings.color_theme`. Four light themes first, then
/// four dark ones; Graphite (everyday dark) is the default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ColorTheme {
    Ledger,
    Vellum,
    Overcast,
    Signal,
    #[default]
    Graphite,
    Tide,
    Lantern,
    Beacon,
}

impl ColorTheme {
    pub const ALL: [Self; 8] = [
        Self::Ledger,
        Self::Vellum,
        Self::Overcast,
        Self::Signal,
        Self::Graphite,
        Self::Tide,
        Self::Lantern,
        Self::Beacon,
    ];

    /// The text stored in SQLite; the table's CHECK constraint lists the same values.
    pub(crate) fn as_sql_text(self) -> &'static str {
        match self {
            Self::Ledger => "ledger",
            Self::Vellum => "vellum",
            Self::Overcast => "overcast",
            Self::Signal => "signal",
            Self::Graphite => "graphite",
            Self::Tide => "tide",
            Self::Lantern => "lantern",
            Self::Beacon => "beacon",
        }
    }

    /// Decodes stored text; `None` for any other value.
    pub(crate) fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.as_sql_text() == text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme decodes from the text it is stored as.
    #[test]
    fn every_theme_round_trips_through_text() {
        for theme in ColorTheme::ALL {
            assert_eq!(ColorTheme::from_sql_text(theme.as_sql_text()), Some(theme));
        }
        assert_eq!(ColorTheme::from_sql_text("solarized"), None);
    }
}
