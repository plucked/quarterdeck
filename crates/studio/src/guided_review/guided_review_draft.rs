use std::collections::{BTreeMap, HashSet};

use crate::{SmartBucket, SmartReviewItem};

use super::canonical_digest::CanonicalDigest;
use super::guide_limits::MAX_CONTENT_BYTES;
use super::{
    BucketCandidate, BucketSummary, CoverageSummary, DraftResponse, DraftStatus, DraftView,
    GuideIssue, GuideIssueCode, GuideIssues, PathPage, ReadinessReceipt, validate_bucket,
};

const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_REPLAY_BYTES: usize = 16 * 1024 * 1024;
const RESPONSE_RESERVE_BYTES: usize = 72 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const CONTRACT_VERSION: &str = "guided-review-v1";

/// An accepted complete bucket, before its separate display order is chosen.
struct AcceptedBucket {
    candidate: BucketCandidate,
    items: Vec<SmartReviewItem>,
    storage_bytes: usize,
}

/// The exact prior result of one operation ID, even if its validation failed.
struct ReplayedOperation {
    digest: String,
    response: DraftResponse,
}

/// Sealing retains a complete immutable guide, never a later unrelated answer.
struct FinishedGuide {
    receipt: ReadinessReceipt,
    buckets: Vec<SmartBucket>,
}

/// A run-owned draft. Its caller serializes access; every operation is synchronous.
pub struct GuidedReviewDraft {
    run_id: String,
    snapshot_id: String,
    changed_paths: Vec<String>,
    revision: u64,
    buckets: BTreeMap<String, AcceptedBucket>,
    content_bytes: usize,
    operations: BTreeMap<String, ReplayedOperation>,
    replay_bytes: usize,
    finished: Option<FinishedGuide>,
}

impl GuidedReviewDraft {
    /// Captures exact paths once, deduplicating without changing their first order.
    pub fn new(run_id: String, snapshot_id: String, changed_paths: Vec<String>) -> Self {
        let mut seen = HashSet::new();
        let changed_paths = changed_paths
            .into_iter()
            .filter(|path| seen.insert(path.clone()))
            .collect();
        Self {
            run_id,
            snapshot_id,
            changed_paths,
            revision: 0,
            buckets: BTreeMap::new(),
            content_bytes: 0,
            operations: BTreeMap::new(),
            replay_bytes: 0,
            finished: None,
        }
    }

    /// Returns accepted progress without exposing a partially validated candidate.
    pub fn draft(&self) -> DraftView {
        let buckets = self
            .buckets
            .iter()
            .map(|(bucket_id, accepted)| BucketSummary {
                bucket_id: bucket_id.clone(),
                label: accepted.candidate.label.trim().into(),
                item_count: accepted.items.len(),
                file_count: accepted.items.iter().map(|item| item.files.len()).sum(),
            })
            .collect();
        DraftView {
            run_id: self.run_id.clone(),
            snapshot_id: self.snapshot_id.clone(),
            revision: self.revision,
            sealed: self.finished.is_some(),
            buckets,
            coverage: self.coverage(),
        }
    }

    /// Reads the accepted selectors needed to repair one complete bucket.
    pub fn bucket(&self, bucket_id: &str) -> Option<&BucketCandidate> {
        self.buckets.get(bucket_id).map(|bucket| &bucket.candidate)
    }

    /// Includes every captured changed path, regardless of readable diff availability.
    pub fn inventory(&self, offset: usize, limit: usize) -> PathPage {
        PathPage::from_paths(self.changed_paths.iter().map(String::as_str), offset, limit)
    }

    /// Missing membership follows captured inventory order and can be paged to completion.
    pub fn missing_paths(&self, offset: usize, limit: usize) -> PathPage {
        let covered = self.covered_paths();
        PathPage::from_paths(
            self.changed_paths
                .iter()
                .map(String::as_str)
                .filter(|path| !covered.contains(path)),
            offset,
            limit,
        )
    }

