# AI-assisted review in Quarterdeck: a native Review Studio

Research and design proposal, 8 November 2026. Nothing here is implemented.

## Summary

Quarterdeck can run `claude -p` and `codex exec` against a pull request without much effort. The work worth designing is what the agent can *say* and where it shows. The proposal:

1. **A closed, typed widget vocabulary** (about 15 kinds) that the agent composes into a studio, modelled on the components the Review Studio skill already proved out (lead, callouts, code comparison, compact tables, traces, diagrams, terms, inspectors), plus the judgement widgets a reviewer needs (findings, attention levels, a review tour, verification status).
2. **The agent references code; the app quotes it.** Every widget points at code through *anchors* (path, side, line range, pinned commit). Quarterdeck renders excerpts from the diff it already has, validates every anchor, and rejects the ones that do not resolve. The model never pastes code into the studio, so it cannot misquote it.
3. **The studio is woven into the existing review surfaces**, not a separate report: a Brief pane beside the diff, AI cards in the diff rows (like comment cards), attention badges and a tour in the file tree, and findings that the reviewer promotes into the existing GitHub-draft or local-note destinations. The AI never posts to GitHub.
4. **Quarterdeck hosts an MCP server** that both CLIs connect to. Read tools serve the cached PR data (no extra GitHub requests, no checkout needed); write tools add widgets one at a time, each validated and rendered as it arrives, so the studio builds up live and the agent fixes its own errors. A final `--json-schema` / `--output-schema` result is the fallback.
5. **Everything is stored in typed SQLite tables**, keyed by the head SHA, which the closed vocabulary makes possible (the repository rule: never JSON).

## 1. What the Review Studio skill already settled

The `exec-review-studio` skill (seed, seed-gnosis, library-eco) is an *explanatory* workflow: it helps a developer understand a change, then inspect its evidence. Its decisions transfer almost unchanged.

| Skill rule | Consequence for Quarterdeck |
|---|---|
| Lead with the changed behavior, then context → mechanism → constraints → evidence | The Brief opens with a `lead`; sections follow that order. |
| Choose the representation from the review question (`representations.md`) | The vocabulary has one widget per reader task: `code_compare` + `trace` for a small fix, `graph` for ownership/flow, `sequence` for concurrency/protocol, `change_map` for broad changes. The prompt carries that table. |
| Keep change size and explanation size aligned | Presets scale: a one-line fix gets a lead, a comparison and a trace; a 27-PR stack gets a map and per-group pages. |
| Main explanation visible; inspectors and definitions are details on demand (Shneiderman) | The diff *is* the inspector: anchors scroll it. Terms open popovers. Nothing gates reading behind a Next button. |
| Claim ledger: claim → source revision/path/lines → status | Anchors are the ledger, attached to the widget they support, validated by the app. |
| Separate implementation facts, author-reported results, examples and open questions | A `verification` widget with explicit status (ran here / author-reported / not run); `callout` tones for evidence vs. qualification. |
| `validate_report.py`: broken anchors, undefined terms, template mismatches are errors | The MCP write tools run the same checks per widget and return errors to the agent. |
| Name every arrow with what it transfers; semantic color is a second cue | `graph` edges require a label; node roles (actor, state owner, transport, store, constraint) map to theme colors. |
| Language rules (no sales words, headings name the mechanism) | Prompt text, plus a lint on headings and banned words. |

What does **not** transfer:

- **Free-form HTML and hand-drawn SVG.** gpui has no web view, and free markup could not link into the diff, follow the eight themes, or be stored as typed columns. Diagrams become graph specs that Quarterdeck lays out and draws.
- **Publishing to `share.seed.ci`.** The studio is private to the reviewer. Because the vocabulary maps one to one onto the skill's components, an *Export as Review Studio* action can later write the skill's `report.json` + fragments for sharing, so the two stay one system.
- **Scope.** The skill explains; it deliberately does not judge. A review app must also help the reviewer decide, so Quarterdeck adds findings, attention levels and a tour, kept visually distinct from the explanation.

## 2. Landscape

### What review products show today

