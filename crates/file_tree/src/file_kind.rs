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
}

impl FileKind {
    /// Every kind, in chip order.
    pub const ALL: [Self; 4] = [Self::Code, Self::Tests, Self::Generated, Self::Moved];

    /// The kind of the file at `path`, checked in the order Moved, then
    /// Generated, then Tests, else Code.
    pub fn of(path: &str, is_unchanged_move: bool) -> Self {
        match (
            is_unchanged_move,
            is_generated_path(path),
            is_test_path(path),
        ) {
            (true, _, _) => Self::Moved,
            (false, true, _) => Self::Generated,
            (false, false, true) => Self::Tests,
            (false, false, false) => Self::Code,
        }
    }

    /// The chip's text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Code => "Code",
            Self::Tests => "Tests",
            Self::Generated => "Generated",
            Self::Moved => "Moved",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file has one kind, Moved winning over Generated over Tests.
    #[test]
    fn kinds_follow_their_order() {
        assert_eq!(FileKind::of("tests/a_test.go", true), FileKind::Moved);
        assert_eq!(FileKind::of("Cargo.lock", false), FileKind::Generated);
        assert_eq!(FileKind::of("src/a_test.go", false), FileKind::Tests);
        assert_eq!(FileKind::of("src/main.rs", false), FileKind::Code);
    }
}