    /// Atomically replaces one complete bucket; incomplete global coverage is allowed.
    pub fn replace_bucket(
        &mut self,
        bucket_id: &str,
        expected_revision: u64,
        operation_id: &str,
        candidate: BucketCandidate,
    ) -> DraftResponse {
        self.replace_bucket_with_constraint(
            bucket_id,
            expected_revision,
            operation_id,
            candidate,
            |_| None,
        )
    }

    /// Applies an adapter's additional constraint inside the same replay and mutation boundary.
    pub(crate) fn replace_bucket_with_constraint(
        &mut self,
        bucket_id: &str,
        expected_revision: u64,
        operation_id: &str,
        candidate: BucketCandidate,
        constraint: impl FnOnce(&BucketCandidate) -> Option<GuideIssue>,
    ) -> DraftResponse {
        // Replay identity precedes revision checks, including after a successful finish.
        let mut digest = operation_digest("replace_bucket", expected_revision);
        digest.text(bucket_id);
        digest.candidate(&candidate);
        let digest = digest.finish();
        if let Some(response) = self.replayed_response(operation_id, &digest) {
            return response;
        }
        let request_bytes = candidate.storage_bytes().saturating_add(bucket_id.len());
        if let Some(response) =
            self.mutation_rejection(expected_revision, operation_id, request_bytes)
        {
            return self.remember(operation_id, digest, response);
        }
        // Adapter limits share retry identity without teaching the draft a wire format.
        if let Some(issue) = constraint(&candidate) {
            let mut issues = GuideIssues::default();
            issues.push(issue);
            let response = self.response(DraftStatus::Rejected, issues, None);
            return self.remember(operation_id, digest, response);
        }
        if !valid_identifier(bucket_id) {
            let response = self.rejection(GuideIssueCode::InvalidIdentifier, "Bucket IDs must contain 1–128 ASCII letters, digits, dots, underscores or hyphens.");
            return self.remember(operation_id, digest, response);
        }

        // Independent semantic problems are collected without touching accepted state.
        let items = match validate_bucket(bucket_id, &candidate, &self.changed_paths) {
            Ok(items) => items,
            Err(issues) => {
                let response = self.response(DraftStatus::Rejected, issues, None);
                return self.remember(operation_id, digest, response);
            }
        };
        let storage_bytes = expanded_bytes(&candidate, &items).saturating_add(bucket_id.len());
        let prior_bytes = self
            .buckets
            .get(bucket_id)
            .map_or(0, |bucket| bucket.storage_bytes);
        let next_bytes = self
            .content_bytes
            .saturating_sub(prior_bytes)
            .saturating_add(storage_bytes);
        if next_bytes > MAX_CONTENT_BYTES {
            let response = self.rejection(GuideIssueCode::ResourceLimit, "The expanded draft exceeds the 16 MiB content-memory limit. Use smaller guidance or fewer redundant groups.");
            return self.remember(operation_id, digest, response);
        }

        // One revision covers the whole replacement, even when many selectors expand.
        self.buckets.insert(
            bucket_id.into(),
            AcceptedBucket {
                candidate,
                items,
                storage_bytes,
            },
        );
        self.content_bytes = next_bytes;
        self.revision += 1;
        let response = self.response(DraftStatus::Accepted, GuideIssues::default(), None);
        self.remember(operation_id, digest, response)
    }

