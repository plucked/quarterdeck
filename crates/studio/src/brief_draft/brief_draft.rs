use std::collections::{BTreeMap, HashSet};
use std::ops::RangeInclusive;

use serde_json::{Value, json};

use crate::guided_review::{DraftStatus, PathPage};
use crate::parse_studio::{MAX_PAGES, check_brief_page};
use crate::{AnchorTargets, CodeSide, Studio, StudioPage};

use super::brief_digest::{brief_digest, value_bytes};
use super::{
    BRIEF_CONTRACT_VERSION, BriefDraftResponse, BriefDraftView, BriefIssue, BriefIssues,
    BriefPageSummary, FinishedBrief,
};

const MAX_WIRE_BYTES: usize = 900 * 1024;
const MAX_REQUEST_MEMORY: usize = 4 * 1024 * 1024;
const MAX_CONTENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_REPLAY_BYTES: usize = 16 * 1024 * 1024;
const RESPONSE_RESERVE: usize = 80 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;

/// One complete checked page and its original agent-authored representation.
struct AcceptedPage {
    page: StudioPage,
    candidate: Value,
    bytes: usize,
}

/// Operation identity survives both successful and rejected replacements.
struct ReplayedOperation {
    digest: String,
    response: BriefDraftResponse,
}

/// One frozen result can be published only by its exact readiness receipt.
struct SealedBrief {
    receipt: String,
    finished: FinishedBrief,
}

/// Private synchronous authoring state bound to one run, snapshot and anchor inventory.
pub struct BriefDraft {
    run_id: String,
    snapshot_id: String,
    targets: AnchorTargets,
    revision: u64,
    lead: String,
    pages: BTreeMap<String, AcceptedPage>,
    content_bytes: usize,
    operations: BTreeMap<String, ReplayedOperation>,
    replay_bytes: usize,
    sealed: Option<SealedBrief>,
}

impl BriefDraft {
    /// Takes ownership of the frozen citable diff lines, including rename aliases.
    pub fn new(run_id: String, snapshot_id: String, targets: AnchorTargets) -> Self {
        Self {
            run_id,
            snapshot_id,
            targets,
            revision: 0,
            lead: String::new(),
            pages: BTreeMap::new(),
            content_bytes: 0,
            operations: BTreeMap::new(),
            replay_bytes: 0,
            sealed: None,
        }
    }

    /// Accepted progress only; adapters page the text and metadata before sending it.
    pub fn draft(&self) -> BriefDraftView {
        BriefDraftView {
            run_id: self.run_id.clone(),
            snapshot_id: self.snapshot_id.clone(),
            revision: self.revision,
            sealed: self.sealed.is_some(),
            lead: self.lead.clone(),
            pages: self
                .pages
                .iter()
                .map(|(page_id, accepted)| BriefPageSummary {
                    page_id: page_id.clone(),
                    title: accepted.page.title.clone(),
                    widget_count: accepted.page.widgets.len(),
                })
                .collect(),
        }
    }

    pub fn page(&self, page_id: &str) -> Option<&Value> {
        self.pages.get(page_id).map(|page| &page.candidate)
    }

    /// Citable paths deliberately differ from guided-review membership inventory.
    pub fn anchor_paths(&self, offset: usize, limit: usize) -> PathPage {
        let paths = self.targets.paths();
        PathPage::from_paths(paths.iter().map(String::as_str), offset, limit)
    }

    /// Full exact ranges are available for bounded pagination by the wire adapter.
    pub fn anchor_ranges(&self, path: &str, side: CodeSide) -> Option<Vec<RangeInclusive<u32>>> {
        self.targets.ranges(path, side)
    }

    /// Replaces the complete lead without disturbing accepted pages.
    pub fn set_lead(
        &mut self,
        expected_revision: u64,
        operation_id: &str,
        lead: String,
    ) -> BriefDraftResponse {
        let payload = json!({"lead":lead});
        self.mutate(
            "set_lead",
            expected_revision,
            operation_id,
            &payload,
            |draft| {
                if lead.trim().is_empty() {
                    return Err(one_issue(
                        "empty_lead",
                        "/lead",
                        "A finished Brief needs its nonempty explanation of the change.",
                    ));
                }
                let next_bytes = draft
                    .content_bytes
                    .saturating_sub(draft.lead.len() * 2)
                    .saturating_add(lead.len() * 2);
                check_content_bytes(next_bytes)?;
                draft.lead = lead;
                draft.content_bytes = next_bytes;
                draft.revision += 1;
                Ok(None)
            },
        )
    }

