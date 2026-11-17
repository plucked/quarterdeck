use super::content_match::ContentMatch;

/// A file's ranked path match or bounded content snippets.
#[derive(Clone, Debug, Default)]
pub(super) struct FileSearchMatch {
    pub score: usize,
    pub lines: Vec<ContentMatch>,
    pub line_count: usize,
}
