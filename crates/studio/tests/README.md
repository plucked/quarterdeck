# qd-studio tests

Tests of the crate's public API: reading an agent's structured output into a checked studio against a request's diffs, and the schema handed to the agents. They never start a paid agent; `run_smart_filter.rs` uses a fake CLI to check the actual process arguments, streamed progress, login failures and cancellation; the brief and guided review launchers are both checked for the absence of a dollar budget cap. Unit tests of the prompt, the anchor checks and the event stream sit beside their code.

`run_guided_review.rs` drives a fake CLI through the real loopback MCP endpoint and verifies semantic repairs, sealed-receipt matching, exact process permissions, terminal and exit failures, output bounds, pipe cleanup, cancellation and endpoint shutdown.

`run_brief_review.rs` applies the same real HTTP/fake CLI boundary to page-level anchor repair, incomplete-brief rejection, full report-artifact retention and failed completion. Pure draft tests validate detailed domain rules independently of the MCP transport.
