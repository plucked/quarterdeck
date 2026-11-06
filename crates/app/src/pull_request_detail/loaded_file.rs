use std::sync::Arc;

use qd_cache::PatchFingerprint;
use qd_diff::{ContextGap, DiffOptions, FileDiff, GapPlace, RevealedLines, SourceLines};
use qd_file_tree::FileFacets;
use qd_github::PullRequestFile;
use qd_syntax::DiffSyntax;

use crate::diff_view::{DIFF_CHARACTER_WIDTH, gutter_width};

/// Changed lines above which a file starts collapsed, so opening a pull
/// request with a generated or vendored file stays quick.
const LARGE_DIFF_LINES: usize = 400;

/// A changed file with its diff built for the current settings. Fields are
/// `Arc`s because each frame's `FileDiffView` owns what it draws.
#[derive(Clone)]
pub struct LoadedFile {
    pub file: Arc<PullRequestFile>,
    /// The diff shown: the patch's, with the revealed lines of `context`.
    pub diff: Arc<FileDiff>,
    /// The patch's diff alone, which `diff` is worked out from.
    patch_diff: Arc<FileDiff>,
    /// The file's text and the lines shown from it; `None` until the user
    /// first reveals lines around the hunks.
    context: Option<FileContext>,
    /// The unchanged lines `diff` leaves out, which the user can reveal;
    /// none for added, deleted and binary files.
    gaps: Arc<[ContextGap]>,
    /// Of the patch as loaded; a viewed mark only holds for the same fingerprint.
    pub fingerprint: PatchFingerprint,
    /// Syntax colors of `diff`, filled in after loading; `None` until then
    /// and for languages without a grammar.
    pub syntax: Option<Arc<DiffSyntax>>,
    /// Width of the line-number gutters, from the largest line number.
    pub gutter_width: f32,
    /// Estimated width of the longest line, which bounds sideways scrolling.
    pub longest_line_width: f32,
    /// The file's kind and type for the filter, worked out once here (on the
    /// background executor) rather than on every filter refresh.
    pub facets: Arc<FileFacets>,
}

/// A file's text after the change, and which of its lines show around the hunks.
#[derive(Clone)]
pub struct FileContext {
    pub source: Arc<SourceLines>,
    pub revealed: RevealedLines,
}

impl LoadedFile {
    /// Builds the file's diff with `options`.
    pub fn build(file: PullRequestFile, options: DiffOptions) -> Self {
        let file = Arc::new(file);
        let diff = Arc::new(build_diff(&file, options));
        let facets = Arc::new(FileFacets::of(&file.filename, is_unchanged_move(&file)));
        Self {
            facets,
            gutter_width: gutter_width(diff.max_line_number()),
            longest_line_width: longest_line_width(&diff),
            gaps: gaps_of(&file, &diff, None),
            patch_diff: diff.clone(),
            diff,
            context: None,
            fingerprint: PatchFingerprint::of(&file),
            syntax: None,
            file,
        }
    }

    /// The same file with its diff rebuilt for new `options`, keeping the
    /// revealed lines; its lines may differ, so syntax colors are
    /// highlighted again afterwards.
    pub fn rebuilt(&self, options: DiffOptions) -> Self {
        let patch_diff = Arc::new(build_diff(&self.file, options));
        let rebuilt = Self {
            file: self.file.clone(),
            gutter_width: gutter_width(patch_diff.max_line_number()),
            longest_line_width: longest_line_width(&patch_diff),
            gaps: gaps_of(&self.file, &patch_diff, None),
            diff: patch_diff.clone(),
            patch_diff,
            context: None,
            fingerprint: self.fingerprint,
            syntax: None,
            facets: self.facets.clone(),
        };
        // The text matched the patch before, so it still does.
        match &self.context {
            Some(context) => rebuilt
                .with_context(context.clone())
                .unwrap_or_else(|| rebuilt.clone()),
            None => rebuilt,
        }
    }

    /// The same file showing `context`'s revealed lines around its hunks;
    /// `None` when the text is not what the patch leads to. Syntax colors
    /// are highlighted again afterwards.
    pub fn with_context(&self, context: FileContext) -> Option<Self> {
        let diff = Arc::new(
            self.patch_diff
                .with_context(&context.source, &context.revealed)?,
        );
        Some(Self {
            file: self.file.clone(),
            gutter_width: gutter_width(diff.max_line_number()),
            longest_line_width: longest_line_width(&diff),
            gaps: gaps_of(&self.file, &diff, Some(context.source.len())),
            diff,
            patch_diff: self.patch_diff.clone(),
            context: Some(context),
            fingerprint: self.fingerprint,
            syntax: None,
            facets: self.facets.clone(),
        })
    }

