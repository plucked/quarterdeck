use serde::{Deserialize, Serialize};

/// A repository identified by owner and name, e.g. `zed-industries/zed`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

impl RepoRef {
    /// `owner/name`, as GitHub writes it.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Parses an API repository URL like `https://api.github.com/repos/{owner}/{name}`.
    pub(crate) fn from_api_url(url: &str) -> Option<Self> {
        let path = url.split("/repos/").nth(1)?;
        let mut segments = path.split('/');
        Some(Self {
            owner: segments.next()?.to_owned(),
            name: segments.next()?.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An API repository URL yields its owner and name.
    #[test]
    fn parses_repo_from_api_url() {
        let repo =
            RepoRef::from_api_url("https://api.github.com/repos/zed-industries/zed").unwrap();
        assert_eq!(repo.full_name(), "zed-industries/zed");
    }

    /// A URL without a `/repos/` path is rejected instead of guessed.
    #[test]
    fn rejects_url_without_repo_path() {
        assert!(RepoRef::from_api_url("https://api.github.com/users/octo").is_none());
    }
}