    /// Removes one accepted bucket without requiring the remaining draft to cover everything.
    pub fn remove_bucket(
        &mut self,
        bucket_id: &str,
        expected_revision: u64,
        operation_id: &str,
    ) -> DraftResponse {
        let mut digest = operation_digest("remove_bucket", expected_revision);
        digest.text(bucket_id);
        let digest = digest.finish();
        if let Some(response) = self.replayed_response(operation_id, &digest) {
            return response;
        }
        if let Some(response) =
            self.mutation_rejection(expected_revision, operation_id, bucket_id.len())
        {
            return self.remember(operation_id, digest, response);
        }
        // A missing target is a repairable error, not a successful revision increment.
        let Some(bucket) = self.buckets.remove(bucket_id) else {
            let response = self.rejection(
                GuideIssueCode::UnknownBucket,
                "This bucket ID is not in the accepted draft.",
            );
            return self.remember(operation_id, digest, response);
        };
        self.content_bytes -= bucket.storage_bytes;
        self.revision += 1;
        let response = self.response(DraftStatus::Accepted, GuideIssues::default(), None);
        self.remember(operation_id, digest, response)
    }

    /// Checks current full coverage and exact bucket order without freezing or publishing.
    pub fn validate_guide(
        &self,
        expected_revision: u64,
        ordered_bucket_ids: &[String],
    ) -> DraftResponse {
        if expected_revision != self.revision {
            return self.revision_rejection();
        }
        let issues = self.guide_issues(ordered_bucket_ids);
        let status = if issues.is_empty() {
            DraftStatus::Valid
        } else {
            DraftStatus::Rejected
        };
        self.response(status, issues, None)
    }

    /// Revalidates and freezes the complete guide. Publication still belongs to the host.
    pub fn finish_guide(
        &mut self,
        expected_revision: u64,
        ordered_bucket_ids: &[String],
        operation_id: &str,
    ) -> DraftResponse {
        let mut digest = operation_digest("finish_guide", expected_revision);
        for bucket_id in ordered_bucket_ids {
            digest.text(bucket_id);
        }
        let digest = digest.finish();
        if let Some(response) = self.replayed_response(operation_id, &digest) {
            return response;
        }
        let request_bytes = order_bytes(ordered_bucket_ids);
        if let Some(response) =
            self.mutation_rejection(expected_revision, operation_id, request_bytes)
        {
            return self.remember(operation_id, digest, response);
        }

        // Rechecking here binds readiness to the current revision, not earlier validation.
        let validation = self.validate_guide(expected_revision, ordered_bucket_ids);
        if validation.is_error() {
            return self.remember(operation_id, digest, validation);
        }
        let buckets = self.ordered_buckets(ordered_bucket_ids);
        let receipt = self.readiness_receipt(ordered_bucket_ids, &buckets);
        self.finished = Some(FinishedGuide {
            receipt: receipt.clone(),
            buckets,
        });
        let response = self.response(DraftStatus::Ready, GuideIssues::default(), Some(receipt));
        self.remember(operation_id, digest, response)
    }

    /// Resolves only an exact terminal receipt to the immutable guide that issued it.
    pub fn finished_buckets(&self, receipt: &str) -> Result<Vec<SmartBucket>, String> {
        let Some(finished) = &self.finished else {
            return Err("The agent did not finish a valid guided review.".into());
        };
        if receipt != finished.receipt.as_str() {
            return Err(
                "The terminal receipt does not match this run's frozen guided review.".into(),
            );
        }
        Ok(finished.buckets.clone())
    }

    /// Canonical paths, not counts alone, decide whether the accepted buckets cover the snapshot.
    fn covered_paths(&self) -> HashSet<&str> {
        self.buckets
            .values()
            .flat_map(|bucket| &bucket.items)
            .flat_map(|item| item.files.iter().map(String::as_str))
            .collect()
    }

    fn coverage(&self) -> CoverageSummary {
        let covered = self.covered_paths();
        let missing_paths = self
            .changed_paths
            .iter()
            .filter(|path| !covered.contains(path.as_str()))
            .count();
        CoverageSummary {
            total_paths: self.changed_paths.len(),
            covered_paths: self.changed_paths.len() - missing_paths,
            missing_paths,
        }
    }

