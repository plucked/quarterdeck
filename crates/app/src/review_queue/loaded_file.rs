use std::sync::Arc;

use qc_cache::PatchFingerprint;
use qc_diff::{DiffOptions, FileDiff};
use qc_github::PullRequestFile;
use qc_syntax::DiffSyntax;

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
}

impl LoadedFile {
    /// Builds the file's diff with `options`.
    pub fn build(file: PullRequestFile, options: DiffOptions) -> Self {
        let file = Arc::new(file);
        Self {
            diff: Arc::new(build_diff(&file, options)),
            fingerprint: PatchFingerprint::of(&file),
            syntax: None,
            file,
        }
    }

    /// The same file with its diff rebuilt for new `options`; its lines may
    /// differ, so syntax colors are highlighted again afterwards.
    pub fn rebuilt(&self, options: DiffOptions) -> Self {
        Self {
            file: self.file.clone(),
            diff: Arc::new(build_diff(&self.file, options)),
            fingerprint: self.fingerprint,
            syntax: None,
        }
    }

    /// Highlights the diff's lines for the file's language.
    pub fn highlight_syntax(&self) -> Option<Arc<DiffSyntax>> {
        DiffSyntax::highlight(&self.diff, self.filename()).map(Arc::new)
    }

    pub fn filename(&self) -> &str {
        &self.file.filename
    }

    /// Whether the file is big enough to start collapsed.
    pub fn is_large(&self) -> bool {
        self.diff.changed_line_count() > LARGE_DIFF_LINES
    }
}

/// The diff of a file's patch; a file without a patch (binary) has none.
fn build_diff(file: &PullRequestFile, options: DiffOptions) -> FileDiff {
    file.patch
        .as_deref()
        .map(|patch| FileDiff::build(patch, options))
        .unwrap_or_default()
}
