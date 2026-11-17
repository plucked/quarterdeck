# Repairable review generation: application contracts

Research date: 15 November 2026. This note describes the current working tree at `9b8613f`, including the existing uncommitted guided-review changes. It is source research and a proposed design, not an implementation. Line references describe that working tree. Protocol, Claude CLI compatibility, and SDK selection belong to the accompanying transport investigation; none of those choices should determine the domain invariants below.

## What failed today

**Observed:** `parse_items` expands each reading item's selectors, then adds its exact files to a bucket-local set. If a second item contains an already-added file, it returns **“A file appears in more than one item of a review bucket.”** This is an intentional semantic check, not malformed JSON. An exact file following an overlapping folder selector in another item can trigger it. For example, an item for `src/` and a later item for `src/main.rs` overlap when that file is in the changed-file inventory. The returned message does not identify the bucket, either item, the file, or the overlapping selectors. [Validation](../../crates/studio/src/smart_filter.rs#L104)

The membership rules are deliberately asymmetric:

- Repeated selectors *within one item* deduplicate while retaining stable order.
- A file may occur in multiple buckets.
- A file must not occur in two items of the *same* bucket.
- Selectors are exact changed paths or folder prefixes ending in `/`; they expand against the captured inventory once. Unknown selectors, empty items, missing reading guidance, duplicate/nonpositive bucket order, and incomplete overall coverage reject a new plan.

