use serde::Deserialize;

use crate::github_client::API_BASE;
use crate::{GitHubClient, GitHubError, RepoRef};

/// The parts of `GET /repos/{owner}/{name}` read here.
#[derive(Deserialize)]
struct RepositoryResponse {
    name: String,
    owner: RepositoryOwner,
}

#[derive(Deserialize)]
struct RepositoryOwner {
    login: String,
}

impl GitHubClient {
    /// The repository `repo` names, as GitHub spells it (a renamed one
    /// answers under its new name). One request; a repository that does not
    /// exist or that the token cannot see is `GitHubError::Api` 404.
    pub fn repository(&self, repo: &RepoRef) -> Result<RepoRef, GitHubError> {
        let response: RepositoryResponse = self.get_json(
            &format!("{API_BASE}/repos/{}/{}", repo.owner, repo.name),
            &[],
        )?;
        Ok(RepoRef {
            owner: response.owner.login,
            name: response.name,
        })
    }
}
