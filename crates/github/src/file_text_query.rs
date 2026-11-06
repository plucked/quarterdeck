//! The GraphQL query of a file at a path in a commit, and its answer.

use serde::Deserialize;

/// The blob at an expression (`COMMIT:PATH`): its SHA, size and text. One
/// GraphQL request (1 point).
pub(crate) const FILE_TEXT_QUERY: &str = r"
query FileText($owner: String!, $name: String!, $expression: String!) {
  repository(owner: $owner, name: $name) {
    object(expression: $expression) {
      ... on Blob { oid byteSize isBinary isTruncated text }
    }
  }
}
";

/// The data of [`FILE_TEXT_QUERY`].
#[derive(Deserialize)]
pub(crate) struct FileTextData {
    repository: Option<RepositoryNode>,
}

#[derive(Deserialize)]
struct RepositoryNode {
    /// `None` when nothing is at the path; an empty object for a non-blob.
    object: Option<BlobNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BlobNode {
    /// Absent when the object is not a blob (a directory or submodule).
    pub oid: Option<String>,
    #[serde(default)]
    pub byte_size: u64,
    #[serde(default)]
    pub is_binary: bool,
    /// Whether `text` holds only the start of a large file.
    #[serde(default)]
    pub is_truncated: bool,
    pub text: Option<String>,
}

impl FileTextData {
    /// The blob found, if any.
    pub fn into_blob(self) -> Option<BlobNode> {
        self.repository?.object
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::GraphQlResponse;

    /// A blob answer is read with its SHA, flags and text.
    #[test]
    fn reads_a_blob() {
        let json = r#"{"data": {"repository": {"object": {"oid": "abc", "byteSize": 4, "isBinary": false, "isTruncated": false, "text": "a\nb\n"}}}}"#;
        let answer: GraphQlResponse<FileTextData> = serde_json::from_str(json).unwrap();
        let blob = answer.into_data().unwrap().into_blob().unwrap();
        assert_eq!(blob.oid.as_deref(), Some("abc"));
        assert_eq!(blob.text.as_deref(), Some("a\nb\n"));
        assert!(!blob.is_truncated);
    }

    /// A path with nothing at it has no blob; a tree has no SHA.
    #[test]
    fn missing_and_non_blob_objects() {
        let missing = r#"{"data": {"repository": {"object": null}}}"#;
        let answer: GraphQlResponse<FileTextData> = serde_json::from_str(missing).unwrap();
        assert!(answer.into_data().unwrap().into_blob().is_none());

        let tree = r#"{"data": {"repository": {"object": {}}}}"#;
        let answer: GraphQlResponse<FileTextData> = serde_json::from_str(tree).unwrap();
        assert!(
            answer
                .into_data()
                .unwrap()
                .into_blob()
                .unwrap()
                .oid
                .is_none()
        );
    }
}