    /// Orders name every accepted bucket exactly once; cross-bucket overlap remains legal.
    fn guide_issues(&self, ordered_bucket_ids: &[String]) -> GuideIssues {
        let mut issues = GuideIssues::default();
        if order_bytes(ordered_bucket_ids) > MAX_REQUEST_BYTES {
            issues.push(GuideIssue::new(
                GuideIssueCode::ResourceLimit,
                "The ordered bucket IDs exceed the 1 MiB request-memory limit.",
            ));
            return issues;
        }
        if self.buckets.is_empty() {
            issues.push(GuideIssue::new(
                GuideIssueCode::InvalidOrder,
                "A finished guide needs at least one accepted bucket.",
            ));
        }
        let mut seen = HashSet::new();
        for (index, bucket_id) in ordered_bucket_ids.iter().enumerate() {
            if !self.buckets.contains_key(bucket_id) || !seen.insert(bucket_id.as_str()) {
                let mut issue = GuideIssue::new(
                    GuideIssueCode::InvalidOrder,
                    "Ordered bucket IDs must name each accepted bucket exactly once.",
                );
                issue.bucket_id = Some(bucket_id.clone());
                issue.locations.push(format!("/ordered_bucket_ids/{index}"));
                issues.push(issue);
            }
        }
        for bucket_id in self
            .buckets
            .keys()
            .filter(|bucket_id| !seen.contains(bucket_id.as_str()))
        {
            let mut issue = GuideIssue::new(
                GuideIssueCode::InvalidOrder,
                "An accepted bucket is missing from the explicit reading order.",
            );
            issue.bucket_id = Some(bucket_id.clone());
            issue.locations.push("/ordered_bucket_ids".into());
            issues.push(issue);
        }

        // A bounded concrete sample accompanies the exact total; missing_paths pages the rest.
        let covered = self.covered_paths();
        for path in self
            .changed_paths
            .iter()
            .filter(|path| !covered.contains(path.as_str()))
        {
            let mut issue = GuideIssue::new(
                GuideIssueCode::MissingPaths,
                "This captured changed file is not covered by any accepted bucket.",
            );
            issue.path = Some(path.clone());
            issues.push(issue);
        }
        issues
    }

    /// The order length fits u32 because every accepted bucket consumes bounded content memory.
    fn ordered_buckets(&self, ordered_bucket_ids: &[String]) -> Vec<SmartBucket> {
        ordered_bucket_ids
            .iter()
            .zip(1_u32..)
            .map(|(bucket_id, order)| {
                let accepted = &self.buckets[bucket_id];
                SmartBucket {
                    order,
                    label: accepted.candidate.label.trim().into(),
                    description: accepted.candidate.description.trim().into(),
                    suggest_viewed: accepted.candidate.suggest_viewed,
                    items: accepted.items.clone(),
                }
            })
            .collect()
    }

    /// The receipt covers run identity, snapshot identity/inventory and exact final expanded content.
    fn readiness_receipt(
        &self,
        ordered_bucket_ids: &[String],
        buckets: &[SmartBucket],
    ) -> ReadinessReceipt {
        let mut digest = CanonicalDigest::new(CONTRACT_VERSION);
        digest.text(&self.run_id);
        digest.text(&self.snapshot_id);
        digest.number(self.revision);
        digest.number(u64::try_from(self.changed_paths.len()).unwrap_or(u64::MAX));
        for path in &self.changed_paths {
            digest.text(path);
        }
        for (bucket_id, bucket) in ordered_bucket_ids.iter().zip(buckets) {
            digest.text(bucket_id);
            digest.number(u64::from(bucket.order));
            digest.candidate(&self.buckets[bucket_id].candidate);
            for item in &bucket.items {
                digest.text(&item.label);
                digest.text(&item.description);
                digest.number(u64::try_from(item.files.len()).unwrap_or(u64::MAX));
                for path in &item.files {
                    digest.text(path);
                }
            }
        }
        ReadinessReceipt::new(&digest.finish())
    }