| Product | Presentation | Worth taking |
|---|---|---|
| [CodeRabbit](https://docs.coderabbit.ai/pr-reviews/walkthroughs) | Walkthrough comment: per-file change summary, Mermaid sequence diagrams only when interactions change, review effort 1–5, merge risk. [Change Stack](https://docs.coderabbit.ai/pr-reviews/change-stack.md) (preview): the PR regrouped into numbered layers in "the order someone would explain the change", with dependencies between layers, an overview with "Needs your attention", chat from a layer/file/range, pinned to one head | Guided order with explicit dependencies; snapshot pinned to a head |
| [Claude Code Code Review](https://code.claude.com/docs/en/code-review) | Find (parallel agents) → verify → rank → post. Markers: Important, Nit, **Pre-existing** (bug not introduced by the PR). Each finding has a collapsed "Why this was flagged". Re-reviews cover only what changed, say how many nits were left out, resolve fixed threads. Never approves or blocks | Pre-existing as its own class; collapsed evidence; verify step; never decide |
| [Cursor Bugbot](https://cursor.com/blog/building-bugbot) | A validator model filters candidates. [Learned rules](https://cursor.com/blog/bugbot-learning) from downvotes, replies and human comments, activated after enough signal, editable. Measures whether a flagged bug was fixed by merge | Feedback becomes editable rules; measure "acted on" |
| [GitHub Copilot review](https://docs.github.com/en/copilot/concepts/agents/code-review) | Overview with an approval assessment, inline comments with one-click fixes, effort levels (Lite ≈ $0.25 … Max ≈ $10 per review), resolves its own addressed comments | Visible cost per effort level |
| [Graphite Agent](https://graphite.com/blog/introducing-graphite-agent-and-pricing) | Inline review plus chat on the PR page; highlight lines and ask. Tracks whether suggestions get committed; thumbs had too few responses to be a metric | Ask about a selection |
| [Greptile](https://greptile.com/docs/code-review/greptile-json-reference) | Summary with confidence 0–5, P0–P2 badges, a sequence diagram or flowchart chosen by what changed (none for trivial PRs), `strictness` 1–3 | Diagram type follows the change |
| [Qodo PR-Agent](https://github.com/Codium-ai/pr-agent), [Sourcery](https://docs.sourcery.ai/reviews/anatomy-of-a-review/), [Ellipsis](https://docs.ellipsis.dev/features/code-review) | "Reviewer guide": effort, focus areas, per-file table, sequence diagram; confidence-ranked suggestions; a confidence threshold for posting | Focus areas, thresholds |
| [Codex on GitHub](https://learn.chatgpt.com/docs/third-party/github), [Gemini Code Assist](https://developers.google.com/gemini-code-assist/docs/customize-gemini-behavior-github) | Codex posts only P0/P1; rules in `AGENTS.md`. Gemini has a severity threshold (default MEDIUM) and a style guide file | Suppress low severities by default |
| Devin Review (Jan 2026, [secondary source](https://www.testingcatalog.com/devin-launches-ai-powered-code-review-tool-for-github-projects)) | Related hunks grouped and ordered for reading | Same as Change Stack |

Every one of these is a **comment poster**: its UI is GitHub's comment thread, Markdown and Mermaid. CodeRabbit's Change Stack and Devin Review are the only ones that leave the thread and build a reading environment, and they arrived at the same idea as the Review Studio skill: regroup the change into an explained order. None of them can tie an explanation to a native diff viewer, theme it, or keep it local. That is the gap a desktop reviewer can fill.

### What research says reviewers need

- Developers expect review to find defects, but what it mostly delivers is understanding, knowledge transfer and awareness. *Understanding the change* is the main challenge, and tools support it poorly ([Bacchelli & Bird, ICSE 2013](https://www.microsoft.com/en-us/research/publication/expectations-outcomes-and-challenges-of-modern-code-review/)). That argues for the Brief before findings.
- Splitting a change into related parts by def-use links was judged useful by authors; 66% of changed methods fell into non-trivial parts ([Barnett et al., ICSE 2015](https://www.microsoft.com/en-us/research/publication/helping-developers-help-themselves-automatic-decomposition-of-code-review-changesets/)). An optimal review order is mostly a grouping by relatedness, which is a partial order rather than a single list ([Baum et al., ICSME 2017](https://tobiasbaum.github.io/rp/optimalordering.pdf)).
- Position matters. Across 330k PRs, the latent bug rate rises from 56.7% at file position 1 to 61.5% at position 30, and each extra file lowers the chance that a file gets any comment by about 8.7% ([Rahman et al., 2026](https://arxiv.org/abs/2609.22610)). Their recommendations are dependency-aware grouping, risk-aware ordering and per-file coverage indicators. Those are the `tour`, the `attention` levels, and the viewed marks Quarterdeck already has.
- LLM comments are acted on when they are concise, include code, are asked for manually, and are anchored to a hunk ([Sun et al., 2025](https://arxiv.org/abs/2508.18771)). In 31k CodeRabbit reviews, 56% were rejected, mostly for false positives, redundancy, out-of-scope feedback and conflict with the author's intent or the project's conventions ([Lin et al., 2026](https://arxiv.org/abs/2607.03316)).
- AI comments can slow teams down. At Beko, 74% of PR-Agent comments were addressed, but PR closure time rose ([Cihan et al., ICSE-SEIP 2025](https://arxiv.org/abs/2412.18531)). PRs reviewed only by agents merged far less often than human-reviewed ones, and 12 of 13 review agents had a signal ratio below 60% ([Chowdhury et al., 2026](https://arxiv.org/abs/2604.03196)). Findings should therefore be drafts for the human, never posted on their own.

### Protocols for agent-composed UI

| Protocol | Shape | Usable natively? |
|---|---|---|
| [A2UI](https://a2ui.org/specification/v1.0-a2ui/) (Google, v1.0) | JSONL messages `createSurface` / `updateComponents` / `updateDataModel` / `deleteSurface`. Components form a **flat list keyed by id**, children referenced by id, one `root`. Missing references render as placeholders, so parts can stream in any order. The catalog is a JSON Schema with an `instructions` field for design guidance. Actions return to the agent with a `context`; functions can require a user gesture | **Yes.** It has SwiftUI and Compose renderers but none in Rust. Its structure is the model to copy |
| [MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview) (SEP-1865) | Tools point at `ui://` HTML resources rendered in a sandboxed iframe; JSON-RPC over postMessage; tools visible only to the app; consent for UI-started calls | No: needs a web view. The ideas (data apart from template, consent) carry over |
| [json-render](https://json-render.dev/docs) (Vercel) | `{root, elements: {id: {type, props, children}}}` streamed as JSONL patches; Zod catalog | Same flat-by-id idea; web renderers only |
| [AG-UI](https://docs.copilotkit.ai/ag-ui/sdk/js/core/events) (CopilotKit) | An event transport (tool-call start/args/end, state snapshots and JSON Patch deltas), not a widget format | Only as a transport idea |
| Claude artifacts | Model-written HTML/React in an iframe, no catalog | No |

The useful lesson from A2UI is the **flat, id-keyed widget list with a catalog the model reads**. The studio's `studio_add(widget)` with `page` and `position` already follows it, and the vocabulary's JSON Schema can carry the *when to use which* guidance the same way A2UI's `instructions` field does. Full A2UI compliance buys little: a studio is read-mostly and needs no two-way data binding, and its value comes from Quarterdeck-specific anchors that no generic catalog has.

## 3. Design principles

1. **The reviewer stays the reviewer.** The agent explains, points and asks. It never approves, never posts, never resolves. Findings are drafts the human accepts, edits or dismisses. No suggested verdict button (it invites rubber-stamping); a `readiness` note at most.
2. **Understand first, judge second.** The Brief explains the change before listing findings. Findings without the mental model behind them are what makes bot reviews noisy.
3. **References, not quotes.** Code shown in the studio is cut from the pinned diff or file text by Quarterdeck. A widget whose anchor does not resolve is rejected, not shown with a warning.
4. **Every claim is one click from its code.** Each widget carries anchors; clicking one scrolls the diff, tints the lines and keeps a way back.
5. **Silence is a valid result.** No findings is a normal outcome. Nits are off by default; findings carry confidence and a short evidence trail; there is a per-run budget.
6. **Pinned to a head.** A studio belongs to one base/head pair. New commits mark it stale and an update re-reads only files whose patch changed (the existing `PatchFingerprint`).
7. **Cheap on GitHub, explicit on cost.** The agent reads Quarterdeck's cache through MCP. Runs start on a click (or an opt-in rule per repository), show progress in the header pill, and record tokens/cost.

## 4. Where the studio lives

```
┌ Sidebar ────────┬ Header: title · meta · stack bar · [Diff | Brief] · ◉ AI 62% ────┬ Side panel ───────────┐
│ Mine Review …   │                                                                  │ Files | Review | AI   │
│                 │  BRIEF (when Brief is picked)                                    │                       │
│ ▸ seed          │  Lead: "Map writes keep the whole-write requirement of maps      │ Tour (4 steps)        │
│   #704 Map …  ● │  they leave out."                                                │  1 ▸ Guard in template│
│                 │                                                                  │  2   Regression test  │
│                 │  ## Clear tracking only for a field included in the write        │  3   Unsent persist   │
│                 │  prose with [terms] and [anchors]                                │                       │
│                 │  ┌ code_compare  before @base  │  after @head ───────────────┐   │ Findings (2)          │
│                 │  │ map?.ClearChanges();        │ if (MapIsDirty) {           │   │  ● concern  high      │
│                 │  └─────────────────────────────┴─────────────────────────────┘   │    Authority path …   │
│                 │  ┌ trace: Action | Before | After ───────────────────────────┐   │  ○ question medium    │
│                 │  └───────────────────────────────────────────────────────────┘   │                       │
│                 │                                                                  │ Attention             │
│                 │  DIFF (when Diff is picked): existing rows, plus AI cards        │  focus 3 · read 5 ·   │
│                 │  under anchored lines and a gutter mark                          │  skim 12 · mech. 40   │
└─────────────────┴──────────────────────────────────────────────────────────────────┴───────────────────────┘
```

- **Brief pane.** A `[Diff | Brief]` segmented control in the sticky header swaps the diff column for the studio (key `b`). It is a virtualized `list` like the diff, so a large studio costs nothing off screen. Clicking an anchor switches to Diff at that line, with a *Back to Brief* chip in the header.
- **AI tab in the side panel.** Stays visible while reading the diff: the tour, findings (filter by severity), attention summary, run status and the *Ask* history. `]`/`[` follow the tour while a tour is active.
- **In the diff.** Findings and answers become a new `DetailRow` kind placed by the same `CommentPlacement` as comment cards, styled apart (AI label, no avatar, confidence pill). A gutter mark shows where cards are folded. A switch hides all AI rows.
- **In the file tree.** Attention levels as a column or badge (`focus` / `read` / `skim` / `mechanical`), and a *By intent* grouping that follows the `change_map` groups instead of folders. The existing Kind filter (code/tests/generated/moved) already covers part of `mechanical` without the AI.
- **Stacks.** A studio can have pages; for a stack each page binds to a layer, and the combined review gets an overview page with a `change_map` across layers (the shape of the M3 report).

## 5. The widget vocabulary (v0)

All widgets share: `id` (agent-chosen, stable within a run), `page`, optional `anchors[]`, optional `title`. Prose fields are the Markdown subset `qd-markdown` already parses; two link schemes are intercepted by the app instead of opening a browser: `[world thread](term:world-thread)` and `[the guard](anchor:a3)`.

### Narrative

| Widget | Fields | Rendering | From the skill |
|---|---|---|---|
| `lead` | `text` (1–3 sentences) | Large lead paragraph at the top of the page; also the one-line summary in the AI tab | `.lead` |
| `section` | `heading`, `body` | H2 + Markdown; headings must name a mechanism or consequence | page sections |
| `callout` | `tone`: `note` / `constraint` / `verdict` / `evidence`, `text` | Tinted block; `constraint` needs cause and effect | `.note`, `.callout.constraints`, `.verdict`, `.proof` |
| `term` | `key`, `name`, `definition`, optional `anchor` | Dotted underline in prose; hover/focus popover (dismissible, hoverable); glossary list in the AI tab | `data-term` + glossary |

### Code

| Widget | Fields | Rendering |
|---|---|---|
| `code_excerpt` | `anchor`, `caption`, `emphasis` lines | Lines cut by the app from the diff or file text at the pinned commit, drawn with the diff cell renderer and syntax colors |
| `code_compare` | `before` anchor (base), `after` anchor (head), `caption` | Two columns aligned like a split row; must be from one logical location | `.code-compare` |
| `suggested_change` | head `anchor`, `replacement` text, `rationale` | Mini diff of replacement vs. anchored lines; promotable to a GitHub ```` ```suggestion ```` block, so only on patch lines |

### Structure and diagrams

| Widget | Fields | Rendering | Reader task |
|---|---|---|---|
| `change_map` | groups: `title`, `why`, members (paths, globs or stack layers), `attention` | Compact table linking to pages and file-tree groups | Explore a broad change / stack |
| `tour` | ordered steps: `title`, `anchors`, `check` (what to verify there) | Numbered list in the AI tab; drives `]`/`[` | Review order |
| `table` | `columns`, `rows` (inline Markdown cells), `role`: plain / mapping / comparison | Compact table | Real mappings only |
| `trace` | `columns` (e.g. Action, Before, After), numbered `rows` | Table with step numbers and a highlighted divergence row | Small fix: minimal failure sequence |
| `graph` | `variant`: flow / ownership / dependency / state; `groups` (boundaries); `nodes` (`id`, `label`, `detail`, `role`: actor / owner / transport / store / constraint / external, `anchors`); `edges` (`from`, `to`, required `label`, `style`: control / data / async / optional) | Layered layout computed by the app, drawn with gpui paths; node click shows `detail` and its anchors; role colors from the theme; legend generated from used styles | Ownership, data/control path |
| `graph_compare` | `before` and `after` graphs sharing node ids | Side by side, same scale and positions; added/removed/changed nodes and edges marked | Compare fixed designs |
| `sequence` | `lanes`, ordered `messages` (`from`, `to`, `label`, `anchor`, `note`) | Static swimlane | Concurrency, protocol |

### Judgement

| Widget | Fields | Rendering |
|---|---|---|
| `finding` | `severity`: blocker / concern / question / nit; `confidence`: high / medium / low; `origin`: introduced / pre-existing (a bug the PR did not add, after Claude Code Review); `category` (correctness, concurrency, security, API, performance, tests, readability); `title`; `body` (short, per the research above); `why`: how it was established, collapsed by default; `anchors`; `evidence` anchors; optional `suggested_change` | Card in the diff under its first anchor and a row in the AI tab. Actions: *Promote to GitHub draft*, *Promote to local note* (text editable first), *Dismiss* with a reason (wrong / not important / already known), *Ask* |
| `attention` | path or glob, `level`: focus / read / skim / mechanical, `reason` | File-tree badge, tree sort/filter, effort estimate |
| `verification` | `item`, `status`: ran-passed / ran-failed / author-reported / not-run, `detail`, anchors | Status list; never upgrades an author's claim to "verified" |
| `answer` | `question` (the user's), `body`, anchors | Card at the asked-about lines and in the Ask history |

### Why a closed vocabulary and not generative HTML

- It can be linked: every widget knows its anchors, so the diff, tree and studio cross-reference each other.
- It can be validated per widget, which is what makes streaming with self-correction work.
- It follows the themes, fonts and accessibility rules for free.
- It can be stored as typed columns and re-rendered after an app update.
- It is small enough for the prompt to carry the *when to use which* table, which matters more for quality than the number of widgets. New kinds are added when a real review needs one, like the skill's rule that interaction must reveal something.

## 6. Anchors: how claims stay attached to code

```text
anchor = { id, path, side: base | head, start_line, end_line, commit: base_sha | head_sha }
```

Validation when a widget arrives:

1. `path` is in the PR's file list (renames resolved through the file's previous name).
2. The line range lies in a hunk of that file, or within the file text at that commit (fetched once by blob SHA through the existing `fetch_file_text`, cached for good).
3. For anything promotable to GitHub (`finding`, `suggested_change`), the range must be patch lines, the rule the comment composer already enforces.
4. Ranges stay short (default ≤ 40 lines); `code_compare` sides must be the same logical location.

Rejected anchors return an error to the agent with the nearest valid range ("line 212 is outside the patch; hunks of this file cover 180–205 and 240–266"). Quarterdeck keeps the resolved excerpt hash so an anchor can be re-checked after a re-run.

## 7. How the agent talks to Quarterdeck

### Host an MCP server

Both CLIs are MCP clients. Quarterdeck runs as the server, with two tool groups.

**Read tools** (served from the cache; GitHub only when the cache lacks something):

| Tool | Returns |
|---|---|
| `pr_overview` | title, description, author, base/head SHAs, labels, stack position, review state |
| `list_files` | paths, status, added/removed, kind (code/tests/generated/moved), patch fingerprint |
| `file_diff(path)` | the patch with line numbers on both sides |
| `file_text(path, side)` | the whole file at base or head (by blob SHA) |
| `conversations` | existing GitHub threads and the user's local notes |
| `stack_layers` | layers, their numbers and ranges |

**Write tools** (each validated; the reply lists errors or the widget's id):

`studio_begin(preset, pages)`, `studio_add(widget)`, `studio_replace(id, widget)`, `studio_remove(id)`, `studio_finish(summary)`.

Each accepted widget is stored and drawn at once, so the user watches the Brief build: lead first, then the map, then sections. A progress note per tool call goes to the header pill.

**Transport without tokio.** Both CLIs launch stdio MCP servers as child processes. Quarterdeck's own binary gets a bridge mode: `Quarterdeck --mcp-bridge <socket>` speaks JSON-RPC on stdio and relays to the running app over a Unix domain socket created per run (in the user's cache folder, mode 0600, removed after the run). The MCP surface needed (`initialize`, `tools/list`, `tools/call`) is small enough to implement on blocking I/O in a UI-free crate, which keeps the repository's no-tokio rule; the official Rust SDK (`rmcp`) is tokio-based. The alternative is a loopback Streamable HTTP endpoint with a per-run bearer token, which both CLIs accept (Claude `{"type":"http","url":…,"headers":…}`, Codex `url` + `bearer_token_env_var`). It saves the bridge process but opens a port; the bridge is the smaller attack surface.

Limits to design for: Claude caps an MCP tool result at 25k tokens by default (`MAX_MCP_OUTPUT_TOKENS`), so `file_diff` and `file_text` page by line range. Headless Claude waits up to `MCP_TIMEOUT` (30 s) for servers; Codex has `startup_timeout_sec` (10 s) and `tool_timeout_sec` (60 s).

### Launching the agents

Claude Code (headless):

```bash
claude -p "$PROMPT" \
  --output-format stream-json --verbose \
  --mcp-config "$RUN_DIR/mcp.json" --strict-mcp-config \
  --tools "" --allowedTools "mcp__quarterdeck__*" \
  --append-system-prompt-file "$RUN_DIR/studio-guide.md" \
  --max-budget-usd 2
```

`--tools ""` removes the built-in tools (shell, edit, web) in diff mode, so the agent can only call Quarterdeck; `--strict-mcp-config` ignores the user's other MCP servers. (Flag names checked against Claude Code 2.1.318 and codex-cli 0.166.0 `--help` on this Mac; the exact `--tools ""` semantics still need a test run.)

Codex:

```bash
codex exec --json --sandbox read-only \
  -c 'mcp_servers.quarterdeck.command="/Applications/Quarterdeck.app/Contents/MacOS/Quarterdeck"' \
  -c 'mcp_servers.quarterdeck.args=["--mcp-bridge","<socket>"]' \
  --output-schema "$RUN_DIR/finish.schema.json" \
  "$PROMPT"
```

`--ignore-user-config` keeps the user's own Codex MCP servers out (a `-c mcp_servers={}` override does not clear them, [openai/codex#16045](https://github.com/openai/codex/issues/16045)); auth still comes from `CODEX_HOME`. Whether `codex exec` runs MCP tool calls without approval, or needs `default_tools_approval_mode = "auto"` for the server, is unverified and is the first thing the spike tests.

Both stream events as JSON lines (Claude: `system/init` lists the MCP servers and their status, then messages, then a `result` with `session_id` and `total_cost_usd`; Codex: `thread.started`, `item.*` including MCP tool calls, `turn.completed`), which feed the progress pill and the run log. Both accept a JSON Schema for the final message (`--json-schema`, `--output-schema`): that is the **fallback path** for a first spike, where the whole studio arrives as one validated object at the end. It is simpler, but loses streaming and per-widget correction, so it is a stepping stone, not the design. The schema has to satisfy both validators: Claude uses draft-07 and re-prompts on a mismatch; Codex uses OpenAI's strict mode (every property required, `additionalProperties: false`, optional fields as nullable). Whether Claude's `--json-schema` combines with `stream-json` is undocumented.

**Untrusted repository configuration.** Without `--bare`, headless Claude loads the working directory's `.claude/settings.json` hooks and `.mcp.json` with no trust prompt ([headless docs](https://code.claude.com/docs/en/headless)). In a checkout of somebody else's PR that runs their hooks on your machine. Diff mode avoids it by starting the CLI in an empty run folder. Repository mode must pass `--setting-sources user` and `--strict-mcp-config` (or `--bare`, which needs an API key and skips the subscription login colleagues use). Codex's handling of a project `.codex/config.toml` in that situation needs checking before repository mode ships.

The binaries come from `PATH`, `/opt/homebrew/bin`, `~/.local/bin` (the same search as `gh`, since a Finder launch has launchd's environment). Settings → AI picks the agent and model, and shows whether each CLI is installed and logged in.

### Repository context

- **Diff mode (default).** The agent sees only the read tools above. Works for every PR with no checkout; enough for most layers of a stack and most fixes.
- **Repository mode (opt-in per repository).** The user maps `owner/name` to a local clone. Quarterdeck creates a detached worktree at the head SHA in its cache folder and starts the CLI there with read-only tools (Claude: `Read`, `Grep`, `Glob`; Codex: `--sandbox read-only`), so the agent can follow callers and tests the way the skill's `sources.md` asks. Base-side reads still go through `file_text(path, base)`.

### Two passes

A run has a *brief* pass (explain; produces lead, map, sections, diagrams, terms, tour, attention) and a *check* pass (find problems; produces findings). The check pass gets the brief as context and must give each finding evidence anchors. With both CLIs available, the second agent can verify the first one's findings: agreement raises confidence, disagreement shows as a question rather than a concern. This is the review pattern the `/code-review` skill uses (find, then adversarially verify).

## 8. Interaction loop

| User action | What happens |
|---|---|
| *Brief me* (button, or `a`) | Starts a run for the shown target (layer or combined range). The preset is picked by size and tab: Mine → self-review, Review/Others → brief + check. |
| Select lines → *Ask* | Resumes the run's session (`claude -p --resume <id>`, `codex exec resume <id>`) with the question and the selection's anchor; the reply arrives as an `answer` widget at those lines. Chat that writes into the studio, not a chat log. |
| Promote a finding | Opens the existing composer with the text and anchor filled in, destination GitHub (draft for the next review) or Local. The promoted comment is the user's from then on. |
| Dismiss a finding | Stored with its reason. A re-run for the same PR receives the dismissed ones and must not repeat them. Later: per-repository *rules* distilled from dismissals and promotions, shown and editable in Settings (the Bugbot and CodeRabbit pattern), never learned silently. |
| (measured, not asked) | Quarterdeck records which findings were promoted, edited before promotion, or dismissed. That *acted on* rate per agent, preset and repository is the quality signal; thumbs get too few answers to be one (Graphite). |
| New commits arrive | The studio shows *stale: 3 files changed*. *Update* re-runs only on files whose patch fingerprint changed; widgets anchored to unchanged lines carry over, others are marked outdated. |
| Export | Local notes export already exists; add *Copy brief as Markdown* and later *Export as Review Studio* (the skill's format). |

## 9. Storage

The closed vocabulary turns into typed tables in `qd-cache` (one migration):

```text
ai_runs        (id, repo, pr_number, target_kind, base_sha, head_sha, agent, model, preset,
                session_id, status, started_at, finished_at, input_tokens, output_tokens, cost_micros)
ai_widgets     (run_id, widget_id, page_id, position, kind, title, text, why, tone, severity,
                confidence, origin, category, level, status, replaced_by)
ai_anchors     (run_id, widget_id, anchor_id, role, path, side, start_line, end_line, commit_sha,
                excerpt_hash)
ai_graph_nodes (run_id, widget_id, node_id, label, detail, role, group_id, graph_side)
ai_graph_edges (run_id, widget_id, from_node, to_node, label, style, graph_side)
ai_table_cells (run_id, widget_id, row, column, text)
ai_terms       (run_id, key, name, definition, anchor_id)
ai_feedback    (repo, pr_number, run_id, widget_id, action, reason, at)
```

Widgets with lists (tour steps, trace rows, sequence messages, change-map groups) use the table/row shape or their own small tables. Runs expire with the cache TTL unless they have promoted or dismissed findings.

## 10. Risks and how the design handles them

| Risk | Mitigation |
|---|---|
| Noise and false positives (the main complaint about review bots) | Explanation first; nits off; confidence and evidence required; per-run finding budget; cross-agent verification; dismissals feed back |
| Hallucinated code or line numbers | App quotes code; anchors validated against the pinned diff; unresolved anchors rejected |
| Prompt injection through PR text, comments or code | The agent only has read tools plus studio writes; no shell, no network beyond the model, no GitHub write path. PR content is labelled as untrusted in the prompt. The reviewed repository's own agent configuration (hooks, MCP servers) is never loaded. A malicious PR can at worst produce a misleading studio, which the reviewer reads against the diff |
| Automation bias (approving because the AI found nothing) | No verdict suggestion; *no findings* is shown with what was and was not examined (diff mode vs. repository mode, files skipped as mechanical) |
| Code leaving the machine | Off until enabled per repository; Settings shows which agent and account receive the code |
| Cost and latency | Manual trigger by default; cached per head SHA; incremental updates; usage shown per run; the brief pass can use a cheaper model than the check pass |
| Stale studios | Pinned to base/head; stale banner; incremental update |
| Vocabulary too narrow for some change | `section` + `table` cover anything; add a kind when two real reviews need it |

## 11. Plan

**Spike (a few days): is the vocabulary good enough?**
1. New UI-free crate `qd-studio`: the widget types, anchors, validation against `qd-diff` data, and a JSON Schema for the vocabulary. JSON only on the wire, never stored.
2. Run `claude -p --json-schema` with the studio guide on five real PRs from the cache (a one-line fix, an API change, a concurrency change, a 100-file feature, a stack). Render the result in a plain Brief pane with `lead`, `section`, `callout`, `code_compare`, `table`, `trace`, `finding`.
3. Compare against Review Studio HTML reports for the same PRs. Decide what the vocabulary lacks.
4. Settle the unverified CLI behaviour: MCP tool approval in `codex exec`, `--tools ""` in Claude, `--json-schema` with `stream-json`, and one schema accepted by both validators.

**Phase 1: the loop.** MCP bridge with read and write tools, streaming Brief, AI cards in the diff, AI tab (findings, tour), promote/dismiss, storage, Settings → AI.

**Phase 2: depth.** `graph` / `graph_compare` / `sequence` with native layout (candidate crates for the layered layout: `rust-sugiyama`, `dagre`, `layout-rs`; to evaluate), term popovers, Ask, Codex as second agent and verifier, repository mode with worktrees.

**Phase 3: scale.** Stack pages and combined atlas, incremental updates on new commits, per-repository learnings, Export as Review Studio.

## 12. Open questions

1. Should Codex and Claude be interchangeable for both passes, or fixed roles (one explains, the other verifies)?
2. Is repository mode needed early? Seed reviews likely need callers and tests; library-eco fixes may not.
3. Should a brief start on its own for PRs that request your review in chosen repositories, or always on a click?
4. Is *Export as Review Studio* (sharing through `share.seed.ci`) wanted for colleagues, or does the studio stay personal?

## 13. Spike results (8 November 2026)

Built: the `qd-studio` crate (widget types, JSON Schema, authoring guide, anchor checks, numbered-diff prompt, confined `claude -p` run with streamed progress) and, in the app, a **Diff | Brief** switch in the detail header with a Brief pane: lead, sections, callouts, code excerpts and comparisons quoted from the loaded diff, tables, traces and findings with folded reasoning. Clicking a quoted range opens the diff at that line. Briefs live in memory for the session; nothing is stored yet.

CLI behaviour, settled on this Mac (Claude Code 2.1.318, codex-cli 0.166.0):

- `claude -p --tools ""` leaves only the `StructuredOutput` tool; `--json-schema` works together with `--output-format stream-json`, and the `result` line carries `structured_output`, `total_cost_usd` and `session_id`. With `--include-partial-messages` the answer streams as `input_json_delta` chunks, which drive the progress text.
- `codex exec --output-schema` accepts the same nullable-required style and returns the JSON as the final `agent_message`. Codex keeps its own tools (shell, patch) under `--sandbox read-only`, so it cannot be confined as tightly as Claude; it waits on stdin unless stdin is closed. MCP tool approval in `codex exec` is still untested.

Two headless runs of the exact app pipeline on commits of this repository (default model, diffs only):

| Change | Prompt | Time | Cost | Widgets | Rejected |
|---|---|---|---|---|---|
| `d737f3a` tree button: 8 files, +266 −35 | 60k chars | 52 s | $0.34 | 12 (5 excerpts, 3 sections, 2 callouts, 1 table, 1 finding) | 0 |
| `9295836` Others sync hardening: 29 files, +855 −294 | 238k chars | 156 s | $1.09 | 22 (7 excerpts, 4 sections, 4 findings, 2 callouts, table, trace, comparison) | 2 |

What the briefs got right:

- The structure follows the guide: changed behavior first, a parts table, mechanism sections whose headings name the mechanism ("Each sync starts where the previous one ended"), constraints with cause and effect, an evidence callout that separates the PR's tests from what is untested and says it ran nothing.
- The trace widget was used as intended: sync start points across one session, before and after the change, including the >20-page restart case.
- Findings came with reasoning that cites lines. Two were checked by hand and are real and still present on `main`: a used-up core budget (`RateLimited`) sets no refresh hold because `GitHubError::retry_after` only answers for `SlowDown` (`crates/github/src/github_error.rs:43`), and a test doc comment that ended up above the wrong test (`crates/cache/src/github_cache.rs:328`). Two others were phrased as questions where the diff alone could not settle them, which is the intended behaviour.

What needs work:

- **Anchors across hunk gaps.** Both rejections were ranges that stepped over a few hidden lines between hunks. Fixed: an anchor now needs only its first and last line in the diff, and the excerpt shows a "N lines not in the diff" row for the gap.
- **Latency.** Most of the time is thinking before the first output byte (30 s and 115 s). The brief only appears at the end, so the streaming design (widgets added one at a time through MCP) matters more than expected.
- **Leads run long** (four to six sentences instead of one to three). A tighter schema description or a length check is enough.
- **Diff-only context shows its limits** in the questions the agent had to ask ("I can't see `RepoRef`'s `PartialEq`"). Repository mode or a `file_text` read tool would answer them.
- Not yet verified by hand in the running app: the Brief pane was built and the app launched, but nobody has clicked through it yet.

### Diagrams (step 2, first cut)

`graph` and `sequence` are in the vocabulary; `graph_compare` became a mark on one graph (`change`: same / added / removed per box and arrow), drawn as aligned Before and After pictures from one layout, which is what the skill asks of fixed-design comparisons. The agent writes only ids, labels, roles, boundaries and arrow styles; the app checks them (known and unique ids, labelled arrows, sizes, an arrow only in the pictures both its boxes are in, anchors in the diff) and lays them out itself, with no layout crate: layers along the arrows (left to right when that fits the column, else top to bottom; laid out again for the column whenever the reader resizes the pane, since the brief takes the pane's width), one band per boundary so borders never overlap, boxes ordered by their sources, cycle-closing arrows routed around the boxes, labels in the gap after an arrow's start and pushed apart. Text is measured by estimate before drawing. The Brief Gallery (Window → Brief Gallery) shows a flow graph in boundaries, a before/after graph, a state graph with a cycle and a self loop, and a sequence on sample data. Not yet tried: what Claude draws when given the new widgets on a real pull request.

### Pages

A studio is now a walk of one to six pages instead of one long column: each page has a tab title and a one-line summary of what it answers; the guide suggests Overview → a page per mechanism → Risks and limits → What to check, with findings last, and one page for a small change. The Brief pane pins numbered tabs above the brief and ends each page with previous / next buttons naming the neighbouring page (the next one with its summary), so the brief reads as a sequence rather than a list of changes. Pages are ranges of one widget list, so anchors, folded reasoning and ids work across them. The gallery's sample is split into four pages.

### Shallow checkout (repository context)

Write brief first checked out the merge base and the head shallowly (`git fetch --depth 1` of the two SHAs, LFS files as pointers) into a workspace under `~/Library/Caches/de.plucked.quarterdeck/review-workspaces/` as `base/`, `head/` and `change.diff`, and runs Claude there with only `Read`, `Grep` and `Glob` under `--restricted --safe-mode`: tested, it cannot read outside the folder, has no shell, and ignores the repository's hooks and CLAUDE.md. The workspace is deleted when the run ends; folders of dead processes are swept at launch. Any failure before the agent starts falls back to the diffs alone, and the footer says why.

Later cut to the head alone: the agent explores around the change in `head/`, and the diff already shows what each changed line was before (a deleted file's patch holds it whole). `base/` was read rarely, doubled the download and needed the merge base, which GitHub hands out only inside a compare response carrying every file's patch. `change.diff` is now the app's loaded patches, so it matches the diff the anchors are checked against.

Trial on `d737f3a` (8 files, +266 −35): checkout 1.0 s, 6 reads, 65 s, $0.55 (diff only: 52 s, $0.34); three pages (Overview with a 5-node graph, Header layout, What to check), nothing rejected, no findings: it checked the folder-prefix match and the order of steps in the code and said they hold, instead of asking.

### A presentation, not a review

The brief is now explicitly a presentation of the change, as one engineer walks another through it, not a code review: the `finding` widget and its grades are gone, along with everything that judges the change. From the research on presenting changes to reviewers (November 2026), what fits that purpose was adopted:

- **Intent before implementation**: the lead and the Overview page say why the change exists before how it works, then build up only the context the change needs (Meta's ARCTIC defines intent as *why*, not *what*).
- **Reading order instead of the file list**: a `tour` widget, 2–9 steps ordered by dependency (foundations, then the core early, call sites, tests next to what they test, mechanical last), each marked core / supporting / mechanical in words and linked to where it starts. Position in the review order measurably changes what is understood (Fregnan et al., ESEC/FSE 2022; "Breaking the Alphabet", ICSE 2026); separating mechanical from semantic changes cuts noise (Di Biase et al.).
- **Scope contract**: a `scope` widget listing what changes, what deliberately stays the same, and what is mechanical.
- **Before → after behaviour** as a concrete example (trace or table), the worked-example effect.
- **Writing rules**: no restating the diff and no file-by-file change tables; every claim from the diff, description or code read; no phantom changes (Gong et al., MSR 2026); declarative, calibrated tone without sales words; say what could not be established; never call a part "safe".
- **Adaptation by change type** (fix, feature, refactor, migration) and length budgets; diagrams only for the changed flow, at most about twelve boxes.

Left out on purpose, because they serve reviewing rather than understanding: severities, confidence badges, findings counts, "where I need you" review asks, risk tiers and verification of the author's claims.

Trial of the presentation guide on `9295836` (29 files, 238k-character prompt, with the checkout): 72 s, $0.98, no reads (the listing held what it needed). Five pages: Overview (why, scope, a 9-step reading order), Others sync, Throttling (with a sequence of one refresh), Fewer redraws, Limits and evidence; no verdicts. What it showed: the reading order was dropped because one step cited 86 lines, so step anchors are now cut to their start line; the scope listed 16 changes and the lead ran to six sentences, so the schema and guide now budget about six scope items per list and a lead of three sentences / 60 words.

### Reader familiarity

Write brief now asks how well the reader knows the area: *I know this code well*, *I know the area roughly* (the default), *I'm new to this area*. The answer is a "The reader" section of the user message: no background for an expert; the touched mechanisms and terms for a rough overview; for a newcomer, a Background page after the Overview (what this part of the system is for, its parts and state owners, the vocabulary, where the change sits) drawn from the surrounding code in `head/`, not only the diff. The facts stay the same at every level; only the explanation around them changes. This follows the expertise reversal effect from the presentation research: scaffolding that helps a newcomer burdens a code owner. The answer is remembered per repository for the session and can be changed in the footer to write the brief again for another reader.

### Stack briefs

A brief belongs to a pull request and its review target, so a layer and a combined range each have their own. A combined brief now also gets the layers it spans (number and title, bottom first, from the loaded stack: no request), with the instruction to follow them where the change builds up layer by layer (a page or a run of reading-order steps per layer, named by number) and to explain the whole where the layers only make sense together. Only the top layer's description is sent; the others' would take a request per layer.

### Reporting a brief

Report, at the right end of the version line above every brief, saves the reader's words with the brief's record to `~/Library/Application Support/Quarterdeck/brief-reports/<UTC stamp>-<owner>-<name>-<number>/`: `report.md` (the words, the subject, layers, commits, reader, code access, model, cost, duration, the answer's shape, every tool call, and where in this repository the brief is made), the exact user message, the guide and schema as compiled into that build, the raw answer, what the checks left out, and the transcript without the partial-message stream. The record is kept in memory with each brief for the session. The point is a feedback loop: a Claude Code session in this repository reads `report.md` and proposes changes to the guide, schema, checks or rendering.

### Stored briefs and their version

Briefs are now stored in the database, one per review scope (a pull request, or a combined range of stack layers), in typed tables like everything else (no JSON): the brief's row with the run's record, then pages, widgets and every part of every widget kind. They are user data, paid for, so they never expire and survive clearing the cache. Each records the head commit it was written for; opening a pull request reads its brief with the files and compares that commit with the shown head: a quiet line when it matches, a marked line with Write again when the pull request or range has moved on. The sidebar marks pull requests and stack layers that have a brief with a ✦ (matched by repository and number, since the search and pulls APIs give different ids). The raw answer and transcript are not stored, so a report of a brief from an earlier session has the prompt, the checked brief, what was left out and what it read, but no `answer.json` or `transcript.jsonl`.

## Reports: a large stack and a missing sequence (2026-11-09)

Two reports. A 27-layer stack (seed #35945, 1608 files) got a compact brief about its top layer: the prompt's 300k-char diff budget was spent in tree order, so `.github/workflows` and `.claude/skills` were listed and the code (Bazel rules, the Rust stager, SeedCmd) was not, and could not be cited; the only description was the top layer's; and six pages cannot hold 27 layers. Changes: the budget now goes to code, then documentation, then tooling under dot-folders; every changed file's numbered diff is written to the workspace as `diffs/<path>.diff` and may be cited; a combined prompt carries every layer's description (cache first, else one request per layer); a big stack may use up to ten pages, a page per step of layers; for a large change the agent is told to start Explore subagents (the `Agent` tool, which keeps the read-only confinement) on separate parts and write the brief from their reports. The other report (library-eco #716, a fence that refuses late writes from a former owner) asked for a sequence diagram: the guide now says to draw a sequence whenever the behaviour depends on the order of messages (races, late or stale messages, handovers, deadlines), next to a background graph.
