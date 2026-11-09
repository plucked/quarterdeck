use super::studio_page::StudioPage;

/// An agent's explanation of a pull request: a lead stating the changed
/// behavior, then pages in the order to read them, each answering one
/// question with widgets in reading order. A small change has one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Studio {
    /// One to three sentences: what changes, for whom.
    pub lead: String,
    pub pages: Vec<StudioPage>,
}
