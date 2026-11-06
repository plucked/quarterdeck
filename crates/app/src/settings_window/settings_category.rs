/// A page of the settings window, listed in this order on the left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsCategory {
    /// The color theme.
    #[default]
    Appearance,
    /// How changed files are shown.
    FileDiff,
    /// The GitHub account, the local cache of GitHub data and API usage.
    GitHub,
    /// The running version and updates.
    Updates,
}

impl SettingsCategory {
    pub const ALL: [Self; 4] = [
        Self::Appearance,
        Self::FileDiff,
        Self::GitHub,
        Self::Updates,
    ];

    /// The category's name in the list and as the page title.
    pub fn title(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::FileDiff => "File Diff",
            Self::GitHub => "GitHub",
            Self::Updates => "Updates",
        }
    }
}
