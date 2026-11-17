//! A private guided-review draft over one captured inventory. Selectors are
//! checked here before transport adapters expose accepted progress or the host
//! obtains a finished plan. This module has no process, runtime or storage work.
//!
//! Files:
//! - `bucket_candidate.rs`, `item_candidate.rs`: complete proposed reading groups.
//! - `guide_issue.rs`, `guide_issue_code.rs`, `guide_issues.rs`: bounded diagnostics.
//! - `guide_limits.rs`: the shared content and expansion memory budget.
//! - `bucket_validation.rs`: shared selector, guidance and membership checks.
//! - `coverage_summary.rs`, `path_page.rs`: coverage and bounded inventory reads.
//! - `draft_response.rs`, `draft_status.rs`: typed command outcomes.
//! - `draft_view.rs`, `bucket_summary.rs`: accepted draft metadata.
//! - `guided_review_draft.rs`: revisioned, replayable draft commands and sealing.
//! - `canonical_digest.rs`, `readiness_receipt.rs`: content-bound finish receipts.
//! - `tests/`: repair, replay, coverage and receipt scenarios.

mod bucket_candidate;
mod bucket_summary;
mod bucket_validation;
mod canonical_digest;
mod coverage_summary;
mod draft_response;
mod draft_status;
mod draft_view;
mod guide_issue;
mod guide_issue_code;
mod guide_issues;
mod guide_limits;
mod guided_review_draft;
mod item_candidate;
mod path_page;
mod readiness_receipt;

pub use bucket_candidate::BucketCandidate;
pub use bucket_summary::BucketSummary;
pub(crate) use bucket_validation::{validate_bucket, validate_legacy_paths};
pub use coverage_summary::CoverageSummary;
pub use draft_response::DraftResponse;
pub use draft_status::DraftStatus;
pub use draft_view::DraftView;
pub use guide_issue::GuideIssue;
pub use guide_issue_code::GuideIssueCode;
pub(crate) use guide_issues::GuideIssues;
pub use guided_review_draft::GuidedReviewDraft;
pub use item_candidate::ItemCandidate;
pub use path_page::PathPage;
pub use readiness_receipt::ReadinessReceipt;

#[cfg(test)]
mod tests;
