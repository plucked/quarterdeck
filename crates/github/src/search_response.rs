use serde::Deserialize;

/// The envelope `GET /search/*` wraps its results in; only the items are read.
#[derive(Deserialize)]
pub(crate) struct SearchResponse<T> {
    pub items: Vec<T>,
}
