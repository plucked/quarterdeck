# Discovering and messaging local Claude Code sessions

Research date: 2026-11-13. Scope: an external application or script on the same Mac, including sessions already running in a terminal or Claude Desktop. Research only; no messages sent, sessions launched, settings changed, or session credentials read.

## Finding

Both capabilities exist on the inspected machine. Discovery has a documented JSON command. Messaging has a documented Unix-domain inbox socket. Follow-up static inspection of the JavaScript embedded in version **2.1.323** establishes its text-message parser, registry lookup, and partial receipt protocol. Ordinary accepted messages do not receive a general success acknowledgement. The remaining uncertainty is runtime validation and compatibility with future versions, rather than an inability to inspect the current implementation.

## Discover active sessions

```sh
claude agents --json
```

This lists live interactive and background sessions. It also includes background sessions classified as working or blocked after their process exits, so distinguish a listed session from a live PID. Fields include `cwd`, `kind`, `startedAt`, and, when available, `pid`, `status`, `sessionId`, and `name`. Live statuses are `busy`, `waiting`, and `idle`; an idle process is still an active session. Background entries additionally have a short `id` and `state`.

`--cwd <path>` filters the results; `--all` includes completed background sessions. The interactive `claude agents` view focuses on background sessions and does not show ordinary foreground sessions until they are backgrounded. For an application, use the JSON command. These are current-user/configuration results, not a machine-wide inventory across other OS users, containers, and remote machines. [Agent view reference](https://code.claude.com/docs/en/agent-view#list-sessions-as-json)

An optional concise display, if `jq` is installed:

```sh
claude agents --json |
  jq -r '.[] | [.pid, .status, .kind, .name, .cwd] | @tsv'
```

Historical conversations and their transcripts are a separate inventory. `--resume` restores a conversation; it is not a general-purpose send command. There is a special case for running background sessions: a terminal invocation of `claude --resume <session> "prompt"` can send the prompt and attach. Piped input/output or incompatible flags make that path refuse instead. This is unsuitable as a universal external messaging API. [Session resume behavior](https://code.claude.com/docs/en/sessions#resume-a-running-background-session)

## Message an existing process

Claude Code documents a per-session inbox socket on macOS. Its built-in `ListAgents` and `SendMessage` tools use cross-session messaging; `/list-agents` or `/peers` shows reachable peers. Delivered messages reach a busy Claude between tool calls or start a turn when idle. They are peer messages, not permission approvals or executable slash commands. The inbox documentation explicitly covers scripts and hooks. [Cross-session messaging](https://code.claude.com/docs/en/cross-session-messaging)

For code launched by the receiving session, documented discovery is `CLAUDE_CODE_MESSAGING_SOCKET`. Claude exports its own address to Bash commands and hooks, including `SessionStart`. It also exports `CLAUDE_CODE_MESSAGING_TOKEN`; an optional first auth line on macOS can establish that a posting child belongs to that session. An external application should not treat itself as the session's child or collect that token merely to send a peer message. [Environment variables](https://code.claude.com/docs/en/env-vars)

### Message envelope observed locally

The installed executables contain an injection diagnostic with this JSON message line:

```json
{"type":"user","message":{"role":"user","content":"Hello from another local process"}}
```

The diagnostic pipes a newline-terminated message to `socat` using `UNIX-CONNECT`. Static inspection confirmed the same envelope in versions **2.1.321, 2.1.322, and 2.1.323**. The follow-up below traces the complete relevant handler in **2.1.323**. This is evidence from shipped code, not an end-to-end send test or a promise that the schema is stable. No messages were sent to a live session.

For a later disposable-session experiment, an external sender could serialize that object, append a newline, connect an `AF_UNIX` stream socket to the selected inbox, and send the bytes. This transport does not require owning the session's stdin or terminal. Successful socket writing is not confirmation that Claude received or acted on the message; the implementation's narrower receipts are described below.

### Locating another session's socket

On this machine, `~/.claude/sessions/<pid>.json` contains `sessionId`, `messagingSocketPath`, `entrypoint`, `version`, and other metadata. All six discovered processes had a corresponding record and an actual socket at the recorded path. The observed paths were `/tmp/cc-socks/<pid>.sock`, with permissions `0600`. `lsof` confirmed the terminal process owned its recorded socket.

The public `claude agents --json` output did **not** include the socket path. Joining its PID/session ID to the metadata record is a practical adapter for this version, but the metadata file is not an established public schema. Do not hard-code `/tmp/cc-socks`: obtain the actual path. A future adapter should recheck the session ID, PID liveness, and process identity before sending, and tolerate missing or changing records. Alternatively, a session-start hook could register the documented exported address with an application; that would require opt-in setup for subsequent launches.

### Delivery policy matters

`crossSessionInbound` can accept, hold, or refuse incoming messages. Without an explicit value, permission-mode rules decide; the existence of a socket alone does not prove automatic acceptance. Managed settings, explicit launch settings, user settings, and stricter project/local settings affect the result. A sender should report an unconfirmed submission separately from confirmed delivery, and should not change the receiver's policy as a side effect. [Settings reference](https://code.claude.com/docs/en/settings-reference#crosssessioninbound)

## Local evidence

The snapshot returned six sessions: one terminal-hosted CLI session and five Claude Desktop-hosted Code sessions. Five reported idle; one reported busy. All six were labeled `kind: interactive`, so that field alone does not distinguish Desktop from a terminal. The installed CLI was **2.1.323**, the running terminal process **2.1.322**, and the Desktop processes **2.1.321**.

Commands and read-only inspections used:

- `claude --version`, `claude --help`, and relevant subcommand help.
- `claude agents --json` and `claude daemon status`.
- `ps` and `lsof`, selected session metadata fields, and socket file types/permissions.
- Bounded reads around the inbox injection diagnostic in installed executable files.

`claude daemon status` reported no background supervisor, despite the six live interactive sessions. Therefore, supervisor state cannot substitute for active-session discovery. The CLI process's stdin/stdout/stderr pointed to an iTerm TTY. No tmux server was found at the default socket. Session names, project paths, IDs, and credential material are intentionally absent from this repository note.

## Other interfaces

| Interface | Relevance |
|---|---|
| Inbox socket | Directly targets an already-running local process; strongest fit for this research. |
| `claude attach` / agent-view reply | Useful interaction with managed background sessions. [CLI reference](https://code.claude.com/docs/en/cli-reference) |
| Channels | An MCP server can push external events into an opted-in running session. Requires channel setup and per-session enablement; useful for a designed integration. [Channels](https://code.claude.com/docs/en/channels) |
| Streaming stdin | `--input-format stream-json` with `--print` supports a process launched with that transport. It is not an instruction to rewire an existing terminal process's stdin. [CLI reference](https://code.claude.com/docs/en/cli-reference) |

## Follow-up: inspecting the installed implementation

The native executable is a macOS ARM64 Mach-O containing a Bun module graph. Parsing its `__BUN` segment recovered **3,000 `.js` modules**, with readable minified JavaScript alongside bytecode. None of the 3,253 module records had a source map. Thus the current implementation is inspectable, although the original TypeScript names/types are not preserved. The container layout was checked against [Bun's module-graph implementation](https://github.com/oven-sh/bun/blob/main/src/standalone_graph/StandaloneModuleGraph.rs).

Inspected executable: `/Users/plucked/.local/share/claude/versions/2.1.323`; SHA-256 `5432918d904a9b6afef7bb79d20df24d76052eabc1f41bceb84091d21f8889a7`. Its build metadata identifies revision `66b1ade1dce6269a9b608cea1920135b24a97a66`. Extraction and formatting were performed under `/tmp/claude-code-static-2.1.323/`; the executable and running sessions were not modified. No extracted application module was executed. The extracted source is kept out of this repository.

### Text-message parsing in 2.1.323

The `en` connection handler decodes UTF-8, parses newline-delimited JSON, and also parses a final nonempty fragment when the sender closes its write side. A connection without a complete line has a 30-second deadline. Its buffered string cap is 1,048,576 JavaScript string code units. Auth is optional on macOS; the default requires auth only on Windows.

The `ze` user-message handler checks these fields:

| Field | Observed behavior |
|---|---|
| `type` | Must be `"user"` to take this handler. |
| `message.content` | Must be a nonempty string. |
| `session_id` | Optional; when present, must exactly match the current target conversation. A mismatch drops the message. |
| `uuid` | Optional string used as the queued message's ID; otherwise generated. Separate from receipt correlation. |
| `msg_id` | Optional UUID-shaped string retained for receipt correlation. |
| `priority` | `"now"`, `"next"`, or `"later"`; unrecognized/missing values become `"next"`. |
| `from` | Sender address; missing becomes `"unknown"`. A usable `uds:` address is needed for socket receipts. |
| `msgV`, `message.role` | The native sender emits `1` and `"user"`, respectively; this receiving text handler does not validate either field. |

For a sender with its own callback socket, a candidate frame is:

```json
{
  "msgV": 1,
  "msg_id": "<new UUID for this submission>",
  "type": "user",
  "session_id": "<target conversation UUID>",
  "from": "uds:<sender's own listener path>",
  "priority": "next",
  "message": {"role": "user", "content": "A message for the existing session"}
}
```

The placeholders above must be replaced; omit `from` if there is no callback listener. This is a reconstructed interface from the current source, not a published TypeScript declaration. Other optional fields serve plugins/attachments and are outside this plain-text path. Inbound policy runs before queue admission. The queued object explicitly disables slash-command execution and automatic attachments.

Sources: [inbox handler `ze`](/tmp/claude-code-static-2.1.323/pretty/chunk-7thh7e5e.js:261), [connection parser `en`](/tmp/claude-code-static-2.1.323/pretty/chunk-7thh7e5e.js:597), [native sender `w0s`](/tmp/claude-code-static-2.1.323/pretty/chunk-k40ph57b.js:622), [message ID/version helper](/tmp/claude-code-static-2.1.323/pretty/chunk-qm8c645g.js:5).

### What receipts actually confirm

The sender's `Ae` function writes the frame and resolves when the connection closes. It has **no response-data handler**. The receiver also does not write a result onto that connection.

Instead, a separate connection to the frame's `from` address can carry a control message with `action: "peer_message_status"` and `orig_msg_id` matching the submitted `msg_id`. The receiver emits:

| Status | Meaning in the traced code |
|---|---|
| `held` | Parked by inbound policy. |
| `denied` | User declined a held message. |
| `expired` | Held message expired or was settled on shutdown. |
| `expired` with `status_detail: "refused"` | Receiver refuses cross-session input; normalized to `refused` by the receiving control handler. |
| `dropped` | Ingress rejected it, for example for rate limiting, duplication, a relay loop, or queue capacity. |
| `delivered` | A **previously held** message was released and admitted to the queue. |

The ordinary immediate-accept branch does not emit `delivered`. No universal model-read or task-completion acknowledgement was found in this local message path. A silent callback listener therefore does not establish success or failure. Earlier uncertainty about whether receipts exist is resolved: **they exist, but are partial and asynchronous**.

Callbacks require a valid reply address. The receiver checks that it belongs to its allowed socket namespace, rejects symlink targets, and, when it obtained the sender's OS process identity, checks that the callback listener belongs to that same PID, UID, and process generation. Consequently, a separate short-lived sender process and unrelated listener process are not interchangeable. These are transport identity checks, independent of the receiver's accept/hold/refuse policy.

Sources: [write/close behavior `Ae`](/tmp/claude-code-static-2.1.323/pretty/chunk-k40ph57b.js:820), [receipt callback](/tmp/claude-code-static-2.1.323/pretty/chunk-7thh7e5e.js:1542), [receipt explanations](/tmp/claude-code-static-2.1.323/pretty/chunk-7thh7e5e.js:1635), [immediate policy branches](/tmp/claude-code-static-2.1.323/pretty/chunk-mcwh1drr.js:279), [held release](/tmp/claude-code-static-2.1.323/pretty/chunk-mcwh1drr.js:428), [reply namespace validation](/tmp/claude-code-static-2.1.323/pretty/chunk-h3xwema8.js:23364).

### Registry lookup is implemented, too

The source confirms `<CLAUDE_CONFIG_DIR or ~/.claude>/sessions/<pid>.json` as its registry. Native `F`/`X` read numeric PID records and map `messagingSocketPath` to an internal socket field. `qBn` resolves a session UUID, handles a session moved into a background job, excludes spare/parked records, validates process identity, and probes socket reachability. The installed Desktop app independently reads the same registry.

This makes external lookup a reproducible implementation dependency rather than a guess. However, invoking internal lookup helpers is not a passive read: they can connect to sockets and clean stale records. An independent discovery adapter should use the public JSON list and read the corresponding metadata without invoking those internals. Stream `system/init` output can also include `messaging_socket_path` when the caller owns the process output.

Sources: [native registry reader](/tmp/claude-code-static-2.1.323/pretty/chunk-k40ph57b.js:986), [session resolver](/tmp/claude-code-static-2.1.323/pretty/chunk-k40ph57b.js:1205), [Desktop registry reader](/tmp/claude-desktop-static/cli-session-registry.pretty.js:48), [init event](/tmp/claude-local-registry/chunk-3jam9ybg.js:136).

For re-extraction after temporary files disappear, the executable contains these source ranges (byte offsets, zero based):

| Embedded module | Offset | Length | Relevant responsibility |
|---|---:|---:|---|
| `chunk-7thh7e5e.js` | 313495911 | 36036 | Inbox parser and receipt callback |
| `chunk-k40ph57b.js` | 285802505 | 24008 | Sender, receipt tracking, session lookup |
| `chunk-mcwh1drr.js` | 311221519 | 10883 | Inbound policy and held-message release |
| `chunk-qm8c645g.js` | 285802047 | 457 | Message version and UUID validation |

These conclusions are static implementation evidence for **2.1.323**. The earlier diagnostic-envelope checks also covered **2.1.321/2.1.322**, but their complete delivery paths were not traced in this follow-up. No live delivery or callback experiment has been performed.

## Suggested next experiment

Build a small discovery view around `claude agents --json`, then validate sending against one disposable session. Check idle delivery, busy delivery, hold/refuse behavior, exit/restart races, and how to observe a reply. Keep socket lookup and serialization isolated so changes to the undocumented portions remain local. The research supports feasibility now; it does not yet establish a reliable bidirectional integration.
