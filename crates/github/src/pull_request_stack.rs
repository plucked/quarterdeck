use serde::Deserialize;

use crate::{RepoRef, StackLayer};

/// A GitHub stack with every layer, bottom (on the trunk) first, as
/// `GET /repos/{owner}/{repo}/stacks/{number}` returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestStack {
    pub repo: RepoRef,
    pub number: u64,
    /// The branch the bottom layer targets.
    pub base_ref: String,
    pub is_open: bool,
    /// Bottom first: layer `i` targets layer `i - 1`'s branch.
    pub layers: Vec<StackLayer>,
}

/// The response of `GET /repos/{owner}/{repo}/stacks/{number}`.
#[derive(Deserialize)]
pub(crate) struct StackResponse {
    number: u64,
    base: BranchName,
    open: bool,
    pull_requests: Vec<StackPullRequest>,
}

#[derive(Deserialize)]
struct BranchName {
    #[serde(rename = "ref")]
    name: String,
}

#[derive(Deserialize)]
struct StackPullRequest {
    id: u64,
    number: u64,
    title: String,
    state: String,
    #[serde(default)]
    draft: bool,
    merged_at: Option<String>,
    html_url: String,
    head: BranchTip,
    base: BranchTip,
    user: Option<Login>,
}

#[derive(Deserialize)]
struct BranchTip {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
}

#[derive(Deserialize)]
struct Login {
    login: String,
}

impl StackResponse {
    /// The stack of `repo`, layers in the order GitHub lists them (bottom first).
    pub fn into_stack(self, repo: &RepoRef) -> PullRequestStack {
        PullRequestStack {
            repo: repo.clone(),
            number: self.number,
            base_ref: self.base.name,
            is_open: self.open,
            layers: self
                .pull_requests
                .into_iter()
                .map(|layer| StackLayer {
                    pull_request_id: layer.id,
                    number: layer.number,
                    title: layer.title,
                    is_open: layer.state == "open",
                    is_draft: layer.draft,
                    is_merged: layer.merged_at.is_some(),
                    html_url: layer.html_url,
                    head_ref: layer.head.name,
                    head_sha: layer.head.sha,
                    base_ref: layer.base.name,
                    base_sha: layer.base.sha,
                    author_login: layer.user.map(|user| user.login).unwrap_or_default(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layers keep GitHub's order and read their branches, commits and state.
    #[test]
    fn reads_stack_layers() {
        let json = r#"{
            "id": 1, "number": 99, "base": {"ref": "main"}, "open": true,
            "pull_requests": [
                {"id": 11, "number": 7, "title": "Bottom", "state": "open", "draft": false,
                 "merged_at": null, "html_url": "https://github.com/a/b/pull/7",
                 "head": {"ref": "one", "sha": "h1"}, "base": {"ref": "main", "sha": "b1"},
                 "user": {"login": "octo"}},
                {"id": 12, "number": 8, "title": "Top", "state": "closed", "draft": true,
                 "merged_at": "2026-01-01T00:00:00Z", "html_url": "https://github.com/a/b/pull/8",
                 "head": {"ref": "two", "sha": "h2"}, "base": {"ref": "one", "sha": "h1"},
                 "user": null}
            ]
        }"#;
        let response: StackResponse = serde_json::from_str(json).unwrap();
        let repo = RepoRef {
            owner: "a".into(),
            name: "b".into(),
        };
        let stack = response.into_stack(&repo);
        assert_eq!(stack.base_ref, "main");
        assert_eq!(stack.layers.len(), 2);
        assert_eq!(stack.layers[1].base_ref, "one");
        assert!(stack.layers[1].is_merged && stack.layers[1].is_draft && !stack.layers[1].is_open);
        assert_eq!(stack.layers[1].author_login, "");
    }
}
