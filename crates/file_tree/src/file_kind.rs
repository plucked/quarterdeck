use crate::FileChange;
use crate::file_category::{is_generated_path, is_test_path};

/// What kind of change a file is, for the kind filter. Every file has
/// exactly one kind, so the kind counts add up to all files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind {
    /// Everything not of another kind.
    Code,
    /// Test files and folders (`is_test_path`).
    Tests,
    /// Generated files (`is_generated_path`).
    Generated,
    /// Moved or copied without changes: nothing in it to review.
    Moved,
    /// Removed from the repository.
    Deleted,
}

impl FileKind {
    /// Every kind, in chip order.
    pub const ALL: [Self; 5] = [
        Self::Code,
        Self::Tests,
        Self::Generated,
        Self::Moved,
        Self::Deleted,
    ];

    /// The kind of the file at `path`, checked in the order Deleted / Moved,
    /// then Generated, then Tests, else Code.
    pub fn of(path: &str, change: FileChange) -> Self {
        match change {
            FileChange::Deleted => Self::Deleted,
            FileChange::UnchangedMove => Self::Moved,
            FileChange::Content => match (is_generated_path(path), is_test_path(path)) {
                (true, _) => Self::Generated,
                (false, true) => Self::Tests,
                (false, false) => Self::Code,
            },
        }
    }

    /// The chip's text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Code => "Code",
            Self::Tests => "Tests",
            Self::Generated => "Generated",
            Self::Moved => "Moved",
            Self::Deleted => "Deleted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file has one kind, Deleted and Moved winning over Generated over Tests.
    #[test]
    fn kinds_follow_their_order() {
        assert_eq!(
            FileKind::of("tests/a_test.go", FileChange::Deleted),
            FileKind::Deleted
        );
        assert_eq!(
            FileKind::of("Cargo.lock", FileChange::Deleted),
            FileKind::Deleted
        );
        assert_eq!(
            FileKind::of("tests/a_test.go", FileChange::UnchangedMove),
            FileKind::Moved
        );
        assert_eq!(
            FileKind::of("Cargo.lock", FileChange::Content),
            FileKind::Generated
        );
        assert_eq!(
            FileKind::of("src/a_test.go", FileChange::Content),
            FileKind::Tests
        );
        assert_eq!(
            FileKind::of("src/main.rs", FileChange::Content),
            FileKind::Code
        );
    }
}
