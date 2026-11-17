# Agent MCP transport

This crate starts an authenticated, ephemeral MCP endpoint for one external agent run. Its blocking guard owns the listener, bearer credential and dedicated runtime thread. The application supplies synchronous tool descriptions and handlers and keeps ownership of all domain state.

The official Rust SDK is pinned to `3.10.0`. Supported revisions are modern `2026-07-28` and legacy `2025-11-25` / `2025-06-18`; legacy HTTP runs without retained sessions. The SDK handles discovery, initialization, metadata and headers, and protocol result shapes. Resources, prompts, Tasks, sampling and elicitation are not advertised.

The endpoint accepts only `127.0.0.1:<chosen port>/mcp`, rejects every present Origin, and requires its fresh 256-bit bearer credential on every request. It bounds bodies to 1 MiB, simultaneous requests to eight, connections to 32, HTTP headers to 16 KiB and five seconds, and each HTTP operation to 30 seconds. These are transport operation limits; there is no deadline or budget for the whole agent run.

Call `McpServer::start` and drop its guard on a background worker. `config_json` returns a Claude configuration using child-environment placeholders; set `QUARTERDECK_MCP_URL` and `QUARTERDECK_MCP_TOKEN` from the guard without logging them. Keep the guard alive until the agent receives its final tool acknowledgement and exits. On cancellation, close application mutations first, then stop the agent and server. Shutdown drains synchronous handlers; handlers must return promptly and perform their own cancellation checks before mutation.
