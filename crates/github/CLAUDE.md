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
| `pull_request_revision` | `PullRequestRevision` (head + base commit) from `GET /pulls/{n}` |
| `etag` / `conditional_response` | `ETag`, `ConditionalResponse` (NotModified / Modified) |
| `next_page_url` | `Link` header pagination |
| `rate_limit` / `api_usage` | `RateLimit`, `RateLimitResource`, `ApiUsage` counts and budgets |

- Types derive `Deserialize` only where they are decoded from a GitHub response (`User`, `PullRequestFile`, `SearchIssue`); none derive `Serialize`. `qc-cache` stores every field in its own column, so a new field also needs a cache migration.
- `pull_request_files` follows `Link: rel="next"` pages (100 per page, at most 30 pages = GitHub's 3000-file cap). The search query reads one page (100 PRs).
- Conditional requests: `pull_request_revision(…, etag)` sends `If-None-Match`; `304` → `ConditionalResponse::NotModified`, which GitHub does not count against the rate limit. The search API sends no ETag (`Cache-Control: no-cache`), so the queue refresh is always one search request (search budget: 30/min).
- `ApiUsage` (shared by clones) counts requests and 304s and keeps the last `x-ratelimit-*` budget per resource (core, search). A 403/429 with `x-ratelimit-remaining: 0` becomes `GitHubError::RateLimited { resource, resets_at }`.
- Tests parse recorded JSON; none touch the network.