    /// The file's text and revealed lines, once fetched.
    pub fn context(&self) -> Option<&FileContext> {
        self.context.as_ref()
    }

    /// The hidden lines at `place`, if the diff leaves any out there.
    pub fn gap_at(&self, place: GapPlace) -> Option<&ContextGap> {
        self.gaps.iter().find(|gap| gap.place == place)
    }

    /// Highlights the diff's lines for the file's language.
    pub fn highlight_syntax(&self) -> Option<Arc<DiffSyntax>> {
        DiffSyntax::highlight(&self.diff, self.filename()).map(Arc::new)
    }

    pub fn filename(&self) -> &str {
        &self.file.filename
    }

    /// Whether the file was moved or copied with its content unchanged:
    /// GitHub sends no patch and counts no changed lines for it.
    pub fn is_unchanged_move(&self) -> bool {
        is_unchanged_move(&self.file)
    }

    /// Whether the file is big enough to start collapsed.
    pub fn is_large(&self) -> bool {
        self.diff.changed_line_count() > LARGE_DIFF_LINES
    }
}

/// The lines `diff` of `file` leaves out, given the file's length when
/// known. Added and deleted files show whole, and a binary file has no
/// lines, so they have none.
fn gaps_of(file: &PullRequestFile, diff: &FileDiff, line_count: Option<u32>) -> Arc<[ContextGap]> {
    match (file.status.as_str(), &file.patch) {
        ("added" | "removed", _) | (_, None) => Arc::from([]),
        (_, Some(_)) => diff.gaps(line_count).into(),
    }
}

/// The longest line's width in the diff font, by its character count.
fn longest_line_width(diff: &FileDiff) -> f32 {
    let longest = diff
        .hunks()
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .map(|line| line.text.chars().count())
        .max()
        .unwrap_or_default();
    f32::from(u16::try_from(longest).unwrap_or(u16::MAX)) * DIFF_CHARACTER_WIDTH
}

/// The diff of a file's patch; a file without a patch (binary) has none.
fn build_diff(file: &PullRequestFile, options: DiffOptions) -> FileDiff {
    file.patch
        .as_deref()
        .map(|patch| FileDiff::build(patch, options))
        .unwrap_or_default()
}

/// Whether `file` was moved or copied with its content unchanged.
fn is_unchanged_move(file: &PullRequestFile) -> bool {
    let is_move = matches!(file.status.as_str(), "renamed" | "copied");
    is_move && file.patch.is_none() && file.additions == 0 && file.deletions == 0
}

#[cfg(test)]
mod tests {
    use qd_diff::WhitespaceMode;

    use super::*;

    /// A one-line change in the middle of a five-line file.
    fn loaded() -> LoadedFile {
        LoadedFile::build(
            PullRequestFile {
                filename: "a.rs".into(),
                previous_filename: None,
                status: "modified".into(),
                additions: 1,
                deletions: 1,
                patch: Some("@@ -3 +3 @@\n-old\n+new\n".into()),
                sha: None,
            },
            DiffOptions::default(),
        )
    }

    fn context(revealed: std::ops::Range<u32>) -> FileContext {
        let mut lines = RevealedLines::default();
        lines.reveal(revealed);
        FileContext {
            source: Arc::new(SourceLines::parse("one\ntwo\nnew\nfour\nfive\n")),
            revealed: lines,
        }
    }

    /// Revealing lines grows the shown diff and closes the gaps they fill;
    /// the end gap's length is known once the text is.
    #[test]
    fn revealed_lines_fill_gaps() {
        let file = loaded();
        assert!(file.gap_at(GapPlace::AboveHunk(0)).is_some());
        assert_eq!(file.gap_at(GapPlace::End).unwrap().new_end, None);

        let expanded = file.with_context(context(1..3)).unwrap();
        assert_eq!(expanded.diff.hunks()[0].lines.len(), 4);
        assert!(expanded.gap_at(GapPlace::AboveHunk(0)).is_none());
        assert_eq!(
            expanded.gap_at(GapPlace::End).unwrap().line_count(),
            Some(2)
        );
    }

    /// Rebuilding for other settings keeps the revealed lines.
    #[test]
    fn rebuilding_keeps_revealed_lines() {
        let expanded = loaded().with_context(context(1..6)).unwrap();
        let rebuilt = expanded.rebuilt(DiffOptions {
            whitespace: WhitespaceMode::Ignore,
            ..DiffOptions::default()
        });
        assert_eq!(rebuilt.diff.hunks()[0].lines.len(), 6);
        assert!(rebuilt.gap_at(GapPlace::End).is_none());
    }
}
