# Guided review MCP spike

On 15 November 2026, the production guided runner completed its first authorized real-Claude probe using the installed CLI and existing subscription login. The workspace contained only three synthetic files: `src/lib.rs`, `src/config.rs`, and `README.md`. The model and service were real; no API responses or MCP validation results were scripted.

The one-off probe used the production official-Rust-SDK transport, private loopback endpoint, isolated CLI configuration, Opus/high effort, and receipt-only completion schema. It refused API-key/provider/base-URL overrides, read no credentials, and launched no native app. The run had no dollar budget flag and a five-minute cancellation deadline. The temporary probe runners were removed after integration; the offline regression suite retains repair and lifecycle coverage.

The independent server audit recorded this sequence:

| Operation | Result | Covered paths |
|---|---|---|
| Replace `greetings` with `src/` and a separate `src/lib.rs` item | Rejected: overlapping membership; revision remained 0 | 0/3 |
| Replace with separate exact code paths | Accepted; revision 1 | 2/3 |
| Validate the ordered guide | Rejected: README coverage missing | 2/3 |
| Replace with three exact file items | Accepted; revision 2 | 3/3 |
| Validate the repaired guide | Valid | 3/3 |
| Finish revision 2 | Ready; one sealed receipt, publication pending | 3/3 |

The full Claude transcript contained two model-visible error tool results. Assertions checked their presence, the ordered rejection/repair audit, complete exact-path coverage, exactly one successful finish, and a successful CLI completion echoing the sealed server receipt. The host returned the server's typed buckets, not a second generated artifact.

Observed model: `claude-opus-5-5`. Reported `total_cost_usd`: **0.180462**. Elapsed time excluding compilation: **23.9 seconds**. Session and receipt identifiers are intentionally omitted. Endpoint credentials were passed only through child environment variables and were never printed or persisted by the probe.

This establishes same-run semantic repair and normal-login MCP compatibility on the synthetic fixture. It does not measure guide quality for large real pull requests. The separate offline runner and transport tests cover lifecycle failures, forged or missing receipts, process-exit failure, cancellation, request validation and endpoint cleanup without invoking a paid model.

## Brief page repair

The first authorized real Brief probe also passed on 15 November 2026, after extracting the shared process/endpoint lifecycle. It reused the same three synthetic source files and supplied a frozen synthetic diff for the greeting function. The production Brief runner retained its existing default model selection.

The one-off Brief probe recorded six authoritative authoring calls:

| Operation | Result |
|---|---|
| Replace `greeting` page with a head-line-99 excerpt anchor | Rejected; revision remained 0 |
| Replace the complete page with the valid head-line-2 anchor | Accepted; revision 1; both section and excerpt preserved |
| Validate while the lead is missing | Rejected; accepted page preserved |
| Set the factual lead | Accepted; revision 2 |
| Validate the repaired brief | Valid |
| Finish revision 2 | Ready; one sealed receipt, publication pending |

The Claude transcript contained both repair errors. Assertions verified their order against the independent server audit, one successful finish, the matching final receipt, one typed page containing both widgets, and the accepted full artifact retained for reports.

Observed model: `claude-opus-5-5`. Reported `total_cost_usd`: **0.274931** (the CLI's floating-point value was `0.2749310000000001`). Elapsed excluding compilation: **22.5 seconds**. Session and receipt identifiers remain omitted. This was the first and only paid Brief attempt; the Guided review probe above was likewise one attempt.

Both production runners now share the lifecycle exercised here: exact MCP capabilities, narrow isolation flags, cancellation with process-group and endpoint cleanup, bounded output/pipe handling, and the required successful terminal event, process exit, and server-sealed receipt. Offline tests additionally exercise forged receipts, failed terminal/exit states, large Unicode recovery pages and abandoned descendants without paying a model.

## Integration verification

The completed app integration passed all 512 workspace tests, strict Clippy across all targets, formatting, and rustdoc with warnings denied. Tests cover publication races, cancellation, stale snapshots, transactional rollback, saved-artifact retention, and durable receipt/audit storage for both workflows.

The rebuilt development bundle was launched with `--no-splash`. A native UI check opened an existing saved Brief after database migration, verified its pages and content still rendered, and restored the original combined-stack diff and Guided review drawer. No additional model run or viewed-state changes were made during that check. Progress and cancellation behavior were verified by automated tests and code review; the native check did not exercise a new generation.
