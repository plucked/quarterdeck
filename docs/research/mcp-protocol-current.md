# Current MCP protocol for per-run Quarterdeck agents

Research date: **2026-11-15**. Scope: primary-source protocol research for a Rust desktop application that starts an ephemeral loopback MCP endpoint, launches a headless agent, receives guided-review buckets or briefs, returns validation feedback, and shuts the endpoint down. No application code was changed. Protocol statements below are separate from recommendations and from the installed client's observed behavior; see [Claude CLI compatibility research](claude-mcp-per-run.md) for the latter.

## Finding

Use ordinary, short **MCP tool calls over Streamable HTTP** for submission, validation, and acknowledgements. The app owns the run, frozen review context, accepted artifacts, and server lifetime. MCP supplies interoperable tool discovery and request/result envelopes; it does not supply the application's publication transaction or guarantee that a model repairs a rejected submission.

The current published core is **`2026-07-28`**, not `2025-11-25`. The official versioning page explicitly marks it current; the maintainers announced general availability on July 28, 2026. It removed `initialize`/`notifications/initialized`, protocol-level HTTP sessions, the independent GET stream, and standalone server-to-client JSON-RPC requests. Those are historical protocol features, not the current default. [Current versioning](https://modelcontextprotocol.io/docs/2026-07-28/learn/versioning), [July 28 GA announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28/)

A tools-only server avoids most optional protocol machinery. Build modern support for verified modern clients; add a legacy adapter only for client versions Quarterdeck deliberately supports. A server cannot upgrade an old client merely by announcing a newer protocol. The compatibility specification explicitly distinguishes modern, legacy, and dual-era implementations. [Version compatibility, revision 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)

## Evidence and dates

The revision date identifies the latest incompatible protocol change; current-revision documents can receive compatible corrections. Therefore this note records both the published revision and the exact repository state inspected. [Versioning policy](https://modelcontextprotocol.io/docs/2026-07-28/learn/versioning)

| Evidence | Date / identity | Use in this note |
| --- | --- | --- |
| [Published core release](https://blog.modelcontextprotocol.io/posts/2026-07-28/) | 2026-07-28 | Establishes that the modern protocol is released, not a prediction or release candidate. |
| [Core release tag](https://github.com/modelcontextprotocol/modelcontextprotocol/tree/2026-07-28) | Tag commit `5f5440bb26a62e2cf3440b92da5a667efa03b267`, 2026-07-28 | Release provenance. |
| [Inspected core source snapshot](https://github.com/modelcontextprotocol/modelcontextprotocol/tree/0175d20f8b2c49e0402c37c37ed39ae2be47277e) | Commit `0175d20f8b2c49e0402c37c37ed39ae2be47277e`, 2026-11-14 | Versioned current-spec prose and schema read for this note. |
| [Current changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog) | Revision 2026-07-28, checked 2026-11-15 | Changes since 2025-11-25. |
| [Draft specification](https://modelcontextprotocol.io/specification/draft) | Checked 2026-11-15 | Development material; not used as evidence that a feature is released. |
| [Tasks source snapshot](https://github.com/modelcontextprotocol/ext-tasks/tree/cc9aaac1047a9440b07486e38f37060bbebeef4c) | Commit `cc9aaac1047a9440b07486e38f37060bbebeef4c`, 2026-11-04 | Separately versioned optional extension. |

Some summaries and examples omit required metadata for readability. Follow the detailed transport, base-schema, and caching requirements when implementing; do not copy a short blog example as a complete wire contract. The current resources and MRTR pages explicitly warn about abbreviated examples. [Resources](https://modelcontextprotocol.io/specification/2026-07-28/server/resources), [MRTR](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr)

## Transport choice

| Choice | Protocol behavior | Quarterdeck assessment |
| --- | --- | --- |
| Streamable HTTP | Independent server; one MCP endpoint, POST per message; a request receives either one JSON response or a request-scoped SSE stream. | Natural fit for an app-owned listener and multiple independently launched agent processes. Short submission calls can use JSON responses. |
| stdio | Client launches a server subprocess; newline-delimited JSON-RPC on stdin/stdout; logs go to stderr. | Good if Quarterdeck ships a small helper that the CLI launches. Connecting that helper to the already-running app adds another IPC boundary. |
| Custom socket transport | Permitted if it preserves MCP framing and semantics. | An app can use a Unix socket internally, but CLI support for arbitrary custom MCP transports must be demonstrated. |
| Old HTTP+SSE | The 2024 transport uses separate SSE/POST mechanics. | Do not choose it for a new integration. |

Sources: [Modern HTTP binding](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http), [stdio binding](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio), [custom transport requirements](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports).

**Recommendation:** bind `127.0.0.1:0`, keep the bound socket, then use its assigned address in the agent configuration. Port zero avoids a find-free-port/rebind race. Make each endpoint's run context immutable. The randomized port is routing/isolation convenience, not authentication. A fresh unpredictable bearer secret should identify the authorized run.

A server using only JSON responses can still be a Streamable HTTP server. Its clients must accept JSON and SSE, but the server need not make every tool call a stream. SSE progress does not establish acceptance or publication of a review artifact. [HTTP sending/receiving rules](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http) In the selected v1 design, successful write tools update draft state; the host publishes later.

## Modern protocol: discovery, metadata, and capabilities

For revision 2026-07-28:

- Every request carries `params._meta["io.modelcontextprotocol/protocolVersion"]` and `params._meta["io.modelcontextprotocol/clientCapabilities"]`. Client identity is recommended, not an authorization credential.
- Every result has `resultType`; normal results use `"complete"`. Unknown discriminators are invalid. Modern clients must interpret an absent discriminator from an older server as complete.
- Missing required metadata is JSON-RPC `-32602`, with HTTP 400. A required but undeclared client capability produces `-32021`.

These are core wire requirements, not optional features. [Base protocol, revision 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic)

Servers must implement `server/discover`; clients may skip it and invoke another method directly. Discovery returns `supportedVersions`, server capabilities, optional instructions, and server identity in result metadata. It is ordinary discovery, not a session-opening transaction. Announce `tools: {}` for a static tools-only server. [Discovery](https://modelcontextprotocol.io/specification/2026-07-28/server/discover)

An unsupported version returns `UnsupportedProtocolVersionError`, code `-32022`, with `data.supported` and `data.requested`. The client should retry a mutually supported version. A dual-era server may support legacy initialization and modern requests concurrently. Modern metadata selects modern behavior; a legacy initialization selects the legacy flow. [Version negotiation and compatibility](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)

HTTP additionally requires `MCP-Protocol-Version`, `Mcp-Method`, and, for tool calls, `Mcp-Name`. Values must agree with the body; header names are case-insensitive, values are not. Missing or inconsistent required mirrored headers produce HTTP 400 / `HeaderMismatch` `-32020`. `Mcp-Name` also applies to prompt names and resource URIs. [HTTP metadata requirements](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http#request-metadata)

Modern complete results for discovery, tool/prompt/resource lists, resource-template lists, and resource reads require `ttlMs` and `cacheScope`. Recommendation: start with `ttlMs: 0` and `cacheScope: "private"`; avoid stale run-specific catalogs. These are JSON result hints, separate from HTTP cache headers. Tool calls are not among the listed cacheable operations. [Caching](https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching)

The tool list must not depend on connection history or a previous request's side effects. It may differ by per-request authorization. Keep names and ordering stable throughout a run; represent “already accepted” in a tool result rather than removing a tool after it succeeds. [Tool capabilities and discovery](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#capabilities)

Minimal modern discovery result for a tools-only server:

```json
{
  "jsonrpc": "2.0",
  "id": "discover-1",
  "result": {
    "resultType": "complete",
    "supportedVersions": ["2026-07-28"],
    "capabilities": { "tools": {} },
    "ttlMs": 0,
    "cacheScope": "private",
    "_meta": {
      "io.modelcontextprotocol/serverInfo": {
        "name": "quarterdeck-review",
        "version": "1"
      }
    }
  }
}
```

The shape follows [discovery](https://modelcontextprotocol.io/specification/2026-07-28/server/discover) and [caching](https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching); the identifiers are illustrative.

## Legacy compatibility must be an explicit adapter

In `2025-11-25`, the client first sends `initialize` with its supported version, identity, and capabilities. The server returns a version it supports and its capabilities; the client sends `notifications/initialized`. If the returned version is unsupported by the client, it should disconnect. HTTP uses the negotiated version header on subsequent requests. There is no standard protocol shutdown RPC. [Legacy lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)

Legacy Streamable HTTP may assign `Mcp-Session-Id`. Subsequent requests carry it; an expired session yields 404 and the client reinitializes. DELETE requests session termination but may receive 405. Servers may offer GET SSE or answer 405. Event IDs / `Last-Event-ID` can support replay. A disconnect should **not** mean cancellation in this era; cancellation is explicit. [Legacy HTTP transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)

Modern HTTP has none of those protocol sessions or SSE replay semantics. Therefore:

- Do not use a session ID as the review run ID, authorization proof, or storage key.
- Do not reuse one transport handler's cancellation behavior for both eras without version-aware dispatch.
- Do not interpret every HTTP 400 as evidence of a legacy server: modern version/capability/header errors also use it.
- If older CLI versions are supported, test their initialization and response shapes. Do not advertise a version whose full wire rules the server does not implement.

These are implementation recommendations derived from the [modern/legacy compatibility rules](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning) and the two HTTP bindings above.

## Submission and repair contract

Use the small application tool surface selected in [the main proposal](mcp-agent-bridge.md): context/read tools, bucket replacements, draft reads/validation, and `guide_finish`; brief page equivalents follow later. Those names are application tools, not standardized MCP methods. Draft reads and idempotent mutation receipts resolve uncertain tool acknowledgements.

The model's proposed brief belongs in **tool arguments** governed by `inputSchema`. The server's receipt and validation diagnostics belong in **tool results**, optionally governed by `outputSchema`. `structuredContent` is server-produced result data; it is explicitly unrelated to model schema-constrained generation. A tool's output schema requires the server to return conforming structured results; client validation is a SHOULD. Preserve a text representation for compatibility. [Structured tool results](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#structured-content)

JSON Schema 2020-12 is the default and required supported dialect. Avoid remote `$ref` dependencies: automatic network dereferencing is forbidden by the current base specification. Use local definitions and bounded object/array/string sizes. Schema validation catches shape, but membership in the frozen diff, line anchors, file coverage, duplicate groups, and revision identity remain Quarterdeck domain validation. [JSON Schema requirements](https://modelcontextprotocol.io/specification/2026-07-28/basic#json-schema-usage)

The important error boundary is:

| Condition | Result | Why |
| --- | --- | --- |
| Malformed JSON-RPC or malformed `CallToolRequest`; unknown method/tool | JSON-RPC error | Protocol dispatch failed. |
| Valid tool invocation with invalid submission shape/value or domain constraint | `result` with `isError: true` | Actionable tool error for the model to correct. |
| Valid draft replacement or finish operation accepted | `result` with `isError: false` | Acknowledges that operation; v1 publication remains pending. |
| Tool failure such as an unavailable dependency | Usually `isError: true`, with actionable classification | The tool ran and has a failure to report. |

The specification puts input validation and business-logic failures in tool errors. Clients should expose these to the model; clients only may expose protocol errors. Do not encode ordinary repair feedback solely as an HTTP error or JSON-RPC exception. [Tool error handling](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#error-handling)

Recommended result payload, with an `outputSchema` covering **both** accepted and rejected variants:

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "result": {
    "resultType": "complete",
    "isError": true,
    "content": [{
      "type": "text",
      "text": "{\"status\":\"needs_repair\",\"submission_id\":\"brief-1\",\"errors\":[{\"code\":\"anchor_out_of_scope\",\"path\":\"/pages/0/blocks/2/anchor\",\"message\":\"Anchor is outside the frozen review diff.\",\"repair\":\"Choose an anchor from the review context tools.\"}]}"
    }],
    "structuredContent": {
      "status": "needs_repair",
      "submission_id": "brief-1",
      "errors": [{
        "code": "anchor_out_of_scope",
        "path": "/pages/0/blocks/2/anchor",
        "message": "Anchor is outside the frozen review diff.",
        "repair": "Choose an anchor from the review context tools."
      }]
    }
  }
}
```

The application-defined fields above are illustrative, not MCP fields. Return bounded, precise diagnostics with paths, stable codes, and repair instructions. Avoid echoing the entire bad document. Reject an invalid replacement without changing the accepted draft unit or previous published artifact. Instruct the model to repair actionable issues and report unresolved problems; handle user cancellation and lack of progress explicitly. This does not impose a blanket agent-duration or attempt cap, and MCP does not guarantee model cooperation.

A successful tool result acknowledges the tool's documented operation, not persistence in general. In selected v1, `guide_finish` / `brief_finish` validate and seal the candidate, then return a readiness receipt with `publication_status: "pending"`. A successful CLI terminal result must reference that receipt. The host then verifies the active generation and atomically publishes the sealed artifact with final cost, model/session details, and full transcript. Completion is shown only after that host transaction succeeds. [Selected publication contract](mcp-agent-bridge.md#draft-publication-and-concurrency-rules)

An explicit tool that durably commits and returns a publication receipt is a valid later design, but is not v1. It would need to preserve the durable result even if the CLI subsequently fails and reconcile run metadata arriving after the commit. MCP permits both designs and does not define their transaction semantics.

## Request IDs, retries, and atomicity

MCP JSON-RPC IDs correlate requests and responses. Current rules require non-null string/integer IDs unique among outstanding requests; they do not define an application idempotency key. Even MRTR retries use a different JSON-RPC ID. [Base requests](https://modelcontextprotocol.io/specification/2026-07-28/basic#requests), [MRTR client requirements](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr#client-requirements-basic-workflow)

`idempotentHint` describes repeated calls with identical arguments having no additional effect. It is an annotation, not automatic deduplication or an exactly-once transaction service. Set it only if Quarterdeck implements that behavior. `readOnlyHint`, `destructiveHint`, and `openWorldHint` likewise describe behavior; they are not permission enforcement. [Tool annotations schema, inspected commit](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/0175d20f8b2c49e0402c37c37ed39ae2be47277e/schema/2026-07-28/schema.ts#L1912)

Recommended application contract:

1. Scope every mutation to an app-generated run/draft identity, immutable review snapshot, unit identity, and operation ID.
2. Validate against that run's frozen context. Do not let a submitted repository path or run ID select another run's data.
3. Atomically update the accepted draft unit and its operation receipt. Finish validates and seals the draft with a readiness receipt; it does not persist the published artifact.
4. The same key and same canonical payload returns the same receipt. The same key with different content returns a clear conflict, unless explicit replacement semantics exist.
5. Serialize conflicting revisions or use compare-and-swap expectations. Concurrent tool calls must not make acceptance order accidental.
6. If a draft-write or finish response disappears, an idempotent retry recovers its receipt while the run state exists. Process-memory drafts need not survive a crash; do not imply durability in their receipts.
7. After successful CLI completion with the matching sealed receipt, the host atomically verifies generation and publishes the artifact plus final run metadata. Host publication has its own durable receipt and retry protection.

The protocol's stateless transport does not prohibit application state. Its stateful-tool guidance recommends explicit handles; an authenticated handle names state and still requires authorization. For a per-agent server, immutable process/run context plus authenticated explicit identifiers is simpler than transport sessions. [Stateful tool guidance](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#stateful-tools)

MCP also does not prove which model, parent agent, or subagent authored a submission. `clientInfo` is self-reported; a shared bearer token identifies the authorized run, not an individual writer inside it. If only one orchestrator may publish, enforce that in the app's capability/credential design and tool exposure. A prompt saying “only the lead agent may submit” is guidance, not access control. [Identity trust limits](https://modelcontextprotocol.io/specification/2026-07-28/server/discover#data-types)

## Progress, notifications, cancellation, and shutdown

**Progress is optional and request-scoped.** The client supplies a `progressToken`; only then may the server report progress using it. Values must increase, and updates stop at completion. A server may send none. This measures execution of the tool, not the model's thinking or the whole review run. [Progress](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/progress)

Recommendation: use the existing agent transcript/stream for generation activity; send draft-acceptance and readiness events from the tool handler directly to Quarterdeck's UI. Label these as previews or pending publication. A quick validation tool does not need artificial MCP progress messages. Long validation could use progress if the real CLI supplies a token and exposes the updates usefully.

**Logging is deprecated** in 2026-07-28. Existing support uses per-request log-level metadata and request-related notifications; it is not a general push channel. New implementations should use stderr for a stdio helper or ordinary app observability. Do not send validation failures only as log messages. [Logging](https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/logging)

**Change notifications are opt-in.** Modern clients open `subscriptions/listen` with filters; the server acknowledges the supported subset and must not emit unrequested types. It replaces resource subscriptions and the independent HTTP GET stream. Request progress/logs belong to their own request, not this channel. No subscriptions are needed for a frozen review context and static tool set. [Subscriptions, inspected source](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/0175d20f8b2c49e0402c37c37ed39ae2be47277e/docs/specification/2026-07-28/basic/patterns/subscriptions.mdx)

**Cancellation differs by transport and era.** Modern HTTP's SSE disconnect is cancellation; stdio uses `notifications/cancelled`. Per-request timeouts are recommended, with a maximum even if progress resets that request's idle timer. Cancellation can race completion, and does not establish rollback of a committed business operation. [Cancellation](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/cancellation)

Modern HTTP does not support `Last-Event-ID` stream resumption. Reconnect is another request, so application-level receipts and idempotency must cover uncertain outcomes. The ephemeral server's death also ends access to any state stored only in its memory. [Modern HTTP receiving rules](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http#receiving-messages)

For stdio, graceful shutdown closes server stdin, waits for exit, then escalates termination if necessary; servers should exit on EOF. The client should restart an unexpectedly terminated server, which is another reason not to treat process memory as durable accepted state. [stdio shutdown](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio#shutdown)

For Quarterdeck's app-owned HTTP endpoint, use app lifecycle management:

1. Start listening and install the run's context before launching the CLI.
2. Keep per-request I/O deadlines and a user cancellation handle; detect and surface lack of progress explicitly without imposing an arbitrary whole-agent duration or attempt cap.
3. Acknowledge validated draft updates and finish readiness; keep the listener alive until the agent receives its result and completes or the run is cancelled. On successful terminal output with the matching readiness receipt, the host publishes the sealed draft with complete run metadata.
4. On cancellation, mark the run closed, reject new mutations, cancel pending work, and terminate the child within a bounded grace period.
5. Drain in-flight handlers and close the listener; remove temporary credentials/configuration; keep committed artifacts and diagnostic receipts as appropriate.

Do not close the endpoint from inside a successful tool handler before its HTTP response has been sent. Do not depend on a final agent call to clean up: the host owns cleanup when the agent crashes, is cancelled, or a detected stall is explicitly stopped.

## Tasks: published extension, optional, and maturity nuance

There are three different facts that should not be collapsed:

| Source | What it says as checked 2026-11-15 |
| --- | --- |
| [2025-11-25 Tasks page](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks) | Tasks were experimental in that core revision; client/requestor-driven, with the old capability and retrieval API. |
| [2026-07-28 changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog) | Tasks moved out of core to `io.modelcontextprotocol/tasks`, with a redesigned lifecycle. |
| [Current extensions overview](https://modelcontextprotocol.io/extensions/overview) | Tasks has lifecycle stage **“Beta, to Core.”** |
| [Tasks repository README at inspected commit](https://github.com/modelcontextprotocol/ext-tasks/blob/cc9aaac1047a9440b07486e38f37060bbebeef4c/README.md) | Its `2026-07-28` schema snapshot is labelled **“Stable”**; `draft` is development. |

Thus “Tasks is experimental” is accurate for the old core API, while “Tasks is an unconditionally stable core capability” is wrong for today's integration. A released schema snapshot and an extension lifecycle stage are different labels; source documents currently use both. Pin the exact extension revision and verify client support rather than relying on either adjective.

In the modern extension, the client declares support per request and the server may return `resultType: "task"` instead of a final tool result. Without that declaration, it must not. The server creates a durable handle before responding; `tasks/get` polls and includes the terminal result, `tasks/update` supplies requested input, and `tasks/cancel` requests cooperative cancellation. The old `tasks/result` / `tasks/list` API is not this protocol. [Tasks extension specification](https://github.com/modelcontextprotocol/ext-tasks/blob/cc9aaac1047a9440b07486e38f37060bbebeef4c/specification/2026-07-28/tasks.md)

A tool result with `isError: true` still makes a task `completed`; `failed` is for a JSON-RPC execution error. Task cancellation acknowledges intent, not guaranteed stoppage. IDs have retention/TTL constraints; neither “durable” nor reconnect means indefinite persistence. [Tasks overview](https://modelcontextprotocol.io/extensions/tasks/overview)

**Recommendation:** omit Tasks for submission/validation. The long-running activity is the external agent process; the MCP server is receiving a short result. Tasks would invert that responsibility and require durability, polling, and extra client support without improving the repair loop. Reconsider only if a tool itself starts substantial asynchronous work.

## Other capabilities and their limits

| Capability | Actual role | Decision for this integration |
| --- | --- | --- |
| Tools | Model-callable operations; result errors can enable repair. | Essential. Keep a small stable surface. |
| Resources | URI-addressed contextual data; the host decides how to expose/include it. | Optional for diffs, manifest, schemas. A `get_review_context` tool gives a simpler headless path if resource consumption is not verified. |
| Prompts | Server-authored templates intended for explicit user selection. | Optional reuse mechanism; not a guarantee that a headless client applies instructions. Put mandatory workflow instructions in the actual agent invocation. |
| Elicitation | Client-mediated user input, with form and URL modes, declared separately. | Avoid for unattended validation. Return machine-actionable tool errors for the agent; this is not a human-input problem. |
| Sampling | Server asks the client's model for a nested generation. Deprecated in current core. | Do not use to make the CLI “keep working.” The existing CLI agent already owns generation. |
| Roots | Client reports filesystem roots. Deprecated in current core. | Not a filesystem sandbox or authorization boundary. Pass scoped context explicitly. |
| Subscriptions | Client opts into change notifications. | Unnecessary for an immutable per-run catalog. |
| MCP Apps / skills / other extensions | Separately negotiated functionality. | Not required for native GPUI rendering or structured review submissions. |

Sources: [Resources](https://modelcontextprotocol.io/specification/2026-07-28/server/resources), [prompts, inspected source](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/0175d20f8b2c49e0402c37c37ed39ae2be47277e/docs/specification/2026-07-28/server/prompts.mdx), [elicitation](https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation), [sampling](https://modelcontextprotocol.io/specification/2026-07-28/client/sampling), [roots](https://modelcontextprotocol.io/specification/2026-07-28/client/roots), [extensions](https://modelcontextprotocol.io/extensions/overview).

Modern server-to-client interactions use **MRTR**: a tool/read/prompt request returns `resultType: "input_required"` with input requests; the client retries the original operation with responses and any opaque state. This replaces independent server JSON-RPC requests. It is not a generic unsolicited-message API, and servers must not assume the client will fulfill the input or retry. [MRTR](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr)

Sampling, roots, and logging remain specified during deprecation, with earliest removal in a revision on or after 2027-07-28. “Deprecated” does not mean already removed. Avoid designing new dependencies on them. [Deprecated-feature registry](https://modelcontextprotocol.io/specification/2026-07-28/deprecated)

## Local security: protocol requirements versus app policy

The current HTTP binding mandates Origin validation and HTTP 403 when a present Origin is invalid. It recommends loopback-only binding and authentication. It does not make a random port a security boundary. [Transport security](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http#security--endpoint)

**Host validation is also necessary engineering protection.** The Rust SDK's April 29, 2026 DNS-rebinding advisory fixed missing Host checks; it distinguishes that fix from Origin validation. Test the two independently. An open proposal from September 18, 2026 discusses revising normative Host/Origin language; as checked November 15, it remains open, so its proposed rules are not the released transport requirements. [Rust SDK advisory](https://github.com/modelcontextprotocol/rust-sdk/security/advisories/GHSA-89vp-x53w-74fx), [open Origin policy issue #3370](https://github.com/modelcontextprotocol/modelcontextprotocol/issues/3370)

Recommended local policy:

- Bind explicitly to IPv4 loopback; support `::1` only if deliberately bound and tested. Do not bind all interfaces.
- Accept only the configured endpoint path and exact expected Host authority. Reject an attacker hostname even if it resolves to loopback.
- Accept a missing Origin for the authorized non-browser CLI. Reject unexpected present origins, including `null`; no wildcard browser origins or permissive CORS are needed.
- Require a fresh, sufficiently random per-run bearer secret on **every** request, including discovery and legacy initialization. A token from run A cannot operate run B.
- Supply the secret through a private temporary config/header mechanism supported by the CLI. Keep it out of prompts, URLs, logs, committed configuration, and report artifacts; expire it when the run closes.
- Enforce body size, parsing-depth, concurrent-request, and per-request idle/total-time limits. These bound individual HTTP operations, not the whole agent run. Validate before mutation. A loopback client can still send malformed or abusive requests.
- Scope every allowed operation to the app's existing review context. Tool annotations, client name, run IDs, source paths, and reference content do not grant authority.

MCP authorization is optional; when implemented for HTTP, the specification recommends its OAuth-based authorization profile. A pre-shared ephemeral local bearer secret is a **Quarterdeck-specific authentication scheme**, not a complete implementation of that OAuth discovery/registration profile. Avoid claiming generic OAuth interoperability for it. The stdio authorization guidance instead uses environment credentials. [Authorization scope](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization)

These controls protect the intended local integration; they do not claim to defend secrets from every equally privileged or compromised process on the same user account. The app still controls the agent's filesystem/tool permissions separately from MCP.

## Compatibility and fault-injection checks before implementation is accepted

The protocol alone cannot prove that a specific CLI exposes structured results, tool errors, progress, subscriptions, or extensions to its model/UI. Use the installed CLI and the exact production flags. The companion [CLI research](claude-mcp-per-run.md) records concrete behavior and launch-mode constraints.

| Test | Required observation |
| --- | --- |
| Modern discovery and direct tool call | `server/discover` works; direct calls with valid metadata also work without initialization; discovery/list cache hints and complete discriminators are correct. |
| Wrong version | Server rejects before tool execution and returns advertised supported versions; actual CLI recovery is recorded. |
| Header/body disagreement | Wrong/missing version, method, or tool-name headers reject before side effects. |
| Legacy support, if promised | Exact supported older CLI initializes correctly, handles initialized notification, and gets legacy wire semantics. Modern failure must not silently execute legacy mutations. |
| Discovery under production flags | The actual restricted headless invocation lists and is allowed to call the intended tools. “Server connected” alone is insufficient. |
| Repair loop | Deliberately wrong anchor/membership produces `isError: true`; model-facing tool result contains diagnostics; the next call repairs it and receives acceptance. |
| Shape rejection | Invalid argument shape becomes actionable tool validation feedback when the `CallToolRequest` itself is well formed; malformed RPC remains a protocol error. |
| Structured result compatibility | Test structured-plus-text, structured-only, and invalid output-schema variants; keep structured-plus-text unless observed behavior proves otherwise. |
| Concurrent submissions | A delayed old revision cannot overwrite a newer accepted one; outcomes and conflict messages are deterministic. |
| Lost draft/finish acknowledgement | Apply the draft mutation or seal it, drop the response, then retry under a new JSON-RPC ID: unchanged draft state and the same operation/readiness receipt. |
| Host publication retry | Successful CLI output references the sealed receipt; host transaction publishes once with complete run metadata, and a repeated host operation recovers the durable receipt. |
| Duplicate key with different payload | Explicit conflict or documented replacement behavior, never accidental duplicate persistence. |
| Context drift | Submitted run/revision/file outside the frozen scope is rejected; one agent cannot affect another run. |
| Cancel before/during/after host publication | No new mutation after cancellation takes effect; a published result remains identifiable; UI distinguishes cancelled run from absent artifact. |
| Agent crash, cancellation, or explicitly stopped stall | Listener/config/secret are cleaned up without a final agent callback. Earlier published output survives; an unpublished draft does not become durable merely because a tool accepted it. |
| Server dies/restarts | Retry/status behavior is explicit; no assumption of preserved MCP session or automatic exactly-once delivery. |
| Host and Origin separately | Bad Host with no Origin fails; good Host with bad Origin fails; authorized CLI with no Origin succeeds. |
| Token checks | Missing/wrong/expired token fails even on loopback; run A's token fails against run B. |
| Resource bounds | Oversized body, deep JSON, many simultaneous calls, and slow requests remain bounded. |
| Optional features absent | Ordinary submission works with empty client capabilities; no surprise Tasks/MRTR/sampling/elicitation dependency. |
| End-of-run acknowledgement race | The listener remains alive long enough to deliver success; host cleanup does not require another model turn. |

Use protocol conformance checks in addition to this matrix, but do not substitute them for app/CLI tests. For example, the official conformance repository has an open September 28, 2026 issue explaining that a combined Host+Origin rejection test cannot prove the two defenses independently. [Conformance issue #535](https://github.com/modelcontextprotocol/conformance/issues/535)

## Implementation boundaries and remaining uncertainty

The minimal useful protocol surface is discovery, a static tool catalog, and synchronous tool calls with structured/text receipts. It does not need Tasks, sampling, roots, resources, prompts, long-lived notifications, or protocol-level sessions to support the proposed workflow.

The exact CLI-supported versions, flags, client capabilities, and model-visible error behavior are client findings, not spec guarantees. Likewise, choosing a Rust SDK requires checking its currently released protocol support and whether its runtime fits Quarterdeck's blocking, no-Tokio architecture; the July GA blog's statement that Rust support was beta is a historical statement about July, not a finding about November. [July SDK announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28/)

Two inspected-prose inconsistencies should not affect this minimal design: current task documents use different schema-versus-lifecycle maturity labels, and the cancellation page says a server must send a cancellation notification when tearing down a subscription while the subscriptions page recommends a successful final response for graceful closure. Avoid depending on subscription shutdown until the chosen implementation's behavior is verified against clarified guidance. [Cancellation](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/cancellation), [subscriptions source](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/0175d20f8b2c49e0402c37c37ed39ae2be47277e/docs/specification/2026-07-28/basic/patterns/subscriptions.mdx)

The key v1 guarantee to implement and test is: **finish seals a validated, revision-scoped draft; successful CLI output references its readiness receipt; the host then atomically publishes the artifact with complete run metadata.** Tool-operation retries and host-publication retries have separate receipts and durability semantics. MCP transports the authoring conversation; Quarterdeck enforces publication.
