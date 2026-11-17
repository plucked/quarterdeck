# Claude Code: per-run MCP compatibility

Research date: 2026-11-15. Installed client: `/Users/plucked/.local/bin/claude`, **Claude Code 2.1.325**. Research only; no application code changed, production agent run started, credentials printed, or repository data sent to a model.

Evidence labels below distinguish **observed** behavior of this binary, **documented** contracts, and **proposed** integration choices. A synthetic Anthropic-compatible API running on loopback supplied scripted responses or rejected requests; it never forwarded them upstream. Every synthetic run reported `total_cost_usd: 0`. This establishes client behavior, not real-model compliance or output quality.

## Conclusion

A native Rust host can expose a small, per-run HTTP MCP endpoint to the existing Claude subprocess. Current **`--safe-mode` prevents that integration**, including an explicit `--mcp-config`. Adding a configuration alone will not work. A narrower isolation configuration passed bounded synthetic checks, but requires a deliberate compatibility gate before replacing the existing behavior. `--bare` is not a drop-in alternative: it changes authentication and the available built-in tools.

Modern MCP 2026-07-28 works on the installed binary. It also successfully falls back to 2025-11-25 when `server/discover` returns JSON-RPC method-not-found. See [the protocol research](mcp-protocol-current.md) for wire requirements and Rust SDK selection; this note covers the Claude client.

## Installed CLI and current runner

**Observed:** `claude --version`, `claude --help`, `claude mcp --help`, and `claude mcp add/add-json/get/serve --help` were inspected without calling account-status commands or reading stored credentials. The installed flags include:

- `--mcp-config <configs...>`: JSON strings or file paths; `--strict-mcp-config`: use only those configurations.
- `--tools`: restrict the built-in set; `--allowedTools`: permission grants.
- `--permission-mode dontAsk`, `--permission-prompts none`, `--restricted`, `--safe-mode`, `--bare`, and `--no-session-persistence`.
- `--forward-subagent-text`, `--include-partial-messages`, and `--output-format stream-json`.

