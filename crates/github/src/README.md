# qc-github source

Talks to the GitHub REST API: finds a token, sends authenticated requests and turns GitHub's JSON into the pull request shapes the rest of the app uses. It does not cache anything and knows nothing about the UI; `qc-cache` stores what it returns and the app decides when to call it. Calls block, so callers run them off the main thread.
