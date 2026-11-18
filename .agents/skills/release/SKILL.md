---
name: release
description: Publish a signed and notarized Quarterdeck release from main with an optional version bump or exact version.
---

# Release Quarterdeck

Follow the [shared release workflow](../../../.claude/skills/release/SKILL.md). Use the version argument from the user's `$release` invocation as `$ARGUMENTS`. If omitted, use the workflow's version selection rule.

If `main` is already checked out in another worktree, locate it with `git worktree list` and run the workflow in that checkout. When the release script starts, wait for its exit status before verifying the feed.