    /// Validates the whole page atomically; any invalid widget preserves its predecessor.
    pub fn replace_page(
        &mut self,
        page_id: &str,
        expected_revision: u64,
        operation_id: &str,
        candidate: Value,
    ) -> BriefDraftResponse {
        let payload = json!({"page_id":page_id,"candidate":candidate});
        self.mutate(
            "replace_page",
            expected_revision,
            operation_id,
            &payload,
            |draft| {
                check_page_id(page_id)?;
                let page = check_brief_page(&candidate, &draft.targets).map_err(|mut issues| {
                    issues.identify_page(page_id);
                    issues
                })?;
                let bytes = value_bytes(&candidate)
                    .saturating_mul(2)
                    .saturating_add(page_id.len());
                let previous = draft.pages.get(page_id).map_or(0, |page| page.bytes);
                let next_bytes = draft
                    .content_bytes
                    .saturating_sub(previous)
                    .saturating_add(bytes);
                check_content_bytes(next_bytes)?;
                draft.pages.insert(
                    page_id.into(),
                    AcceptedPage {
                        page,
                        candidate,
                        bytes,
                    },
                );
                draft.content_bytes = next_bytes;
                draft.revision += 1;
                Ok(None)
            },
        )
    }

    /// Removes one private page; final page count and order are checked at completion.
    pub fn remove_page(
        &mut self,
        page_id: &str,
        expected_revision: u64,
        operation_id: &str,
    ) -> BriefDraftResponse {
        let payload = json!({"page_id":page_id});
        self.mutate(
            "remove_page",
            expected_revision,
            operation_id,
            &payload,
            |draft| {
                check_page_id(page_id)?;
                let Some(page) = draft.pages.remove(page_id) else {
                    return Err(one_issue(
                        "unknown_page",
                        "/page_id",
                        "This page ID is not in the accepted draft.",
                    ));
                };
                draft.content_bytes -= page.bytes;
                draft.revision += 1;
                Ok(None)
            },
        )
    }

    /// Validates a complete explicit reading order without freezing or publishing.
    pub fn validate_brief(
        &self,
        expected_revision: u64,
        ordered_page_ids: &[String],
    ) -> BriefDraftResponse {
        if expected_revision != self.revision {
            return self.rejected(
                "revision_conflict",
                "Read the current draft before using a new operation ID.",
            );
        }
        let issues = self.completion_issues(ordered_page_ids);
        let status = if issues.is_empty() {
            DraftStatus::Valid
        } else {
            DraftStatus::Rejected
        };
        self.response(status, issues, None)
    }

    /// Revalidates and seals both the rendered Brief and its complete original wire artifact.
    pub fn finish_brief(
        &mut self,
        expected_revision: u64,
        ordered_page_ids: &[String],
        operation_id: &str,
    ) -> BriefDraftResponse {
        let payload = json!({"ordered_page_ids":ordered_page_ids});
        self.mutate("finish_brief", expected_revision, operation_id, &payload, |draft| {
            let issues = draft.completion_issues(ordered_page_ids);
            if !issues.is_empty() { return Err(issues); }
            let studio = Studio {
                lead: draft.lead.trim().into(),
                pages: ordered_page_ids.iter().map(|page_id| draft.pages[page_id].page.clone()).collect(),
            };
            let artifact = json!({"lead":draft.lead,"pages":ordered_page_ids.iter().map(|page_id| draft.pages[page_id].candidate.clone()).collect::<Vec<_>>()});
            let bound_content = json!({"run_id":draft.run_id,"snapshot_id":draft.snapshot_id,"ordered_page_ids":ordered_page_ids,"artifact":artifact});
            let receipt = format!("{BRIEF_CONTRACT_VERSION}:{}", brief_digest(BRIEF_CONTRACT_VERSION, draft.revision, &bound_content));
            draft.sealed = Some(SealedBrief { receipt: receipt.clone(), finished: FinishedBrief { studio, artifact } });
            Ok(Some(receipt))
        })
    }

    /// Returns a clone only for this run's exact frozen receipt, never a model-supplied artifact.
    pub fn finished_studio(&self, receipt: &str) -> Result<FinishedBrief, String> {
        let Some(sealed) = &self.sealed else {
            return Err("The agent did not finish a valid Brief.".into());
        };
        if sealed.receipt != receipt {
            return Err("The terminal receipt does not match this run's frozen Brief.".into());
        }
        Ok(sealed.finished.clone())
    }

    /// A complete brief names every accepted page exactly once and obeys the existing tab limit.
    fn completion_issues(&self, order: &[String]) -> BriefIssues {
        let mut issues = BriefIssues::default();
        if order.len() > MAX_PAGES || !(1..=MAX_PAGES).contains(&self.pages.len()) {
            issues.push(BriefIssue::new(
                "invalid_page_count",
                "/ordered_page_ids",
                "A complete Brief contains one to ten pages.",
            ));
        }
        if self.lead.trim().is_empty() {
            issues.push(BriefIssue::new(
                "empty_lead",
                "/lead",
                "Set the Brief's lead before finishing.",
            ));
        }
        // Page count is constrained already; refuse an oversized ordering without a long scan.
        if order.len() > MAX_PAGES {
            return issues;
        }
        let mut seen = HashSet::new();
        for (index, page_id) in order.iter().enumerate() {
            if !self.pages.contains_key(page_id) || !seen.insert(page_id.as_str()) {
                issues.push(BriefIssue::new(
                    "invalid_order",
                    &format!("/ordered_page_ids/{index}"),
                    "Every accepted page ID must appear exactly once in reading order.",
                ));
            }
        }
        for page_id in self.pages.keys().filter(|id| !seen.contains(id.as_str())) {
            let mut issue = BriefIssue::new(
                "invalid_order",
                "/ordered_page_ids",
                "An accepted page is absent from the proposed reading order.",
            );
            issue.page_id = Some(page_id.clone());
            issues.push(issue);
        }
        issues
    }

