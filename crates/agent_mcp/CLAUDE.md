# qd-agent-mcp

Blocking, application-independent ownership of a per-run loopback MCP endpoint. The official `rmcp =3.10.0` SDK owns protocol parsing, version negotiation, discovery and result envelopes. Only this crate may depend directly on Tokio: a private runtime runs on its own thread, and callers start, shut down and drop the server on their background executor.

| Module | Holds |
|---|---|
| `http_listener` | Bounded loopback connections and HTTP header deadlines |
| `http_policy` | Request authorization, exact authority/path checks and operation limits |
| `mcp_error` | Public startup and shutdown errors |
| `mcp_handler` | Blocking tool-dispatch contract |
| `mcp_server` | Endpoint, credentials, configuration and owned runtime lifetime |
| `mcp_service` | Private SDK handler and descriptor validation |
| `mcp_tool_descriptor` | Application-owned tool descriptions and schemas |
| `mcp_tool_result` | Structured and readable tool success/error results |
| `run_credential` | Operating-system randomness for an ephemeral bearer credential |

Handlers are short synchronous operations; they must not perform unbounded I/O. Shutdown rejects new work, closes connections and joins already executing handlers. HTTP cancellation and deadlines do not roll back a synchronous handler. The application closes its run state before shutdown when cancelling, and owns validation, idempotency, revision checks and publication. Nothing in this crate knows about review data, CLI permissions, persistence or gpui.

Tests use only synthetic handlers and IPv4 loopback, never external services or a user's credentials/cache. Protocol fixtures are public-API tests against the real SDK transport.
