//! A synchronous private Brief draft over frozen code-anchor targets.
//!
//! Files:
//! - `brief_draft.rs`: atomic page commands, revision replay and immutable finishing.
//! - `brief_draft_response.rs`, `brief_draft_view.rs`, `brief_page_summary.rs`: typed outcomes and accepted progress.
//! - `brief_issue.rs`, `brief_issues.rs`: bounded repair diagnostics and anchor evidence.
//! - `brief_schema_checks.rs`: strict shape checks against the existing compiled schema.
//! - `brief_anchor_checks.rs`: shared anchor validation with precise widget pointers.
//! - `brief_digest.rs`: canonical typed request and content digests.
//! - `brief_contract_version.rs`: the receipt/report contract identity.
//! - `finished_brief.rs`: the sealed typed studio and original accepted wire artifact.
//! - `tests/`: page repair, replay and finish scenarios.

mod brief_anchor_checks;
mod brief_contract_version;
mod brief_digest;
mod brief_draft;
mod brief_draft_response;
mod brief_draft_view;
mod brief_issue;
mod brief_issues;
mod brief_page_summary;
mod brief_schema_checks;
mod finished_brief;

pub(crate) use brief_anchor_checks::check_page_anchors;
pub use brief_contract_version::BRIEF_CONTRACT_VERSION;
pub use brief_draft::BriefDraft;
pub use brief_draft_response::BriefDraftResponse;
pub use brief_draft_view::BriefDraftView;
pub use brief_issue::BriefIssue;
pub(crate) use brief_issues::BriefIssues;
pub use brief_page_summary::BriefPageSummary;
pub(crate) use brief_schema_checks::check_page_schema;
pub use finished_brief::FinishedBrief;

#[cfg(test)]
mod tests;
