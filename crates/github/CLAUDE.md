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
| `pull_request_revision` / `revision_check` | `PullRequestRevision` (head + base commit) and `RevisionCheck` (+ changed file count) from `GET /pulls/{n}` |
| `stack_membership` / `listed_stack_membership` / `stack_membership_query` | `StackMembership` (stack number, position, size, base branch); the GraphQL query filling it for both lists |
| `pull_request_stack` / `stack_layer` | `PullRequestStack` and its `StackLayer`s from `GET /repos/{o}/{r}/stacks/{n}` |
| `unified_diff` | splits a `git diff` (`application/vnd.github.diff`) into `PullRequestFile`s (crate-private) |
| `download_progress` | `DownloadProgress` (bytes received, `Content-Length`) |
| `graphql_response` | GraphQL envelope (data or errors; errors come with status 200) (crate-private) |
| `line_side` / `review_event` | `LineSide` (Old/New = LEFT/RIGHT), `ReviewEvent` (Comment, Approve, RequestChanges) |
| `review_thread` / `pull_request_review_state` | `ReviewThread` (+ `ThreadComment`, `ThreadPermissions`), `PullRequestReviewState` (node id, viewer, pending review, threads) |
| `review_submission` | `ReviewSubmission` (+ `NewReviewThread`, `NewThreadReply`, `SubmittedPart`) |
| `review_queries` / `review_requests` | the review GraphQL texts and answers; `GitHubClient::review_state`, `submit_review`, `discard_pending_review`, `update_review_comment`, `delete_review_comment`, `set_thread_resolved` |
| `etag` / `conditional_response` | `ETag`, `ConditionalResponse` (NotModified / Modified) |
| `next_page_url` | `Link` header pagination |
| `rate_limit` / `api_usage` | `RateLimit`, `RateLimitResource`, `ApiUsage` counts and budgets |

- Types derive `Deserialize` only where they are decoded from a GitHub response (`User`, `PullRequestFile`, `SearchIssue`); none derive `Serialize`. `qc-cache` stores every field in its own column, so a new field also needs a cache migration.
- `pull_request_files` follows `Link: rel="next"` pages (100 per page, at most 30 pages = GitHub's 3000-file cap). The search query reads one page (100 PRs).
- Conditional requests: `pull_request_revision(…, etag)` sends `If-None-Match`; `304` → `ConditionalResponse::NotModified`, which GitHub does not count against the rate limit. `pull_request_stack(…, etag)` works the same. The search API sends no ETag (`Cache-Control: no-cache`), so each list is always one search request (search budget: 30/min).
- `ApiUsage` (shared by clones) counts requests and 304s and keeps the last `x-ratelimit-*` budget per resource (core, search, GraphQL). A 403/429 with `x-ratelimit-remaining: 0` becomes `GitHubError::RateLimited { resource, resets_at }`.
- Lists: `authored()` and `review_requested()` (REST search, issue ids); `pull_request_by_number` uses the issues API so ids match the search ids. `stack_memberships()` is one GraphQL POST with both search texts as variables (1 point).
- `compare_files(base, head)` asks for the diff media type: one request for every file (the JSON compare stops at 300). Measured: 25-layer stack, 1692 files, 3 MB in 1.3 s; parsing takes 19 ms. Bodies are read in 64 KB chunks, reporting `DownloadProgress` (gzip makes received bytes exceed `Content-Length`).
- Reviews (GraphQL): `review_state` reads ≤100 threads × ≤100 comments and the viewer's pending review (only its author sees it). `submit_review`: with no pending review and no replies, one `addPullRequestReview` with `event` + `threads` creates and submits; otherwise threads (`addPullRequestReviewThread` into the pending one, or a new pending review holding them), replies (`addPullRequestReviewThreadReply`), then `submitPullRequestReview`; each part GitHub takes is reported (`SubmittedPart`) so a retry does not repost it. Lines are GitHub's: `line` on `side`, `startLine` for ranges; no `commitOID` (the head at submit time).
- Tests parse recorded JSON; none touch the network.
