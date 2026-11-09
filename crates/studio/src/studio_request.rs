use super::code_access::CodeAccess;
use super::reader_familiarity::ReaderFamiliarity;
use super::request_layer::RequestLayer;
use super::studio_request_file::StudioRequestFile;

/// What a studio is written about: the pull request (or combined stack
/// range) and its changed files in review order.
#[derive(Clone)]
pub struct StudioRequest {
    /// `owner/name`.
    pub repository: String,
    /// `Pull request #12: Title`, or the combined range of a stack.
    pub subject: String,
    /// The author's login.
    pub author: String,
    /// The pull request's description as written, possibly empty.
    pub description: String,
    pub files: Vec<StudioRequestFile>,
    /// Whether the agent can read the checked-out code.
    pub code: CodeAccess,
    /// How well the reader knows the area, which sets the onboarding.
    pub reader: ReaderFamiliarity,
    /// The stack layers a combined diff spans, bottom first; empty for one
    /// pull request.
    pub layers: Vec<RequestLayer>,
}
