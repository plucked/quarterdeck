//! `test_endpoint` supplies a synthetic tool server and bounded HTTP requests.

mod test_endpoint;

pub use test_endpoint::{TestEndpoint, modern_request, post};