    /// One serialized command owns replay checking, preflight, mutation and response recording.
    fn mutate(
        &mut self,
        name: &str,
        expected_revision: u64,
        operation_id: &str,
        payload: &Value,
        action: impl FnOnce(&mut Self) -> Result<Option<String>, BriefIssues>,
    ) -> BriefDraftResponse {
        let digest = brief_digest(name, expected_revision, payload);
        if let Some(operation) = self.operations.get(operation_id) {
            if operation.digest == digest {
                return operation.response.clone();
            }
            return self.rejected(
                "operation_conflict",
                "This operation ID has different recorded input. Use a fresh ID for a repair.",
            );
        }
        // Reserve enough response space before accepting state changes; no old replay is evicted.
        let response = match self.preflight(expected_revision, operation_id, payload) {
            Some(response) => response,
            None => match action(self) {
                Ok(receipt) => {
                    let status = if receipt.is_some() {
                        DraftStatus::Ready
                    } else {
                        DraftStatus::Accepted
                    };
                    self.response(status, BriefIssues::default(), receipt)
                }
                Err(issues) => self.response(DraftStatus::Rejected, issues, None),
            },
        };
        let bytes = response.storage_bytes()
            + operation_id.len()
            + digest.len()
            + size_of::<ReplayedOperation>();
        if valid_id(operation_id) && self.replay_bytes.saturating_add(bytes) <= MAX_REPLAY_BYTES {
            self.replay_bytes += bytes;
            self.operations.insert(
                operation_id.into(),
                ReplayedOperation {
                    digest,
                    response: response.clone(),
                },
            );
        }
        response
    }

    fn preflight(
        &self,
        expected_revision: u64,
        operation_id: &str,
        payload: &Value,
    ) -> Option<BriefDraftResponse> {
        if !valid_id(operation_id) {
            return Some(self.rejected(
                "invalid_identifier",
                "Operation IDs need 1–128 ASCII letters, digits, dots, underscores or hyphens.",
            ));
        }
        if self.sealed.is_some() {
            return Some(self.rejected(
                "sealed_draft",
                "This Brief is frozen; accepted pages and lead cannot change.",
            ));
        }
        if expected_revision != self.revision {
            return Some(self.rejected(
                "revision_conflict",
                "Read the current draft before using a new operation ID.",
            ));
        }
        if payload.to_string().len() > MAX_WIRE_BYTES || value_bytes(payload) > MAX_REQUEST_MEMORY {
            return Some(self.rejected("resource_limit", "This command exceeds the 900 KiB wire or 4 MiB decoded-memory bound. Use a smaller complete page."));
        }
        if self.replay_bytes.saturating_add(RESPONSE_RESERVE) > MAX_REPLAY_BYTES
            || self.revision == u64::MAX
        {
            return Some(self.rejected("resource_limit", "This run exhausted its 16 MiB replay-memory capacity. Accepted content is unchanged."));
        }
        None
    }

    fn rejected(&self, code: &str, message: &str) -> BriefDraftResponse {
        self.response(DraftStatus::Rejected, one_issue(code, "", message), None)
    }

    fn response(
        &self,
        status: DraftStatus,
        issues: BriefIssues,
        receipt: Option<String>,
    ) -> BriefDraftResponse {
        BriefDraftResponse {
            status,
            revision: self.revision,
            issues: issues.issues,
            issue_count: issues.total,
            receipt,
        }
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_IDENTIFIER_BYTES
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn check_page_id(id: &str) -> Result<(), BriefIssues> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(one_issue(
            "invalid_identifier",
            "/page_id",
            "Page IDs need 1–128 ASCII letters, digits, dots, underscores or hyphens.",
        ))
    }
}

fn check_content_bytes(bytes: usize) -> Result<(), BriefIssues> {
    if bytes <= MAX_CONTENT_BYTES {
        Ok(())
    } else {
        Err(one_issue(
            "resource_limit",
            "",
            "The complete Brief exceeds its 16 MiB content-memory budget.",
        ))
    }
}

fn one_issue(code: &str, location: &str, message: &str) -> BriefIssues {
    let mut issues = BriefIssues::default();
    issues.push(BriefIssue::new(code, location, message));
    issues
}
