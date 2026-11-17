//! Contract scenarios for independently repairable, immutable guided reviews.
//!
//! Files:
//! - `bucket_repair.rs`: exact/folder overlap and atomic repair.
//! - `draft_coverage.rs`: partial drafts, explicit order and paginated large inventories.
//! - `operation_replay.rs`: stale revisions, operation replay and resource limits.
//! - `finished_receipt.rs`: immutable receipt-bound output.
//! - `group_order.rs`: explicit member order, folder expansion and order-bound receipts.
//! - `fixtures.rs`: small typed proposals used across scenarios.

mod bucket_repair;
mod draft_coverage;
mod finished_receipt;
mod fixtures;
mod group_order;
mod operation_replay;
