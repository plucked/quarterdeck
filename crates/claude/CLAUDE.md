# qd-claude

Blocking, UI-free access to existing local Claude Code sessions. Run on the app's background executor. Never edit Claude's registry or settings, launch a conversation, or attest a permission mode for Quarterdeck.

| Module | Holds |
|---|---|
| `find_claude` | CLI lookup through PATH, the native installer and Homebrew |
| `command_output` | bounded local command output and an execution deadline, killing and reaping a timed-out child |
| `git_command` | local Git command setup without optional locks or inherited repository overrides |
| `git_repository` | main-checkout identity from NUL-delimited Git worktree records; linked and detached worktrees share a repository, separate local clones stay distinct; two-second deadline |
| `git_branch` | optional branch name from a read-only Git HEAD lookup in the session directory; linked worktrees and subdirectories supported, inherited repository overrides removed, two-second deadline |
| `session_id`, `claude_session` | checked session UUID and a live process identity with display metadata |
| `session_inventory` | fresh `claude agents --json`, joined to matching owned registry records and inbox sockets |
| `session_sender` | refresh before every send, process identity checks, bounded NDJSON write with Quarterdeck attribution and no reply address |

The registry and wire adapter reflect Claude Code 2.1.321–2.1.323. An accepted socket write is only a submission; Claude may hold or refuse it. Tests use temporary executables, registry records and Unix sockets, never installed Claude or the user's sessions.

`procStart` is formatted by Claude with `LC_ALL=C TZ=UTC`; the inventory's `ps` query must use both overrides. The timezone regression runs the app-side lookup in a subprocess with `TZ=Europe/Berlin` against a UTC registry fixture.
