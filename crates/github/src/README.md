# qd-github source

Talks to GitHub: finds a token (the environment, the Keychain where the app saves a pasted one, or the `gh` CLI), sends authenticated REST requests and GraphQL queries for pull request metadata, checks, and conversations and turns GitHub's JSON and diffs into the pull request, stack and file shapes the rest of the app uses. It does not cache anything and knows nothing about the UI; `qd-cache` stores what it returns and the app decides when to call it. Calls block, so callers run them off the main thread; long downloads report their progress.
