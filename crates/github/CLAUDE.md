# qd-github

| Module | Holds |
|---|---|
| `github_client` | `GitHubClient`: authenticated blocking REST calls |
| `github_error` | `GitHubError` |
| `resolve_token` / `resolved_token` / `token_source` | token lookup → `ResolvedToken` with its `TokenSource`: `GITHUB_TOKEN`, then the Keychain, then `gh auth token` (`gh` on `PATH`, `/opt/homebrew/bin/gh`, `/usr/local/bin/gh`: a Finder launch has neither Homebrew folder on `PATH`) |
| `keychain_token` | the token saved from the app's settings, a generic password in the login Keychain (service `de.plucked.quarterdeck`, account `github-token`, `security-framework`): `stored_token`, `store_token`, `delete_stored_token`; `GitHubError::Keychain` |
| `viewer_request` | `GitHubClient::viewer()`: `GET /user`, which checks a pasted token; `viewer_login()` asks once and remembers (shared by clones) |
| `repo_ref` | `RepoRef` (owner/name) |
| `user` | `User` |
| `label` | `Label` (name, color), on `PullRequest::labels` from the search results |
| `download_avatar` | `download_avatar(url, size)`: an avatar's PNG/JPEG bytes from `avatars.githubusercontent.com` only (`s=` size), without the token, at most 1 MB |
| `other_pull_requests` | `GitHubClient::other_pull_requests(repo, known, viewer_login)`: the issues API (`GET /repos/{o}/{r}/issues`, issue ids like the search results; core budget): nothing known → every open item (`SyncMode::Full`), else changes since the newest known `updated_at` (`SyncMode::Since`, `state=all`) merged into `known`; pull requests only (`pull_request` present), open, not the viewer's; up to 20 pages of 100; `others_search_queries` for the stack membership query |
| `repository_request` | `GitHubClient::repository(repo)`: `GET /repos/{o}/{r}`, the repository as GitHub spells it (404 when missing or invisible to the token); `RepoRef::parse_input` reads `owner/name` or a GitHub URL |
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
| `merge_method` / `merge_options` / `merge_outcome` / `merge_queries` / `pull_request_requests` | `MergeMethod`, `MergeOptions` (state, `MergeReadiness`, allowed methods, `MergePermissions`, `MergeQueueState`, branches), `MergeOutcome`; `GitHubClient::merge_options`, `start_merge`, `merge_outcome`, `close_pull_request`, `delete_branch` |
| `review_queries` / `review_requests` | the review GraphQL texts and answers; `GitHubClient::review_state`, `submit_review`, `discard_pending_review`, `update_review_comment`, `delete_review_comment`, `set_thread_resolved` |
| `file_text` / `file_text_location` / `file_text_query` / `file_text_requests` | `FileText` (blob SHA + text), `FileTextLocation` (full blob SHA, or path in a commit); `GitHubClient::file_text`: a blob by SHA is `GET …/git/blobs/{sha}` raw (one REST request), a path is one GraphQL `object(expression: "COMMIT:PATH")` (→ the REST request when `isTruncated`); binary (NUL in the first 8000 bytes) or over 8 MB is `GitHubError::NoFileText` |
| `etag` / `conditional_response` | `ETag`, `ConditionalResponse` (NotModified / Modified) |
| `next_page_url` | `Link` header pagination |
| `rate_limit` / `api_usage` | `RateLimit`, `RateLimitResource`, `ApiUsage` counts and budgets |

- `PullRequestFile::sha`: the blob after the change, full from the files API, abbreviated from a whole diff's `index OLD..NEW` line (none for a deleted file).
- Types derive `Deserialize` only where they are decoded from a GitHub response (`User`, `PullRequestFile`, `SearchIssue`); none derive `Serialize`. `qd-cache` stores every field in its own column, so a new field also needs a cache migration.
- `pull_request_files` follows `Link: rel="next"` pages (100 per page, at most 30 pages = GitHub's 3000-file cap, `MAX_LISTED_FILES`). Past it, `compare_files(base, head)` gets every file: the compare `.diff` has no file limit, while a pull request's own `.diff` answers 406 past 300 files. The search query reads one page (100 PRs).
- Conditional requests: `pull_request_revision(…, etag)` sends `If-None-Match`; `304` → `ConditionalResponse::NotModified`, which GitHub does not count against the rate limit. `pull_request_stack(…, etag)` works the same. The search API sends no ETag (`Cache-Control: no-cache`), so each list is always one search request (search budget: 30/min).
- Errors: `GitHubError::Api` shows the `message` of GitHub's JSON error body; a 403/429 with `retry-after` or naming the secondary rate limit (a burst of requests) is `GitHubError::SlowDown`.
- `ApiUsage` (shared by clones) counts requests and 304s and keeps the last `x-ratelimit-*` budget per resource (core, search, GraphQL). A 403/429 with `x-ratelimit-remaining: 0` becomes `GitHubError::RateLimited { resource, resets_at }`.
- Lists: `authored()` and `review_requested()` (REST search, issue ids); `pull_request_by_number` uses the issues API so ids match the search ids. `stack_memberships(other_repositories)` is one GraphQL POST with the search texts as variables (1 point); the others' search (the first 10 repositories, 100 newest) is included only when repositories are watched (`@include(if: $includeOthers)`).
- `compare_files(base, head)` asks for the diff media type: one request for every file (the JSON compare stops at 300). Measured: 25-layer stack, 1692 files, 3 MB in 1.3 s; parsing takes 19 ms. Bodies are read in 64 KB chunks, reporting `DownloadProgress` (gzip makes received bytes exceed `Content-Length`).
- Reviews (GraphQL): `review_state` reads ≤100 threads × ≤100 comments and the viewer's pending review (only its author sees it). `submit_review`: with no pending review and no replies, one `addPullRequestReview` with `event` + `threads` creates and submits; otherwise threads (`addPullRequestReviewThread` into the pending one, or a new pending review holding them), replies (`addPullRequestReviewThreadReply`), then `submitPullRequestReview`; each part GitHub takes is reported (`SubmittedPart`) so a retry does not repost it. Lines are GitHub's: `line` on `side`, `startLine` for ranges; no `commitOID` (the head at submit time).
- Merging uses the asynchronous endpoint (`PUT …/pulls/{n}/merge-async`, API version 2026-03-10): the only one that merges stacked PRs (it merges every unmerged layer below too, atomically) and it joins a merge queue when the base has one; `sha` pins the head, `bypass_rules` is the admin merge. A 409 (merge already pending) is followed, not an error. The result is polled at `…/merge-async/{uuid}` (`pending` → `merged` / `enqueued` / `failed`). Closing is `PATCH …/pulls/{n}` `state: closed`; the branch is `DELETE …/git/refs/heads/{branch}`. GitHub reports `viewerCanDeleteHeadRef: false` for every open pull request (it turns true once closed), so `MergePermissions::can_delete_head_ref` is that flag or push access (`viewerPermission` ADMIN/MAINTAIN/WRITE) on a branch that exists and is not a fork's.
- Tests parse recorded JSON; none touch the network.
