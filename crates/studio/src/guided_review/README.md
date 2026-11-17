# Guided-review drafts

This module owns a synchronous, in-memory draft over an immutable changed-file
inventory. Complete bucket replacements are validated atomically; full coverage
and an explicit bucket order are required only when validating or finishing the
guide. The one-shot parser shares these path and reading-item checks.

The agent chooses bucket, item and member-file order. An item's selectors are
expanded in their given order, with folder matches in frozen inventory order and
only the first occurrence of each member kept. Exact paths let the agent choose
every file's position. Finished membership preserves that sequence.

Finishing seals exact expanded membership and returns a receipt bound to the run,
snapshot, contract, revision and ordered content. The host still owns process
success, cancellation and publication. Transport adapters decode and encode the
wire messages and impose encoded-byte limits through the draft's additional
constraint check, after resolving retries. This module does not start processes,
run an async runtime or save artifacts. Input, accepted-content and replay-memory
limits bound resources; there is no bucket-count, attempt-count or spending limit.
