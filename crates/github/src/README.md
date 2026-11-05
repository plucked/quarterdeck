# qd-github source

Talks to GitHub: finds a token, sends authenticated REST requests (and one GraphQL query for stack membership, which the REST search lacks) and turns GitHub's JSON and diffs into the pull request, stack and file shapes the rest of the app uses. It does not cache anything and knows nothing about the UI; `qd-cache` stores what it returns and the app decides when to call it. Calls block, so callers run them off the main thread; long downloads report their progress.
