# Pull request detail interaction tests

These scenarios render a pull request from an isolated cache and exercise it through GPUI mouse and keyboard input. They cover Find in wrapped diffs, search focus in the review queue, and single-line and range comments in both split gutters. Split comment scenarios check actual GitHub coordinates on shared context, changed lines and folded whitespace replacements.

Run them with `cargo test -p quarterdeck pull_request_detail::tests`.
