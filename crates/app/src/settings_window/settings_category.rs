/// A page of the settings window, listed in this order on the left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsCategory {
    /// How changed files are shown.
    #[default]
    FileDiff,
    /// The local cache of GitHub data and API usage.
    GitHub,
}

impl SettingsCategory {
    pub const ALL: [Self; 2] = [Self::FileDiff, Self::GitHub];

    /// The category's name in the list and as the page title.
    pub fn title(self) -> &'static str {
        match self {
            Self::FileDiff => "File Diff",
            Self::GitHub => "GitHub",
        }
    }
}
