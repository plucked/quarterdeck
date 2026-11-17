use serde::Deserialize;

/// The envelope `GET /search/*` wraps its results in; the flags distinguish a complete result from a partial search.
#[derive(Deserialize)]
pub(crate) struct SearchResponse<T> {
    pub total_count: usize,
    pub incomplete_results: bool,
    pub items: Vec<T>,
}
