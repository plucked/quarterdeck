use crate::file_text_query::{BlobNode, FILE_TEXT_QUERY, FileTextData};
use crate::github_client::RAW_MEDIA_TYPE;
use crate::{FileText, FileTextLocation, GitHubClient, GitHubError, RepoRef};

/// Files larger than this are not shown whole: their text would cost more
/// memory and cache space than a review needs.
const MAX_FILE_TEXT_BYTES: u64 = 8 * 1024 * 1024;
/// How far into a file git looks for a NUL byte to call it binary.
const BINARY_PROBE_BYTES: usize = 8000;

impl GitHubClient {
    /// The whole text of one file. A blob by full SHA is one REST request
    /// (raw content, up to 100 MB); a path in a commit is one GraphQL
    /// request, plus the REST request when GitHub cut the text short.
    /// Binary files and files over 8 MB are [`GitHubError::NoFileText`].
    pub fn file_text(
        &self,
        repo: &RepoRef,
        location: &FileTextLocation,
    ) -> Result<FileText, GitHubError> {
        match location {
            FileTextLocation::Blob(blob_sha) => self.blob_text(repo, blob_sha),
            FileTextLocation::Path { commit, path } => {
                let data: FileTextData = self.graphql(
                    FILE_TEXT_QUERY,
                    &serde_json::json!({
                        "owner": repo.owner,
                        "name": repo.name,
                        "expression": format!("{commit}:{path}"),
                    }),
                )?;
                let blob = data.into_blob().ok_or(GitHubError::NoFileText {
                    reason: "is not in the commit",
                })?;
                self.text_of_blob_node(repo, blob)
            }
        }
    }

    /// The text of a GraphQL blob answer, read again by its SHA when GitHub
    /// truncated it.
    fn text_of_blob_node(&self, repo: &RepoRef, blob: BlobNode) -> Result<FileText, GitHubError> {
        // Only blobs have a SHA: a directory or submodule has no text.
        let blob_sha = blob.oid.ok_or(GitHubError::NoFileText {
            reason: "is not a file",
        })?;
        check_text_size(blob.byte_size)?;
        if blob.is_binary {
            return Err(GitHubError::NoFileText {
                reason: "is binary",
            });
        }
        match (blob.text, blob.is_truncated) {
            (Some(text), false) => Ok(FileText { blob_sha, text }),
            (Some(_) | None, true) | (None, false) => self.blob_text(repo, &blob_sha),
        }
    }

    /// A blob's raw content by its full SHA. One REST request.
    fn blob_text(&self, repo: &RepoRef, blob_sha: &str) -> Result<FileText, GitHubError> {
        let url = format!("{}/git/blobs/{blob_sha}", repo.api_url());
        let response = self.get(&url, &[], None, RAW_MEDIA_TYPE, &mut |_| {})?;
        check_text_size(u64::try_from(response.body.len()).unwrap_or(u64::MAX))?;
        if is_binary(&response.body) {
            return Err(GitHubError::NoFileText {
                reason: "is binary",
            });
        }
        Ok(FileText {
            blob_sha: blob_sha.to_owned(),
            text: response.body,
        })
    }
}

/// An error for a file over [`MAX_FILE_TEXT_BYTES`].
fn check_text_size(byte_size: u64) -> Result<(), GitHubError> {
    match byte_size.cmp(&MAX_FILE_TEXT_BYTES) {
        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(GitHubError::NoFileText {
            reason: "is larger than 8 MB",
        }),
    }
}

/// Whether `text` looks binary the way git decides: a NUL byte near its start.
fn is_binary(text: &str) -> bool {
    text.bytes().take(BINARY_PROBE_BYTES).any(|byte| byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A NUL byte near the start marks a file binary; plain text is not.
    #[test]
    fn detects_binary_content() {
        assert!(is_binary("PNG\0\u{1}"));
        assert!(!is_binary("fn main() {}\n"));
    }

    /// Files up to the limit pass; larger ones are refused.
    #[test]
    fn refuses_files_over_the_limit() {
        assert!(check_text_size(MAX_FILE_TEXT_BYTES).is_ok());
        assert!(matches!(
            check_text_size(MAX_FILE_TEXT_BYTES + 1),
            Err(GitHubError::NoFileText { .. })
        ));
    }
}
