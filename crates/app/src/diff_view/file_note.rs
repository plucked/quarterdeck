/// Why a file shows a one-line note instead of (or before) its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileNote {
    /// GitHub sent no patch: a binary file or one too large.
    NoPatch,
    /// Hiding whitespace changes left nothing.
    OnlyWhitespace,
    RenamedWithoutChanges,
    NoChanges,
    /// A large file started collapsed; clicking the note expands it.
    CollapsedLarge {
        changed_lines: usize,
    },
}

impl FileNote {
    /// The note's text.
    pub fn message(self) -> String {
        match self {
            Self::NoPatch => "Binary file, or a diff too large for GitHub to send.".to_owned(),
            Self::OnlyWhitespace => "Only whitespace changes.".to_owned(),
            Self::RenamedWithoutChanges => "File renamed without changes.".to_owned(),
            Self::NoChanges => "No changes to show.".to_owned(),
            Self::CollapsedLarge { changed_lines } => {
                format!("Large diff ({changed_lines} changed lines) — click to show")
            }
        }
    }

    /// Whether clicking the note expands the file.
    pub fn is_expandable(self) -> bool {
        matches!(self, Self::CollapsedLarge { .. })
    }
}
