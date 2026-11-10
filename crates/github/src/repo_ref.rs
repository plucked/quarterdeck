use crate::github_client::API_BASE;

/// A repository identified by owner and name, e.g. `zed-industries/zed`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

impl RepoRef {
    /// `owner/name`, as GitHub writes it.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Reads what a person types for a repository: `owner/name`, or a
    /// GitHub URL of it or of anything in it (`https://github.com/owner/name/pull/1`).
    /// `None` when no owner and name can be read.
    pub fn parse_input(text: &str) -> Option<Self> {
        let text = text.trim().trim_end_matches(".git");
        let path = text
            .strip_prefix("https://github.com/")
            .or_else(|| text.strip_prefix("http://github.com/"))
            .or_else(|| text.strip_prefix("github.com/"))
            .unwrap_or(text);
        let mut segments = path.split('/');
        let owner = segments.next()?.trim();
        let name = segments.next()?.trim();
        let is_name = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
        };
        (is_name(owner) && is_name(name)).then(|| Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    /// The repository's REST API URL, `https://api.github.com/repos/{owner}/{name}`,
    /// which every repository request extends.
    pub(crate) fn api_url(&self) -> String {
        format!("{API_BASE}/repos/{}/{}", self.owner, self.name)
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

    /// A repository's API URL is the base its requests extend.
    #[test]
    fn api_url_names_owner_and_name() {
        let repo = RepoRef::parse_input("zed-industries/zed").unwrap();
        assert_eq!(
            repo.api_url(),
            "https://api.github.com/repos/zed-industries/zed"
        );
    }

    /// An API repository URL yields its owner and name.
    #[test]
    fn parses_repo_from_api_url() {
        let repo =
            RepoRef::from_api_url("https://api.github.com/repos/zed-industries/zed").unwrap();
        assert_eq!(repo.full_name(), "zed-industries/zed");
    }

    /// Typed names and GitHub URLs read alike; anything else is refused.
    #[test]
    fn parses_typed_repositories() {
        for text in [
            "a/b",
            " a/b ",
            "https://github.com/a/b",
            "https://github.com/a/b/pull/12",
            "github.com/a/b.git",
        ] {
            assert_eq!(
                RepoRef::parse_input(text).unwrap().full_name(),
                "a/b",
                "{text}"
            );
        }
        for text in ["", "a", "a/", "a b/c", "https://github.com/a"] {
            assert!(RepoRef::parse_input(text).is_none(), "{text}");
        }
    }

    /// A URL without a `/repos/` path is rejected instead of guessed.
    #[test]
    fn rejects_url_without_repo_path() {
        assert!(RepoRef::from_api_url("https://api.github.com/users/octo").is_none());
    }
}
