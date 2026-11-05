# qc-github

| Module | Holds |
|---|---|
| `github_client` | `GitHubClient`: authenticated blocking REST calls |
| `github_error` | `GitHubError` |
| `resolve_token` | token lookup: `GITHUB_TOKEN`, then `gh auth token` |
| `repo_ref` | `RepoRef` (owner/name) |
| `user` | `User` |
| `pull_request` | `PullRequest`, normalized |
| `pull_request_file` | `PullRequestFile` (changed file + patch) |
| `search_issue` | raw search-API issue shape → `PullRequest` (crate-private) |
| `search_response` | search envelope (crate-private) |

- Public data types are `Serialize + Deserialize`: `qc-cache` stores them as JSON. Renaming a field breaks existing cache rows; add a schema migration or `#[serde(alias)]`.
- Only the first page (100) of any list is read; pagination is not implemented.
- Tests parse recorded JSON; none touch the network.
