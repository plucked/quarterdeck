use std::sync::Arc;

use qd_github::PullRequest;
use qd_markdown::MarkdownDocument;

/// The selected PR's parsed Markdown. The detail retains its document and
/// text selection while the description stays the same.
pub struct ParsedDescription {
    /// `None` when the description is empty.
    document: Option<Arc<MarkdownDocument>>,
}

impl ParsedDescription {
    pub fn parse(pull_request: &PullRequest) -> Self {
        Self {
            document: pull_request
                .body
                .as_deref()
                .map(MarkdownDocument::parse)
                .filter(|document| !document.is_empty())
                .map(Arc::new),
        }
    }

    pub fn document(&self) -> Option<&Arc<MarkdownDocument>> {
        self.document.as_ref()
    }
}