The [official CLI reference](https://code.claude.com/docs/en/cli-reference) documents these distinctions. `--restricted` confines built-in file access and ignores user/project/local settings while retaining managed settings and explicit `--settings`; it is not a sandbox for application-defined MCP handlers.

The current [runner](../../crates/studio/src/run_claude.rs) uses restricted + safe mode, strict MCP configuration with no supplied servers, no permission prompts, read tools with optional `Agent`, a final JSON schema, and an appended guide. It imposes no spending cap. Its [event parser](../../crates/studio/src/claude_event.rs) recognizes initialization, partial tool starts/input, and final results; whole tool results and permission-denial events currently fall through as `Other` while remaining in the transcript. [Cancellation](../../crates/studio/src/stop_child.rs) sends SIGTERM, waits two seconds, then kills the child.

## Exact observed compatibility matrix

Every row used a fresh temporary cwd/config directory, synthetic credentials, a fresh loopback port, `--strict-mcp-config`, `dontAsk`, `--permission-prompts none`, and the same explicit HTTP server. Successful model turns were supplied by a local scripted API.

| Launch/input | Observed result on 2.1.325 |
|---|---|
| `--restricted --safe-mode --mcp-config <JSON>` | No MCP requests; `system.init.mcp_servers` empty; no MCP tool |
| `--safe-mode` without restricted mode | Same exclusion of explicit MCP |
| `--restricted` without safe mode | Connected server and exposed its tool |
| Inline JSON versus config file | Both expanded URL/token environment references and sent the expected Bearer header |
| `--restricted --bare` | MCP connected; only `Read` survived from requested `Read,Grep,Glob,Agent` |
| Narrower isolation flags below | Preserved `Agent,Grep,Glob,Read` plus explicit MCP |
| `--tools ""` plus MCP allow rule | No built-in tools, but MCP remained callable |
| MCP `readOnlyHint:false`, explicitly allowlisted | Executed under restricted + dontAsk; only synthetic memory was mutated |
| Same tool omitted from `--allowedTools` | Tool remained visible; no MCP dispatch; permission-denial events returned |
| Modern discovery supported | `server/discover` → `tools/list` → `tools/call`, all at 2026-07-28; no initialize |
| Modern discovery returns `-32601` | Fell back to `initialize` requesting 2025-11-25, then initialized/list requests |
| Wrong type in `structuredContent` versus `outputSchema` | Client returned an error tool result to its next model request |
| One general-purpose foreground subagent | Inherited and called the same MCP tool; only one server discovery/list occurred |

The modern HTTP requests carried `MCP-Protocol-Version`, `Mcp-Method`, and, for the call, `Mcp-Name: probe_echo`. Call metadata included client name/version, protocol version, `claudecode/toolUseId`, `progressToken`, and client capabilities `roots.listChanged` plus `elicitation.form`/`elicitation.url`. The legacy initialize advertised the same roots/elicitation capabilities. No sampling or Tasks capability appeared in this one-shot probe.

## Isolation replacement is a separate decision

**Observed candidate**, retaining existing tools, permissions, schema, guide, and authentication path:

```text
--restricted
--disable-slash-commands
--setting-sources ""
--settings {"claudeMdExcludes":["**"],"disableAllHooks":true,"autoMemoryEnabled":false}
--strict-mcp-config
--mcp-config <per-run config>
--permission-mode dontAsk
--permission-prompts none
```

Do not also pass `--safe-mode` or inherit `CLAUDE_CODE_SAFE_MODE=1` for this candidate.

A positive-control fixture contained parent, project, user, and nested `CLAUDE.md`; user/project custom agents and skills; user/project SessionStart hooks; and a locally registered installed plugin with a skill, agent, and SessionStart hook. Ordinary startup included memory/agent/skill/plugin sentinel strings and executed all three hook markers. A subsequent synthetic `Read` of a nested file loaded its nested instructions. The candidate included none of these sentinel strings before or after the same read and executed no markers, while preserving the explicit MCP server. A safe-mode control excluded the same fixture content but also excluded MCP.

The first plugin fixture lacked a marketplace registration and was inconclusive; the final fixture included the local marketplace, and its baseline positively loaded plugin content and ran its hook. Project and user skills initially shared a name, so the positive control established the resolved user skill rather than both simultaneously. The real runner omits `Skill` already; the final isolation control deliberately included it to verify `--disable-slash-commands` removes it.

**Documented limits:** `claudeMdExcludes` matches absolute paths but cannot exclude managed policy memory. `disableAllHooks` outside managed settings leaves managed hooks and hooks from force-enabled managed plugins running. Managed policy must remain effective. The synthetic test did not modify system-managed configuration, seed actual auto-memory storage, or exercise real OAuth credentials. Thus this is a tested candidate for user/project customization isolation, not a proof of safe-mode equivalence on every managed machine. [Settings reference](https://code.claude.com/docs/en/settings-reference#claudemdexcludes), [hook setting](https://code.claude.com/docs/en/settings-reference#disableallhooks).

`--bare` explicitly avoids OAuth/keychain authentication, requiring API-key/helper or provider authentication, and disables background work. Our local probe also lost Agent/Grep/Glob. Preserve the user's existing login flow; do not adopt bare mode simply because MCP works under it. [Headless bare-mode documentation](https://code.claude.com/docs/en/headless#start-faster-with-bare-mode).

## Per-run connection and authorization

**Proposed configuration**, with syntax and interpolation observed locally:

```json
{
  "mcpServers": {
    "quarterdeck_run": {
      "type": "http",
      "url": "${QUARTERDECK_MCP_URL}",
      "headers": {
        "Authorization": "Bearer ${QUARTERDECK_MCP_TOKEN}"
      }
    }
  }
}
```

The host binds `127.0.0.1:0`, obtains the chosen port, starts accepting requests, and sets only the child environment's run URL/token. Supplying the JSON as one `Command::arg` avoids a config-file lifecycle; environment references keep the token out of argv. A private temporary configuration file also worked. Avoid `claude mcp add`, which persists installation state and is unnecessary for one run. Do not put credentials in the endpoint URL or application logs. Bearer authorization on HTTP is supported by the installed `mcp add --help`; interpolation worked in both explicit config forms.

Keep a stable, simple server key so permission names remain predictable. Explicitly allow each intended capability, for example:

```text
--tools Read,Grep,Glob,Agent
--allowedTools Read,Grep,Glob,Agent,mcp__quarterdeck_run__review_context,mcp__quarterdeck_run__guide_validate,mcp__quarterdeck_run__guide_finish
```

This abbreviated example illustrates permissions, not the complete proposed tool surface. `guide_finish` freezes a valid draft and returns a validation receipt with publication pending; durable publication remains a host operation after the successful CLI result echoes that receipt. An allow rule authorizes execution; it does not restrict discovery. The server should advertise only the run's tools. The existing no-workspace `--tools ""` path would gain MCP tools when supplied, so its current “no tools” assumption must be revisited deliberately. MCP permissions accept full names or `mcp__server__*`; narrower names are preferable when mutation authority differs. Tools marked `anthropic/requiresUserInteraction` are denied in `dontAsk` even if allowlisted. [Permission rules](https://code.claude.com/docs/en/permissions#mcp).

**Host responsibility:** restricted mode did not prevent the synthetic mutation tool. Validate paths, IDs, revisions, payload bounds, and draft transitions inside the Rust handlers. Claude's file confinement does not constrain what those handlers can access. A run token is an application capability, not a substitute for these checks.

## Discovery and tool visibility

For a small fixed set, **propose `ENABLE_TOOL_SEARCH=false`**, which was used successfully in the scripted-call probes. This removes a discovery step and makes the entire schema visible up front. Current tool search is enabled by default; it can defer tools and adds a model round trip when searching. The old assumption that search is enabled only after a 10% threshold is no longer the default (`auto` selects that behavior). [Tool-search documentation](https://code.claude.com/docs/en/agent-sdk/tool-search).

Explicit servers can still connect asynchronously with tool search. The SDK documentation describes startup waits and the server-level `alwaysLoad:true` option, which waits up to its first-turn cap and loads schemas upfront. An init event is a snapshot, not necessarily final connection status. For this integration, bind before spawn, disable deferral for the tiny tool set, and treat missing required MCP tools/server failure as a run failure rather than allowing a plausible answer without validation. [MCP connection timing](https://code.claude.com/docs/en/agent-sdk/mcp#connection-timing).

## Result, error, and stream behavior

**Observed with scripted tool calls:**

- A successful result contained text `synthetic text result` and `structuredContent:{"ok":true}`. The next model request used the serialized structured JSON as its tool-result content, replacing that text. The stream's `user.tool_use_result` retained `structuredContent` as an object.
- `isError:true` produced `user.message.content[].is_error:true`; the model received the text diagnostic. Structured error data was not preserved in the same success shape. Put actionable repair information in the error's text content.
- A schema-declared boolean returned as a string produced `Structured content does not match the tool's output schema: data/ok must be boolean` as an error tool result.
- An unallowlisted call emitted `system/permission_denied`, a user error tool result, and a final `permission_denials` entry. The scripted model still finished successfully. Therefore a successful final run does not prove every required operation happened.
- Tool starts appeared in partial `stream_event/content_block_start` events with the fully qualified name. Complete user results carry `tool_use_id`, and successful results also included permission-decision metadata.

Anthropic's [official changelog](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md#2021) records structured-content support since 2.0.21. The client behavior above was verified directly rather than inferred from that release note.

**Integration consequence:** require a server-issued receipt for the sealed draft and a successful final CLI result referencing it; the host then publishes and records complete run metadata. Keep `--json-schema` for the final answer envelope if useful, but it does not validate application state transitions. Extend progress parsing to correlate tool calls/results and permission failures if the UI needs validation or repair status. Main-session token deltas are distinct from complete messages; subagent attribution uses `parent_tool_use_id`, and `--forward-subagent-text` can include delegated text. [Streaming-output documentation](https://code.claude.com/docs/en/agent-sdk/streaming-output).

## Subagents and concurrent access

**Observed:** a scripted foreground `Agent` call of type `general-purpose` spawned one child. The child saw `Agent,Grep,Glob,Read` and the MCP tool, called the same endpoint with its own `claudecode/toolUseId`, and returned successfully. The server was discovered only once. This disproves any assumption that adding MCP only gives the parent mutation authority.

**Documented:** subagents inherit the available MCP pool and permission rules; a custom agent's `tools`/`disallowedTools` narrows it. Named server references share the parent's connection; inline server definitions have their own lifecycle. Background subagents retain MCP tools too. [Subagent capability documentation](https://code.claude.com/docs/en/sub-agents#available-tools).

Read-only calls can run concurrently; mutation tools normally run sequentially within an execution loop. Set truthful `readOnlyHint` annotations. Do not interpret this scheduler as a cross-subagent transaction lock: separate agents can access one run state. Serialize or compare-and-swap revision-changing handlers in the host. Consider permitting only read/validation tools to explicit custom subagents, with draft-finalization authority confined to the main workflow and durable publication confined to the host. [Agent-loop parallel execution](https://code.claude.com/docs/en/agent-sdk/agent-loop#parallel-tool-execution).

## Ownership, cancellation, and optional features

HTTP suits an in-process Rust service: Quarterdeck owns the listener/state; Claude owns a client connection. For stdio, Claude launches and stops the configured child command, requiring a separate helper or application subprocess mode. Its documented Unix shutdown sequence is SIGINT, 100 ms, SIGTERM, 400 ms, then SIGKILL. [MCP stdio lifecycle](https://code.claude.com/docs/en/mcp#how-claude-code-stops-a-stdio-server).

On SIGTERM, a headless run exits 143 without a final result; a one-shot run does not wait for background MCP work. Application cancellation must close the run to further mutations, abort active handler work, revoke its token, and stop its listener independently of whether the client sends a protocol cancellation. A dropped HTTP connection is not proof that an operation did not finish. [Headless shutdown documentation](https://code.claude.com/docs/en/headless#stop-a-run-with-sigterm).

| Feature | Evidence and applicability |
|---|---|
| Tools / input schemas / structured results / output schemas | Modern path, authorization, errors, and schema mismatch observed; sufficient for the proposed first version |
| Resources | Legacy probe advertised resources and received `resources/list`; existing built-in allowlist exposed no resource-reading tool. Do not assume resource links are automatically usable under this whitelist |
| Elicitation | Form/URL capabilities observed; unattended handling was not exercised. Avoid human-interaction dependencies in this run contract |
| Progress | `progressToken` observed in calls; notification-to-stdout/UI behavior unverified. Keep app progress authoritative or probe explicitly |
| Tasks | Current docs exclude one-shot text-input `-p` runs; stream-json **input** with open stdin and specific background flags is a different lifecycle. Do not confuse stream-json **output** with enabling Tasks |

Resources, elicitation, task requirements, and automatic backgrounding are described in the [Claude MCP reference](https://code.claude.com/docs/en/mcp#what-a-call-needs-to-become-an-mcp-task). Its feature-flag restriction for Tasks conflicts with a newer changelog entry saying terminal Tasks can operate without flag fetching; the one-shot exclusion is still explicit. Treat feature availability as version/session dependent, not a protocol-wide guarantee. The installed client and [official changelog](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md) should be checked together.

## Reproduction and remaining compatibility gate

No-charge probe recipe:

1. Create a fresh temporary working directory and `CLAUDE_CONFIG_DIR`; retain the real `HOME` without reading or modifying it. Launch the CLI with a whitelisted environment, fake `ANTHROPIC_API_KEY`, and `ANTHROPIC_BASE_URL=http://127.0.0.1:<port>/api`. Disable telemetry/nonessential traffic. Never forward API requests.
2. Serve the synthetic `/mcp` endpoint and reject `/api/v1/messages` to test startup only. For call tests, return valid local Anthropic SSE events containing a fixed `tool_use`, then a fixed final text response. Capture only synthetic tool names/results and sentinel presence; do not log real credentials.
3. Test modern `server/discover` with `resultType:complete`, version/capabilities, and private zero-TTL discovery/list caching metadata. Test fallback separately by replying `-32601` to discovery and accepting legacy initialize.
4. Vary safe/restricted/bare flags, explicit config string/file, allow rules, tool annotations, result types, and output-schema mismatch. Set process-group timeouts and clean up each fixture.
5. For customization isolation, seed known sentinel instructions, registered local plugin components, and marker-only hooks. Require a positive baseline before concluding suppression. Force a nested Read and compare the next synthetic API request.

The smallest real-model compatibility probe still needed is one isolated synthetic workspace with a deliberately invalid draft: read context, call validate, receive a deterministic repairable `isError` diagnostic, repair, validate again, and finish exactly one revision. Verify that the final successful CLI result echoes the sealed validation receipt, then have the host publish once with complete cost/session/transcript metadata. Verify the final schema, server audit, and durable receipt agree. A separate two-child case should test conflicting draft updates, cancellation during a delayed call, and no writes or publication after cancel. Also exercise the intended real login method, managed-policy constraints, and future supported client versions. No paid model probe was run for this research, and this note does not impose a production dollar cap.
