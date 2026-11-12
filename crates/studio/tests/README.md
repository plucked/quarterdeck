# qd-studio tests

Tests of the crate's public API: reading an agent's structured output into a checked studio against a request's diffs, and the schema handed to the agents. They never start a paid agent; `run_smart_filter.rs` uses a fake CLI to check the actual process arguments, streamed progress, login failures and cancellation. Unit tests of the prompt, the anchor checks and the event stream sit beside their code.
