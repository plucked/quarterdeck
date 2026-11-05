use qd_diff::{DiffLineKind, FileDiff};

use crate::SyntaxSpan;
use crate::syntax_highlighter::SyntaxHighlighter;

/// Files with more diff lines than this are left plain, so one huge file
/// does not hold up the colors of the rest.
const MAX_HIGHLIGHTED_DIFF_LINES: usize = 5_000;

/// Syntax colors for every line of a [`FileDiff`], by hunk and line index.
///
/// Each hunk is highlighted on its own, old side (context and removed
/// lines) and new side (context and added lines) separately and in order,
/// so a construct spanning several lines of a side is colored right. A hunk
/// that starts inside a construct opened above it (a long block comment)
/// may be colored as code, since the lines above are not in the patch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffSyntax {
    hunks: Vec<Vec<Vec<SyntaxSpan>>>,
}

impl DiffSyntax {
    /// Highlights `diff` of the file at `path`. `None` for a language
    /// without a grammar, or a file too large to highlight.
    pub fn highlight(diff: &FileDiff, path: &str) -> Option<Self> {
        let line_count: usize = diff.hunks().iter().map(|hunk| hunk.lines.len()).sum();
        if line_count > MAX_HIGHLIGHTED_DIFF_LINES {
            return None;
        }
        let highlighter = SyntaxHighlighter::shared();
        let first_line = diff
            .hunks()
            .first()
            .and_then(|hunk| hunk.lines.first())
            .map(|line| line.text.as_str());
        let syntax = highlighter.find_syntax(path, first_line)?;

        let hunks = diff
            .hunks()
            .iter()
            .map(|hunk| {
                // Each side in order: old is context + removed, new is context + added.
                let old_side: Vec<usize> = (0..hunk.lines.len())
                    .filter(|index| hunk.lines[*index].kind != DiffLineKind::Added)
                    .collect();
                let new_side: Vec<usize> = (0..hunk.lines.len())
                    .filter(|index| hunk.lines[*index].kind != DiffLineKind::Removed)
                    .collect();
                let text_of = |index: &usize| hunk.lines[*index].text.as_str();
                let old_spans = highlighter.highlight_lines(syntax, old_side.iter().map(text_of));
                let new_spans = highlighter.highlight_lines(syntax, new_side.iter().map(text_of));

                // Context and added lines take the new side's colors, removed lines the old side's.
                let mut line_spans = vec![Vec::new(); hunk.lines.len()];
                for (index, spans) in new_side.into_iter().zip(new_spans) {
                    line_spans[index] = spans;
                }
                for (index, spans) in old_side.into_iter().zip(old_spans) {
                    if hunk.lines[index].kind == DiffLineKind::Removed {
                        line_spans[index] = spans;
                    }
                }
                line_spans
            })
            .collect();
        Some(Self { hunks })
    }

    /// The spans of line `line_index` of hunk `hunk_index`; none when out of range.
    pub fn spans(&self, hunk_index: usize, line_index: usize) -> &[SyntaxSpan] {
        self.hunks
            .get(hunk_index)
            .and_then(|hunk| hunk.get(line_index))
            .map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;

    use super::*;

    /// A removed line inside a block comment is colored as comment from the old side.
    #[test]
    fn removed_lines_use_old_side_state() {
        let patch = "@@ -1,3 +1,3 @@\n /* start\n-old comment line\n+new_code();\n";
        let diff = FileDiff::build(patch, DiffOptions::default());
        let syntax = DiffSyntax::highlight(&diff, "a.rs").unwrap();
        let comment = syntax.spans(0, 0)[0].color;
        // The removed line continues the comment on the old side...
        assert_eq!(syntax.spans(0, 1)[0].color, comment);
        // ...and the added line is still inside it on the new side too.
        assert_eq!(syntax.spans(0, 2)[0].color, comment);
    }

    /// An unknown language has no syntax colors.
    #[test]
    fn unknown_language_is_plain() {
        let diff = FileDiff::build("@@ -1 +1 @@\n-a\n+b\n", DiffOptions::default());
        assert_eq!(DiffSyntax::highlight(&diff, "data.unknownext"), None);
    }
}
