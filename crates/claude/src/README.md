# Local Claude sessions

Discovers running Claude Code sessions and submits text to one selected inbox. Discovery reads the public CLI inventory and joins it to the local session registry without changing it. Sending refreshes discovery, checks the selected process and appends a human explanation of Quarterdeck's one-way messages. It does not run an agent, change permissions, receive replies or confirm that Claude acted on a message. The app owns session selection and review-note formatting; the studio crate owns new headless runs.

Optional read-only Git lookups supply the branch and the main checkout path shared by a repository and its linked worktrees. The main path groups sessions, including detached worktrees; independent clones retain separate identities. This is separate from inventory and sending so the app can enrich visible rows without delaying discovery. Non-repositories, detached HEAD, unavailable Git or a timed-out lookup simply have no branch label.
