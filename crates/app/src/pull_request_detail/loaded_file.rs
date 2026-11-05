use std::sync::Arc;

use qd_cache::PatchFingerprint;
use qd_diff::{DiffOptions, FileDiff};
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
    pub diff: Arc<FileDiff>,
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

impl LoadedFile {
    /// Builds the file's diff with `options`.
    pub fn build(file: PullRequestFile, options: DiffOptions) -> Self {
        let file = Arc::new(file);
        let diff = build_diff(&file, options);
        let facets = Arc::new(FileFacets::of(&file.filename, is_unchanged_move(&file)));
        Self {
            facets,
            gutter_width: gutter_width(diff.max_line_number()),
            longest_line_width: longest_line_width(&diff),
            diff: Arc::new(diff),
            fingerprint: PatchFingerprint::of(&file),
            syntax: None,
            file,
        }
    }

    /// The same file with its diff rebuilt for new `options`; its lines may
    /// differ, so syntax colors are highlighted again afterwards.
    pub fn rebuilt(&self, options: DiffOptions) -> Self {
        let diff = build_diff(&self.file, options);
        Self {
            file: self.file.clone(),
            gutter_width: gutter_width(diff.max_line_number()),
            longest_line_width: longest_line_width(&diff),
            diff: Arc::new(diff),
            fingerprint: self.fingerprint,
            syntax: None,
            facets: self.facets.clone(),
        }
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
