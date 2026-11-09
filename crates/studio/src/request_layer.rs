/// One layer of a stack that a combined brief spans: its pull request
/// number, title and description (its author's text; `None` when it could
/// not be read), bottom first in the request's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLayer {
    pub number: u64,
    pub title: String,
    pub description: Option<String>,
}