These are current rules, not proposed stricter uniqueness. Coverage is currently checked by comparing the covered set's length with the input inventory's length, assuming the inventory itself has unique paths. [Parser](../../crates/studio/src/smart_filter.rs#L61), [selector expansion](../../crates/studio/src/smart_filter.rs#L144), [overlapping-bucket regression](../../crates/studio/src/smart_filter.rs#L209)

**Observed:** the guide already tells Claude to cover all changed files and use each file exactly once within a bucket. The schema checks objects, arrays, required fields and basic numeric/string constraints; it does not know the captured changed-file inventory or the result of expanding folders. Even adding `uniqueItems` to `paths` cannot detect two different selectors that expand to the same file across two items. [Guide and schema](../../crates/studio/src/smart_filter_guide.rs#L9)

**Observed:** Claude's schema loop cannot catch this Rust error today. `run_smart_filter` passes the schema to the CLI; the runner returns `structured_output` from the final result. Only afterwards does `SmartFilterJob` call `parse_smart_buckets`. There is no semantic-validation callback and no application repair attempt. The error is shown to the user after the run has ended. [CLI configuration](../../crates/studio/src/run_claude.rs#L154), [final-answer handling](../../crates/studio/src/run_claude.rs#L281), [job boundary](../../crates/app/src/pull_request_detail/smart_filter_job.rs#L53)

**Inference:** the failure demonstrates an incomplete feedback loop, rather than evidence that the semantic rule is wrong. It does not reveal which selectors the reported run used; the guided-review job currently discards the answer's transcript and metadata after parsing instead of keeping a report record. [Job](../../crates/app/src/pull_request_detail/smart_filter_job.rs#L53)

## What the brief pipeline shares, and where it differs

**Observed:** both kinds share `StudioRequest`, complete file inventory, budgeted diff listings, a confined CLI runner, workspace preparation and progress events. The inventory includes files whose diffs are omitted. With a workspace, eligible numbered diffs beyond the prompt budget remain in `diffs/` and enter the anchor targets; without a workspace, those omitted diffs cannot be cited. Generated, unchanged-moved and patchless files are explicitly left out of eligible listings by the app. A “changed file exists” test therefore cannot substitute for an anchor-target test. [Request construction](../../crates/app/src/pull_request_detail/studio_request.rs#L54), [listing eligibility](../../crates/app/src/pull_request_detail/studio_request.rs#L76), [shared prompts](../../crates/studio/src/studio_prompt.rs#L77), [target construction](../../crates/studio/src/studio_prompt.rs#L204)

**Observed:** `parse_studio` deserializes the whole answer, then keeps each valid widget and records invalid widgets as `RejectedWidget`. Excess pages are rejected; a malformed top-level shape returns an error. Semantic invalidity is generally a *partial-success* path: the brief is still stored and shown with a list of omitted parts. The parser can retain a page whose widgets were all rejected. This is not an in-run repair loop. [Parser](../../crates/studio/src/parse_studio.rs#L117), [checked result](../../crates/studio/src/checked_studio.rs#L4), [rejection UI](../../crates/app/src/brief_view/brief_footer_rendering.rs#L74), [brief job](../../crates/app/src/pull_request_detail/brief_job.rs#L74)

Anchor checks require a known target path (renames also answer to their old path), a valid inclusive range of at most 80 lines, and both endpoints visible on the requested base/head side. Gaps between hunks are allowed and shown honestly. Tours check only the anchor's start line. Comparisons require base before/head after. Graph and sequence validation checks local reference IDs, sizes, anchors and relevant structural relationships; the graph additionally checks whether an edge exists in a before/after picture where both endpoint nodes exist. Table row widths must match columns. [Anchor targets](../../crates/studio/src/anchor_targets.rs#L29), [widget checks](../../crates/studio/src/parse_studio.rs#L184), [tour checks](../../crates/studio/src/parse_studio.rs#L248), [table checks](../../crates/studio/src/parse_studio.rs#L284), [graph checks](../../crates/studio/src/wire_graph.rs#L96), [sequence checks](../../crates/studio/src/wire_sequence.rs#L60)

**Observed:** brief reports preserve the rejected reasons, prompt, model, cost, tool calls and other run metadata. The live-session record also has raw answer/transcript; typed SQLite storage intentionally does not retain those two values. Reports write the guide/schema compiled into the reporting build, so a report for a restored old brief is not proof of the exact older guide/schema. Guided review has no equivalent record in its job. [BriefRecord](../../crates/studio/src/brief_record.rs#L9), [restoration](../../crates/cache/src/brief_storage.rs#L91), [report files](../../crates/studio/src/write_brief_report.rs#L69)

## Existing lifecycle guarantees and gaps

| Boundary | Observed behavior |
|---|---|
| Guided-review replacement | The previous plan remains separately stored during generation and on error/cancel. One run per scope is allowed; navigating to another scope does not make a result belong to it. [State](../../crates/app/src/pull_request_detail/smart_filter_state.rs#L6), [actions](../../crates/app/src/pull_request_detail/smart_filter_actions.rs#L49) |
| Brief replacement | Cancellation restores the previous brief. Failure sets `Failed` and discards the in-memory `replaced` brief, although an unsuccessful generation has not replaced its database row. This differs from guided review. [State transitions](../../crates/app/src/pull_request_detail/brief_pane.rs#L168), [cancellation](../../crates/app/src/pull_request_detail/brief_pane.rs#L226), [completion](../../crates/app/src/pull_request_detail/brief_actions.rs#L221) |
| Persistent replacement | Both stores delete/insert the full artifact inside one SQLite transaction. SQL failure preserves the previous artifact. Neither write has an expected-generation or expected-snapshot condition. [Guided review](../../crates/cache/src/smart_filter_storage.rs#L12), [brief](../../crates/cache/src/brief_storage.rs#L18) |
| Save failure | Guided review reports a session-only warning. Brief save failure is printed to stderr and the new brief still shows for this session. [Guided job](../../crates/app/src/pull_request_detail/smart_filter_job.rs#L71), [brief store wrapper](../../crates/app/src/ui_state/ui_state_store_reads.rs#L44) |
| Cancellation | Dropping `RunCancel` sets a flag, and blocking process loops stop the child. Guided review checks again before locking the store; brief has no post-answer cancellation check before persistence. A flag check followed by an unrelated store lock is not a single atomic cancellation/publication decision. [Guard](../../crates/app/src/pull_request_detail/run_cancel.rs#L4), [guided job](../../crates/app/src/pull_request_detail/smart_filter_job.rs#L68), [brief job](../../crates/app/src/pull_request_detail/brief_job.rs#L74) |
| Staleness | Guided review records and compares both head and optional base. Brief record/version UI compares only head. No write-time check compares the captured revision with the latest known revision for that scope. [Guide version](../../crates/studio/src/smart_filter.rs#L55), [brief version](../../crates/app/src/brief_view/version_rendering.rs#L74) |
| UI updates | Blocking jobs run on `background_executor`, emit through `futures` channels, and mutate the view with `detail.update`. Existing updates are keyed by scope or brief key, not an explicit application run generation. [Guide actions](../../crates/app/src/pull_request_detail/smart_filter_actions.rs#L89), [brief actions](../../crates/app/src/pull_request_detail/brief_actions.rs#L199) |

The cancellation/store race is a source-derived risk, not a reproduced production incident. Existing task dropping prevents many stale UI callbacks; that is not a durable compare-and-swap guard at the publication boundary.

## Proposed domain contract

Everything in this section is a **design decision**, not current behavior.

### One pinned run owns one artifact draft

Use an application `RunId`, independent of any protocol session, plus an immutable `ReviewSnapshot`. The snapshot binds artifact kind, review scope, repository, exact base/head or combined-range endpoints, changed-file inventory, eligible anchor targets, and a domain-contract version. An opaque `SnapshotId` can stand for these values on the wire. Construct it once at the click from the data already loaded; tools do not silently switch to whatever PR the user currently has open.

The snapshot must distinguish full membership inventory from readable/citable files. Deduplicate/check the input inventory once and compare actual set differences for coverage; do not rely on equal cardinalities. File identity is the exact canonical path from that snapshot. Do not resolve filesystem paths, case-fold, or normalize `..` into a different file. Rename aliases are accepted by anchor resolution according to the existing target rules; they do not invent duplicate guided-review members.

Keep an explicit lifecycle: `Drafting → Ready (frozen) → Committing → Committed`, with terminal `Cancelled`, `Superseded`, `Failed`, or `StaleSnapshot`. Validation never publishes. Cancellation and publication share the same controller/locking boundary, so their order is defined: cancellation first forbids publication; committed first means a complete artifact already exists and cancellation only stops remaining agent work.

### Repair complete buckets; validate the complete guide

A useful typed surface is:

```text
review_context() -> SnapshotSummary + file inventory pages
replace_bucket(bucket_id, expected_revision, operation_id, complete_bucket)
    -> Accepted { revision, expanded_members, coverage_summary }
     | Rejected { revision, issues }
remove_bucket(bucket_id, expected_revision, operation_id)
validate_guide(expected_revision, ordered_bucket_ids)
    -> Valid { revision, coverage_summary }
     | Invalid { revision, issues }
finish_guide(expected_revision, ordered_bucket_ids, operation_id)
    -> Ready { revision, validation_receipt, coverage_summary }
     | Invalid { revision, issues }
```

`replace_bucket` expands all selectors and validates the complete proposed bucket atomically. A conflict with an existing item inside that bucket rejects the replacement and leaves the earlier accepted bucket unchanged. It does not silently move files, discard reading guidance or guess the author's intended grouping. Coverage is a global completion requirement, not a reason to reject the first valid bucket in an incomplete draft. `validate_guide` requires at least one bucket, unique ordered bucket IDs and complete file coverage; overlapping membership across buckets remains allowed. It returns diagnostics without freezing the draft. `finish_guide` repeats these authoritative checks, freezes the current revision and produces the typed `ValidatedGuide` with a readiness receipt. Its result explicitly says publication is pending.

Use a stable opaque bucket ID for creation/upsert; permit a short agent-chosen ID scoped to the run, with duplicate-ID checks. Display order is a separate ordered list. Existing `SmartBucket.order` is a sort key, and `WalkthroughItemId` is two vector positions, so neither is a reliable identity under reorder. Do not use a label as identity. Items can remain positional within a complete bucket replacement; no item patch API is needed. [Current bucket data](../../crates/studio/src/smart_filter.rs#L16), [UI positions](../../crates/app/src/pull_request_detail/walkthrough_item_id.rs#L1)

A whole-guide `submit_candidate` is an excellent first proof that tool errors reach the agent and repair the exact failure. For a 3,803-file stack, resending every bucket to repair one overlap can waste output tokens and introduce changes to previously correct buckets. Production bucket replacement bounds the repair to a meaningful unit while keeping all membership checks local and deterministic. This size is a design scenario, not a measured run in this investigation. Do not jump to arbitrary JSON patches or individual file-membership mutation; they create more ordering and intermediate-invalid-state problems than they solve.

### Return actionable, bounded diagnostics

Return typed issue codes, object IDs, field pointers and concrete evidence. For example:

```json
{
  "code": "duplicate_file_in_bucket",
  "bucket_id": "runtime",
  "path": "src/main.rs",
  "locations": ["/items/0/paths/0", "/items/2/paths/0"],
  "selectors": ["src/", "src/main.rs"],
  "message": "src/main.rs belongs to both items after selector expansion. Revise this bucket so it belongs to one item."
}
```

Other codes include `unknown_path`, `missing_paths`, `empty_guidance`, `invalid_order`, `anchor_outside_diff`, `unknown_diagram_id`, `revision_conflict`, `stale_snapshot`, and `cancelled`. For anchors, report valid ranges on the requested side, reusing the current helpful anchor error information. For missing files, return total count and a bounded page of exact paths plus a cursor to read the remainder. Aggregate independent issues so one repair can fix several errors; cap result bytes and preserve deterministic ordering.

A validator result describes structural correctness, coverage and anchor resolvability. It must not claim that an explanation is true, a mechanical label is justified, or a review is complete. `suggest_viewed` remains a suggestion requiring the user's existing action.

### Revisions, retries and concurrent requests

Each run has one serialized draft owner. Mutations include `expected_revision`; a successful mutation increments it once. Concurrent calls based on the same revision cannot both succeed: the loser receives `revision_conflict` and reads current state before retrying. Different runs do not share mutable draft maps. Start with the parent agent as the single author even when exploration is delegated; exposing shared draft writes to several subagents adds conflict handling without improving this repair case.

`operation_id` is stable for a retried request. Record its canonical typed payload digest and response for the run lifetime: same ID and same payload returns the original response without another revision increment; same ID with changed payload is an explicit conflict. Check replay identity before checking `expected_revision`, so a lost successful response can be recovered. Failed validation leaves accepted state/revision unchanged, but its operation response is still replayable. Changed repair content uses a new operation ID.

`finish_guide` issues a receipt bound to run, snapshot, contract version, accepted draft revision and canonical content digest. The frozen draft rejects further mutations; retrying the same finish operation returns the same readiness receipt. Any accepted mutation before finish invalidates earlier validation results. The host's `commit` only accepts that exact frozen revision. Validation plus an arbitrary final text answer is not sufficient authority to publish different content.

### Commit once, preserve the previous result

Keep publication host-owned for the first version. The agent's final structured answer echoes the readiness receipt identifying the frozen candidate. A successful terminal CLI result and a matching readiness receipt are both mandatory: a process success without the receipt, or a receipt followed by an unsuccessful run, is incomplete. The background job combines the frozen artifact with final cost, session, model and transcript metadata, then calls the typed publisher; it does not parse a second unrelated full document. This avoids claiming durability to the agent before persistence. Only the completed database transaction yields a durable publication receipt. A later explicit `commit` tool may call the same publisher, but its success must mean the transaction committed; that alternative needs post-commit metadata amendment, and subsequent CLI failure must not relabel a committed artifact as failed.

The publisher checks, under a serialized publication gate, that the run still owns the current generation for `(scope, artifact kind)`, has not been cancelled, targets the latest known snapshot, and presents the current receipt. A run from an older snapshot must not replace a newer result. Navigation alone does not invalidate the original scope. A newly observed base/head revision does; return `StaleSnapshot` and keep the earlier published result. This is a check against application knowledge, not a guarantee that an unobserved remote push has not occurred.

Retain typed SQLite atomic replacement. Store a publication generation or artifact ID with the scope and use an expected-generation condition in the same transaction, so delayed writers cannot overwrite a newer commit. Define one publication gate as owner of both cancellation/supersession and this transaction; a standalone `AtomicBool` check is insufficient. A repeated commit of the same run/receipt returns the original committed artifact ID; a different draft cannot reuse it. Once committed, the run cannot publish again.

Choose the first production contract to publish only after durable save succeeds. On save failure, retain the previous visible and persistent artifact and keep the validated draft available for a host retry. If preserving today's session-only-new-result behavior is desired, represent it explicitly as `ValidUnsaved`; do not report it as `Committed`. In either case, cancellation, invalid data, CLI failure and stale results retain the previous good guide *and brief*, with a separate failed-attempt message.

### Briefs reuse the lifecycle, not guided-review coverage rules

Reuse snapshots, revisions, operation IDs, diagnostics, receipts and publication. Add `replace_page(page_id, expected_revision, operation_id, complete_page)` later; a page is a practical repair unit. Validate all widgets and anchors in that proposed page before replacing the accepted page. Preserve stable page IDs, order pages separately, and identify widget errors by page ID plus field pointer/index. Graph node/lane IDs remain local to their widget. Whole-page replacement avoids a general-purpose widget mutation engine.

Existing widget checks should be factored into reusable typed functions rather than duplicated in a transport handler. For the new repair path, existing widget rejections become tool errors, so the agent can fix or deliberately remove the widget before final validation. Legacy stored briefs and the old tolerant parser can remain readable. This changes new-generation failure handling and must be reviewed as a domain decision, not an automatic consequence of using MCP. If a new rule such as “every published page must contain a useful widget” is added, encode it in both validator and schema and test it explicitly; current schema/parser do not guarantee it.

Show accepted draft progress or a clearly marked preview separately from the published artifact. Never expose partial guided-review grouping to viewed/bucket actions. Do not overwrite stored pages after every tool call. Cancellation removes the preview and restores the complete previous artifact. Native layout still happens on the main thread; transport callbacks do not touch GPUI entities.

## Minimal module seams

**Proposed:** keep domain checks, snapshots, diagnostic types, draft state and validated result types in `qd-studio`. Split the existing parsing functions into wire decoding and typed validation so both the one-shot adapter and an MCP adapter call the same implementation. Avoid a new general workflow crate, event sourcing, arbitrary widget registry or persistent draft history until there is a demonstrated use.

Keep CLI process management and the run-owned server/helper guard together in the runner orchestration. Any endpoint, configuration, reader/writer tasks and child lifetime belong to that single run and close on all outcomes. The endpoint is a capability to this run's snapshot, never a generic app control surface. Transport selection and its runtime policy are separate decisions; do not hand-write an HTTP parser merely to preserve the current no-Tokio rule.

Launch integration is gated on a configuration-isolation probe. The current runner hardcodes `--safe-mode`, `--restricted` and `--strict-mcp-config`; the transport investigation must establish an invocation that loads the explicitly supplied tools while retaining the intended repository-configuration isolation and the user's authentication. Simply deleting `--safe-mode`, or assuming a different minimal-startup flag is equivalent, is not an implementation plan. Run the compatibility/confinement experiment before changing the production launch. [Current invocation](../../crates/studio/src/run_claude.rs#L154)

Keep `qd-cache` responsible for typed transactional persistence and expected-generation publication. The app assembles the snapshot and current run ownership, launches blocking work, consumes channel events, and displays previous result + draft progress + last-attempt outcome. Tag every UI event with `RunId` and reject events from a superseded run even when scope keys match. This extends the existing job/channel seams instead of moving GPUI into a server callback. The repository's explicit rule remains “Only the app depends on gpui. Library crates are blocking and UI-free … No tokio.” Any chosen SDK that requires a runtime needs a narrow, explicit architecture decision outside the domain library. [Repository architecture](../../CLAUDE.md#L26), [existing guided job bridge](../../crates/app/src/pull_request_detail/smart_filter_actions.rs#L89)

## Rollout and acceptance evidence

1. **Make the contract independently testable.** Introduce typed diagnostics and shared checks; reproduce the overlap with two selectors that are individually valid. Keep the one-shot path as an adapter. Improve guided-review failure records and preserve prior briefs on failure. No transport is needed to validate these changes.
2. **Prove repair through the chosen per-run transport.** Use one complete-candidate tool in an isolated experiment: intentionally return the duplicate-item error, have the same agent correct it, then return a validated receipt. Verify tool availability under actual confinement and cancellation before building a large read API. A schema-only retry with an additional process/prompt can also prove the domain contract, but does not establish same-run tool feedback.
3. **Ship guided-review bucket replacement and atomic publication.** Add run identity, revision/idempotency checks, coverage diagnostics and store publication guard together. Benchmark repair cost on a large explicit inventory. Detect repeated attempts that make no progress, retain their diagnostics and keep cancellation available. Any automatic retry or stop policy must be visible and intentional; this proposal introduces neither an arbitrary attempt limit nor an app-imposed dollar cap. A failed attempt preserves the prior result.
4. **Bring briefs onto the same lifecycle.** Add page replacement, anchor/diagram repair and reports of validation attempts. Stream optional previews only after durable publication semantics are established. Keep protocol conformance and SDK testing in the transport workstream.

Meaningful acceptance tests:

- Exact duplicate and folder/exact overlap identify the canonical path, bucket and both items; a corrected bucket succeeds in the same run. Duplicated selectors within one item deduplicate, and overlap across buckets remains accepted.
- A first valid bucket is accepted while total coverage is incomplete; finalization returns the exact missing paths, including binary/generated/moved inventory entries. Unknown paths never enter accepted state, and newly changed files never appear through an old saved folder selector.
- A large inventory with thousands of files repairs one bucket without resending or changing the others. Measure tool request/response size and repair attempts, not only wall time of model inference.
- Duplicate operation replay returns exactly one mutation; changed payload under the same operation ID fails. Two concurrent expected-revision mutations have one winner. An edit makes earlier validation results stale; a frozen finished draft rejects further edits, and an identical finish retry returns the same readiness receipt.
- Run A cannot write or publish into run B. A late result from an older run, a changed base with unchanged head, or a newer committed generation cannot replace the current artifact. Navigation to another PR does not misattribute an otherwise valid original-scope result.
- Deterministic barriers race cancel/supersede against publication: whichever crosses the publication gate first wins; no post-cancel partial replacement is possible. Cover a silent CLI, view destruction, helper failure and workspace cleanup separately.
- Inject SQL failure after deleting the previous row and during child inserts; reopening the database must return the previous complete artifact. Retry a successful publication receipt and observe one committed artifact. Explicitly verify the chosen save-failure UI state.
- Brief anchor repairs preserve rename/base/head/80-line/gap semantics; unknown graph IDs, impossible before/after edges and table-width mismatches reach the agent. Invalid replacement pages do not erase accepted pages. Cancellation/failure restores the previous published brief.
- An incomplete streamed guide never drives marking actions. A partial brief preview never becomes persistent by accident. Stale progress messages cannot reset current state.
- Reports identify contract/guide/schema version, captured base/head, run, accepted and rejected revisions, diagnostics, operation IDs and publication outcome. Keep raw transcripts optional and bounded; do not persist JSON blobs in the typed artifact tables.

Existing source tests already cover the duplicate-member rejection, cross-bucket overlap, one-shot runner flags/cancellation and transactional guided-review rollback. They do not demonstrate model self-correction, transport behavior, concurrent draft writes or a publication compare-and-swap. [Semantic regressions](../../crates/studio/src/smart_filter.rs#L259), [runner regression](../../crates/studio/tests/run_smart_filter.rs#L75), [storage rollback](../../crates/cache/src/smart_filter_storage.rs#L147)

Verification performed for this note: `cargo test -p qd-studio rejects_invalid_walkthroughs` passed (one selected unit test). All linked local files and cited line numbers were checked. No production source was edited.

## Historical design is useful intent, not implementation evidence

The older studio research proposed MCP read/write tools, stable widget IDs, immediate display/storage per accepted widget, and a transport bridge. It also contains proposals for findings and cache expiration that differ from today's implemented presentation-only, user-data persistence. In particular, its immediate-per-widget storage should not be carried forward as the publication contract: private draft progress plus atomic replacement avoids sacrificing the last good artifact. Its prediction that schema-only output is a stepping stone is consistent with the observed missing semantic feedback, but does not settle SDK/runtime compatibility or justify a large generic UI protocol. [Historical widget IDs](ai-review-studio.md#L113), [historical MCP proposal](ai-review-studio.md#L178), [historical storage](ai-review-studio.md#L259), [current studio responsibility](../../crates/studio/CLAUDE.md#L5), [current typed storage](../../crates/cache/src/brief_storage.rs#L18)
