use std::sync::Arc;

use qd_github::PullRequest;
use qd_markdown::MarkdownDocument;

/// The parsed description of one version of a pull request. Kept while that
/// version stays shown, so a refresh does not replace the document and reset
/// its text selection.
pub struct ParsedDescription {
    pull_request_id: u64,
    updated_at: String,
    /// `None` when the description is empty.
    document: Option<Arc<MarkdownDocument>>,
}

impl ParsedDescription {
    pub fn parse(pull_request: &PullRequest) -> Self {
        Self {
            pull_request_id: pull_request.id,
            updated_at: pull_request.updated_at.clone(),
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

    /// Whether this is the description of `pull_request` as it is now.
    pub fn is_of(&self, pull_request: &PullRequest) -> bool {
        self.pull_request_id == pull_request.id && self.updated_at == pull_request.updated_at
    }
}
