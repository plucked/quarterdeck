/// The Files pane's navigation mode, retained across launches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FilesMode {
    #[default]
    Tree,
    Plan,
}

impl FilesMode {
    pub const fn as_sql_text(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Plan => "plan",
        }
    }

    pub fn from_sql_text(text: &str) -> Option<Self> {
        match text {
            "tree" => Some(Self::Tree),
            "plan" => Some(Self::Plan),
            _ => None,
        }
    }
}
