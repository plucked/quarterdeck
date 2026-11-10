use ureq::http::StatusCode;

use crate::ETag;

/// One answer from GitHub, read whole.
pub(crate) struct HttpResponse {
    pub status: StatusCode,
    pub body: String,
    pub etag: Option<ETag>,
    pub next_page: Option<String>,
}