    /// Changed input cannot reuse an operation ID, even when its expected revision is now stale.
    fn replayed_response(&self, operation_id: &str, digest: &str) -> Option<DraftResponse> {
        self.operations.get(operation_id).map(|operation| {
            if operation.digest == digest {
                operation.response.clone()
            } else {
                self.rejection(GuideIssueCode::OperationConflict, "This operation ID was already used for different input. Use a new operation ID for a repair.")
            }
        })
    }

    /// Reserves replay space before changing state, so every accepted command stays replayable.
    fn mutation_rejection(
        &self,
        expected_revision: u64,
        operation_id: &str,
        request_bytes: usize,
    ) -> Option<DraftResponse> {
        if !valid_identifier(operation_id) {
            return Some(self.rejection(GuideIssueCode::InvalidIdentifier, "Operation IDs must contain 1–128 ASCII letters, digits, dots, underscores or hyphens."));
        }
        if self.finished.is_some() {
            return Some(self.rejection(
                GuideIssueCode::SealedDraft,
                "This guide is frozen. Its accepted content cannot change after finishing.",
            ));
        }
        if expected_revision != self.revision {
            return Some(self.revision_rejection());
        }
        if request_bytes > MAX_REQUEST_BYTES {
            return Some(self.rejection(GuideIssueCode::ResourceLimit, "The request exceeds the 1 MiB input-memory limit. Replace a smaller complete bucket."));
        }
        if self.replay_bytes.saturating_add(RESPONSE_RESERVE_BYTES) > MAX_REPLAY_BYTES
            || self.revision == u64::MAX
        {
            return Some(self.rejection(GuideIssueCode::ResourceLimit, "This run has exhausted its 16 MiB replay-memory capacity; its accepted draft is unchanged."));
        }
        None
    }

    fn revision_rejection(&self) -> DraftResponse {
        self.rejection(GuideIssueCode::RevisionConflict, "The expected revision is stale. Read the current draft before using a new operation ID.")
    }

    fn rejection(&self, code: GuideIssueCode, message: &str) -> DraftResponse {
        let mut issues = GuideIssues::default();
        issues.push(GuideIssue::new(code, message));
        self.response(DraftStatus::Rejected, issues, None)
    }

    fn response(
        &self,
        status: DraftStatus,
        issues: GuideIssues,
        receipt: Option<ReadinessReceipt>,
    ) -> DraftResponse {
        DraftResponse {
            status,
            revision: self.revision,
            coverage: self.coverage(),
            issues: issues.issues,
            issue_count: issues.total,
            receipt,
        }
    }

    /// No eviction makes retry identity stable for the whole lifetime of a run.
    fn remember(
        &mut self,
        operation_id: &str,
        digest: String,
        response: DraftResponse,
    ) -> DraftResponse {
        let bytes = operation_id.len()
            + digest.len()
            + response.storage_bytes()
            + size_of::<ReplayedOperation>();
        if valid_identifier(operation_id)
            && self.replay_bytes.saturating_add(bytes) <= MAX_REPLAY_BYTES
        {
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
}

fn operation_digest(operation: &str, expected_revision: u64) -> CanonicalDigest {
    let mut digest = CanonicalDigest::new(operation);
    digest.number(expected_revision);
    digest
}

fn valid_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier.len() <= MAX_IDENTIFIER_BYTES
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn order_bytes(ordered_bucket_ids: &[String]) -> usize {
    ordered_bucket_ids.iter().fold(0_usize, |bytes, id| {
        bytes.saturating_add(size_of::<String>() + id.len())
    })
}

/// Expanded membership is charged as well as selectors, avoiding cheap folder fan-out.
fn expanded_bytes(candidate: &BucketCandidate, items: &[SmartReviewItem]) -> usize {
    items.iter().fold(candidate.storage_bytes(), |bytes, item| {
        item.files.iter().fold(
            bytes.saturating_add(
                size_of::<SmartReviewItem>() + item.label.len() + item.description.len(),
            ),
            |bytes, path| bytes.saturating_add(size_of::<String>() + path.len()),
        )
    })
}
