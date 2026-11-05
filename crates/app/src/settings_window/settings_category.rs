/// A page of the settings window, listed in this order on the left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsCategory {
    /// The color theme.
    #[default]
    Appearance,
    /// How changed files are shown.
    FileDiff,
    /// The local cache of GitHub data and API usage.
    GitHub,
}

impl SettingsCategory {
    pub const ALL: [Self; 3] = [Self::Appearance, Self::FileDiff, Self::GitHub];

    /// The category's name in the list and as the page title.
    pub fn title(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::FileDiff => "File Diff",
            Self::GitHub => "GitHub",
        }
    }
}
