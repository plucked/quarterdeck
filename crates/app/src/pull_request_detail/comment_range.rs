use qd_github::LineSide;

use super::diff_line_ref::DiffLineRef;
use super::loaded_file::LoadedFile;
use crate::diff_line_sides::line_number_on;

/// The lines the comment being written or edited is about: lines
/// `first..=last` on `side` of the loaded file `file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentRange {
    pub file: usize,
    pub side: LineSide,
    pub first: u32,
    pub last: u32,
}

impl CommentRange {
    /// Both endpoints must be in one original patch hunk, even after context expansion joins displayed hunks.
    pub fn in_patch(self, file: &LoadedFile) -> bool {
        file.patch_diff().hunks().iter().any(|hunk| {
            let lines = match self.side {
                LineSide::New => hunk.new_lines(),
                LineSide::Old => hunk.old_lines(),
            };
            lines.contains(&self.first) && lines.contains(&self.last)
        })
    }

    /// Extends an anchor in one hunk. Shared context can be addressed from either gutter.
    pub fn extended_to(self, at: DiffLineRef, file: &LoadedFile) -> Option<Self> {
        if self.file != at.file {
            return None;
        }
        let point = Self::ending_at(at.file, at.side, at.line, None);
        let (anchor, point) = if self.side == at.side {
            (self, point)
        } else if let Some(point) = point.shared_context_on(self.side, file) {
            (self, point)
        } else {
            (self.shared_context_on(at.side, file)?, point)
        };
        let extended = Self {
            first: anchor.first.min(point.first),
            last: anchor.last.max(point.last),
            ..anchor
        };
        extended.in_patch(file).then_some(extended)
    }

    /// Translates an unchanged span when the other endpoint determines which side GitHub needs.
    fn shared_context_on(self, side: LineSide, file: &LoadedFile) -> Option<Self> {
        let mut first = None;
        for line in file
            .patch_diff()
            .hunks()
            .iter()
            .flat_map(|hunk| &hunk.lines)
        {
            let Some(number) = line_number_on(line, self.side) else {
                continue;
            };
            if !(self.first..=self.last).contains(&number) {
                continue;
            }
            if line.origin != qd_diff::LineOrigin::Patch {
                return None;
            }
            let other = line_number_on(line, side)?;
            let first = *first.get_or_insert(other);
            if other.checked_sub(first)? != number - self.first {
                return None;
            }
            if number == self.last {
                return Some(Self {
                    side,
                    first,
                    last: other,
                    ..self
                });
            }
        }
        None
    }

    /// Whether a clicked gutter line falls within this comment's anchor.
    pub fn contains(self, at: DiffLineRef) -> bool {
        self.file == at.file && self.side == at.side && (self.first..=self.last).contains(&at.line)
    }

    /// A comment under line `line` that starts at `start_line` (or at
    /// `line` itself without one).
    pub fn ending_at(file: usize, side: LineSide, line: u32, start_line: Option<u32>) -> Self {
        Self {
            file,
            side,
            first: start_line.unwrap_or(line),
            last: line,
        }
    }
}
