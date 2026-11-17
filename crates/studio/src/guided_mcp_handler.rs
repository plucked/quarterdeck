use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

use qd_agent_mcp::{McpHandler, McpToolDescriptor, McpToolResult};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::guided_review::{DraftResponse, DraftStatus, GuidedReviewDraft, PathPage};
use crate::mcp_json_page::{MAX_PAGE_CHARACTERS, mcp_json_page};
use crate::wire_bucket_candidate::{WireBucketCandidate, candidate_json, candidate_wire_issue};
use crate::{AgentProgress, GuidedReviewRun, GuidedToolAudit, SmartBucket};

/// The small catalog is explicit both in discovery and the CLI permission grant.
pub(crate) const GUIDED_TOOL_NAMES: &[&str] = &[
    "review_context",
    "review_inventory",
    "guide_missing_paths",
    "guide_read_draft",
    "guide_read_bucket",
    "guide_replace_bucket",
    "guide_remove_bucket",
    "guide_validate",
    "guide_finish",
];
const MAX_AUDIT_ENTRIES: usize = 512;
const MAX_PROGRESS_ENTRIES: usize = 64;
const MAX_SUMMARY_PAGE_BYTES: usize = 48 * 1024;

/// Owns a private domain draft behind the serialization gate used by every MCP call.
pub(crate) struct GuidedMcpHandler {
    state: Mutex<GuidedHandlerState>,
}

/// All mutable authoring data shares one lock, including endpoint revocation.
struct GuidedHandlerState {
    draft: GuidedReviewDraft,
    closed: bool,
    audit: VecDeque<GuidedToolAudit>,
    progress: VecDeque<AgentProgress>,
}

/// Distinguishes a semantic validation error from an accepted bounded tool result.
struct GuidedToolResponse {
    is_error: bool,
    value: Value,
}

impl GuidedToolResponse {
    fn read(value: Value) -> Self {
        Self {
            is_error: false,
            value,
        }
    }
}

/// Complete mutation arguments; unknown fields are rejected before domain dispatch.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceArguments {
    run_id: String,
    snapshot_id: String,
    bucket_id: String,
    expected_revision: u64,
    operation_id: String,
    candidate: WireBucketCandidate,
}

/// Removing a bucket uses the same optimistic revision and retry identity.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoveArguments {
    run_id: String,
    snapshot_id: String,
    bucket_id: String,
    expected_revision: u64,
    operation_id: String,
}

/// Validation chooses the complete bucket ordering without changing the draft.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    ordered_bucket_ids: Vec<String>,
}

/// Finish seals a current valid ordering with its own idempotency key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    ordered_bucket_ids: Vec<String>,
    operation_id: String,
}

impl GuidedMcpHandler {
    pub(crate) fn new(run: GuidedReviewRun<'_>) -> Self {
        Self {
            state: Mutex::new(GuidedHandlerState {
                draft: GuidedReviewDraft::new(
                    run.run_id.into(),
                    run.snapshot_id.into(),
                    run.changed_paths.into(),
                ),
                closed: false,
                audit: VecDeque::new(),
                progress: VecDeque::new(),
            }),
        }
    }

    /// A panic closes authoring rather than accepting potentially inconsistent state.
    fn lock(&self) -> Result<MutexGuard<'_, GuidedHandlerState>, String> {
        self.state
            .lock()
            .map_err(|_| "The guide authoring state is unavailable.".into())
    }

    pub(crate) fn audit(&self) -> Vec<GuidedToolAudit> {
        self.lock()
            .map(|state| state.audit.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn finished_buckets(&self, receipt: &str) -> Result<Vec<SmartBucket>, String> {
        self.lock()?.draft.finished_buckets(receipt)
    }
}

impl crate::run_mcp_handler::RunMcpHandler for GuidedMcpHandler {
    /// Revokes mutations under the same gate that applies them.
    fn close(&self) {
        if let Ok(mut state) = self.lock() {
            state.closed = true;
        }
    }

    fn drain_progress(&self) -> Vec<AgentProgress> {
        self.lock()
            .map(|mut state| state.progress.drain(..).collect())
            .unwrap_or_default()
    }
}

impl McpHandler for GuidedMcpHandler {
    fn tools(&self) -> Vec<McpToolDescriptor> {
        GUIDED_TOOL_NAMES
            .iter()
            .map(|name| descriptor(name))
            .collect()
    }

    fn call_tool(&self, name: &str, arguments: Value) -> McpToolResult {
        // Serialize domain operations and reject requests after cancellation or completion.
        let result = self.lock().and_then(|mut state| {
            if state.closed {
                return Err(
                    "This run is closed; no further authoring operation is allowed.".into(),
                );
            }
            dispatch(&mut state, name, arguments)
        });
        match result {
            Ok(response) => {
                // Claude substitutes structured success data for text, so it contains all guidance.
                let text = response.value.to_string();
                if response.is_error {
                    McpToolResult::failure(text, response.value)
                } else {
                    McpToolResult::success(text, response.value)
                }
            }
            Err(message) => {
                let value = json!({"status":"rejected","issues":[{"code":"invalid_request","message":message}],"repair":"Read review_context and guide_read_draft, then resend the documented arguments."});
                McpToolResult::failure(value.to_string(), value)
            }
        }
    }
}

/// Routes only the fixed catalog; no tool can access arbitrary application state.
fn dispatch(
    state: &mut GuidedHandlerState,
    name: &str,
    arguments: Value,
) -> Result<GuidedToolResponse, String> {
    // Context and inventory reads are bounded and describe this endpoint's frozen snapshot.
    match name {
        "review_context" => {
            require_keys(&arguments, &[])?;
            return Ok(GuidedToolResponse::read(draft_json(&state.draft, 0, 32)));
        }
        "guide_read_draft" => {
            require_optional_keys(&arguments, &[], &["offset", "limit"])?;
            let offset = optional_usize(&arguments, "offset", 0)?;
            let limit = optional_usize(&arguments, "limit", 32)?;
            return Ok(GuidedToolResponse::read(draft_json(
                &state.draft,
                offset,
                limit,
            )));
        }
        "review_inventory" | "guide_missing_paths" => {
            require_keys(&arguments, &["offset", "limit"])?;
            let offset = usize_argument(&arguments, "offset")?;
            let limit = usize_argument(&arguments, "limit")?;
            let page = match name {
                "review_inventory" => state.draft.inventory(offset, limit),
                _ => state.draft.missing_paths(offset, limit),
            };
            return Ok(GuidedToolResponse::read(page_json(&page)));
        }
        "guide_read_bucket" => {
            require_optional_keys(&arguments, &["bucket_id"], &["offset", "limit"])?;
            let bucket_id = arguments["bucket_id"]
                .as_str()
                .ok_or("bucket_id must be a string")?;
            let offset = optional_usize(&arguments, "offset", 0)?;
            let limit = optional_usize(&arguments, "limit", MAX_PAGE_CHARACTERS)?;
            return Ok(GuidedToolResponse::read(bucket_page_json(
                &state.draft,
                bucket_id,
                offset,
                limit,
            )?));
        }
        _ => {}
    }
    // Run identities are checked before the domain can mutate or seal the draft.
    let response = match name {
        "guide_replace_bucket" => {
            let arguments: ReplaceArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(state, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.replace_bucket_with_constraint(
                &arguments.bucket_id,
                arguments.expected_revision,
                &arguments.operation_id,
                arguments.candidate.into_candidate(),
                candidate_wire_issue,
            )
        }
        "guide_remove_bucket" => {
            let arguments: RemoveArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(state, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.remove_bucket(
                &arguments.bucket_id,
                arguments.expected_revision,
                &arguments.operation_id,
            )
        }
        "guide_validate" => {
            let arguments: ValidateArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(state, &arguments.run_id, &arguments.snapshot_id)?;
            state
                .draft
                .validate_guide(arguments.expected_revision, &arguments.ordered_bucket_ids)
        }
        "guide_finish" => {
            let arguments: FinishArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(state, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.finish_guide(
                arguments.expected_revision,
                &arguments.ordered_bucket_ids,
                &arguments.operation_id,
            )
        }
        _ => return Err("Unknown guided review tool.".into()),
    };
    let value = record_response(state, name, &response);
    Ok(GuidedToolResponse {
        is_error: response.is_error(),
        value,
    })
}

/// Records bounded audit and UI events independently from the model transcript.
fn record_response(state: &mut GuidedHandlerState, name: &str, response: &DraftResponse) -> Value {
    // Keep bounded audit and progress independently from the model's transcript.
    let value = response_json(response);
    state.audit.push_back(GuidedToolAudit {
        tool_name: name.into(),
        status: status_text(response.status).into(),
        revision: response.revision,
        issue_codes: response
            .issues
            .iter()
            .map(|issue| issue.code.as_str().into())
            .collect(),
    });
    if state.audit.len() > MAX_AUDIT_ENTRIES {
        state.audit.pop_front();
    }
    state.progress.push_back(AgentProgress::ReviewingGuide {
        buckets: state.draft.draft().buckets.len(),
        covered_files: response.coverage.covered_paths,
        total_files: response.coverage.total_paths,
        issues: response.issue_count,
    });
    if state.progress.len() > MAX_PROGRESS_ENTRIES {
        state.progress.pop_front();
    }
    value
}

fn check_scope(state: &GuidedHandlerState, run_id: &str, snapshot_id: &str) -> Result<(), String> {
    let view = state.draft.draft();
    if view.run_id != run_id || view.snapshot_id != snapshot_id {
        return Err(
            "run_id or snapshot_id differs from this frozen review; read review_context.".into(),
        );
    }
    Ok(())
}

fn require_keys(arguments: &Value, keys: &[&str]) -> Result<(), String> {
    let object = arguments
        .as_object()
        .ok_or("Arguments must be an object.")?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(format!(
            "Expected exactly these argument keys: {}",
            keys.join(", ")
        ));
    }
    Ok(())
}

fn usize_argument(arguments: &Value, key: &str) -> Result<usize, String> {
    arguments[key]
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .ok_or_else(|| format!("{key} must be a nonnegative integer."))
}

fn status_text(status: DraftStatus) -> &'static str {
    match status {
        DraftStatus::Accepted => "accepted",
        DraftStatus::Rejected => "rejected",
        DraftStatus::Valid => "valid",
        DraftStatus::Ready => "ready",
    }
}

/// All diagnostics and the finish receipt stay in both MCP representations.
fn response_json(response: &DraftResponse) -> Value {
    json!({
        "status":status_text(response.status),"revision":response.revision,
        "coverage":{"total_paths":response.coverage.total_paths,"covered_paths":response.coverage.covered_paths,"missing_paths":response.coverage.missing_paths},
        "issues":response.issues.iter().map(|issue| json!({"code":issue.code.as_str(),"bucket_id":issue.bucket_id,"path":issue.path,"item_indexes":issue.item_indexes,"locations":issue.locations,"selectors":issue.selectors,"message":issue.message})).collect::<Vec<_>>(),
        "issue_count":response.issue_count,
        "validation_receipt":response.receipt.as_ref().map(crate::guided_review::ReadinessReceipt::as_str),
        "publication_status":"pending",
        "next_step":"Repair rejected candidates using the diagnostics. For a newly accepted operation use its revision. After recovering an uncertain operation by retry, call guide_read_draft to obtain the latest revision; validate complete coverage, then guide_finish and echo its validation_receipt in the final structured answer."
    })
}

/// Pages accepted summaries while retaining a precise revision for recovery.
fn draft_json(draft: &GuidedReviewDraft, offset: usize, limit: usize) -> Value {
    let view = draft.draft();
    let mut buckets = Vec::new();
    let mut bytes = 0;
    for bucket in view.buckets.iter().skip(offset).take(limit.clamp(1, 256)) {
        let label: String = bucket.label.chars().take(96).collect();
        let summary = json!({"bucket_id":bucket.bucket_id,"label":label,"item_count":bucket.item_count,"file_count":bucket.file_count});
        let encoded = summary.to_string().len();
        if bytes + encoded > MAX_SUMMARY_PAGE_BYTES {
            break;
        }
        bytes += encoded;
        buckets.push(summary);
    }
    let next = offset.saturating_add(buckets.len());
    let next_offset = (next < view.buckets.len()).then_some(next);
    json!({"run_id":view.run_id,"snapshot_id":view.snapshot_id,"revision":view.revision,"sealed":view.sealed,
        "buckets":buckets,"bucket_count":view.buckets.len(),"offset":offset,"next_offset":next_offset,
        "coverage":{"total_paths":view.coverage.total_paths,"covered_paths":view.coverage.covered_paths,"missing_paths":view.coverage.missing_paths},
        "inventory":page_json(&draft.inventory(0, 64)),
        "next_step":"Follow next_offset with guide_read_draft offset/limit to recover all bucket IDs. Labels are abbreviated summaries; guide_read_bucket preserves full candidate JSON in character-offset fragments. Mutations require these exact identities and the latest expected_revision. Rejections leave revision unchanged."
    })
}

/// Slices canonical JSON by Unicode scalar offsets so even one long field is recoverable.
fn bucket_page_json(
    draft: &GuidedReviewDraft,
    bucket_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    let candidate = draft
        .bucket(bucket_id)
        .ok_or("Unknown bucket_id; read guide_read_draft.")?;
    Ok(mcp_json_page(
        &candidate_json(candidate),
        json!({"bucket_id":bucket_id,"revision":draft.draft().revision}),
        offset,
        limit,
    ))
}

/// Accepts only required keys and the explicitly documented optional pagination keys.
fn require_optional_keys(
    arguments: &Value,
    required: &[&str],
    optional: &[&str],
) -> Result<(), String> {
    let object = arguments
        .as_object()
        .ok_or("Arguments must be an object.")?;
    if required.iter().any(|key| !object.contains_key(*key))
        || object
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err("Arguments do not match the documented required and optional keys.".into());
    }
    Ok(())
}

fn optional_usize(arguments: &Value, key: &str, default: usize) -> Result<usize, String> {
    if arguments.get(key).is_none() {
        return Ok(default);
    }
    usize_argument(arguments, key)
}

fn page_json(page: &PathPage) -> Value {
    json!({"paths":page.paths,"total":page.total,"next_offset":page.next_offset})
}

/// Describes the exact bounded tool surface using JSON Schema objects.
fn descriptor(name: &str) -> McpToolDescriptor {
    let text = json!({"type":"string","minLength":1});
    let revision = json!({"type":"integer","minimum":0});
    let mut properties = serde_json::Map::new();
    let description = match name {
        "review_context" => {
            "Read the frozen run identity, inventory page, current revision and coverage."
        }
        "guide_read_draft" => {
            properties.insert("offset".into(), revision.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":256}),
            );
            "Read current revision and a bounded bucket-summary page. Optional offset/limit; follow next_offset."
        }
        "review_inventory" | "guide_missing_paths" => {
            properties.insert("offset".into(), revision.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":256}),
            );
            "Read a bounded page of changed paths; follow next_offset until null."
        }
        "guide_read_bucket" => {
            properties.insert("bucket_id".into(), text.clone());
            properties.insert("offset".into(), revision.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_PAGE_CHARACTERS}),
            );
            "Read a canonical candidate JSON fragment; optional offset/limit count Unicode scalars. Concatenate successive next_offset pages at the same revision to recover the complete bucket."
        }
        _ => {
            properties.insert("run_id".into(), text.clone());
            properties.insert("snapshot_id".into(), text.clone());
            properties.insert("expected_revision".into(), revision);
            if name != "guide_validate" {
                properties.insert("operation_id".into(), text.clone());
            }
            match name {
                "guide_replace_bucket" | "guide_remove_bucket" => {
                    properties.insert("bucket_id".into(), text.clone());
                }
                _ => {
                    properties.insert(
                        "ordered_bucket_ids".into(),
                        json!({"type":"array","items":text,"minItems":1}),
                    );
                }
            }
            if name == "guide_replace_bucket" {
                properties.insert("candidate".into(), candidate_schema());
            }
            match name {
                "guide_replace_bucket" => {
                    "Atomically validate and replace one whole bucket. Errors leave the accepted draft unchanged; repair the complete candidate."
                }
                "guide_remove_bucket" => "Remove an accepted bucket at the expected revision.",
                "guide_validate" => {
                    "Check the complete ordered guide and every changed file's coverage without sealing it. Repair any diagnostics."
                }
                _ => {
                    "Validate and seal the complete current guide. Return its validation_receipt in your final answer; publication remains pending."
                }
            }
        }
    };
    let required: Vec<_> = properties
        .keys()
        .filter(|key| {
            !matches!(name, "guide_read_draft" | "guide_read_bucket")
                || !matches!(key.as_str(), "offset" | "limit")
        })
        .cloned()
        .collect();
    McpToolDescriptor {
        name: name.into(),
        description: description.into(),
        input_schema: json!({"type":"object","additionalProperties":false,"required":required,"properties":properties}),
        output_schema: Some(json!({"type":"object"})),
        read_only: !matches!(
            name,
            "guide_replace_bucket" | "guide_remove_bucket" | "guide_finish"
        ),
    }
}

fn candidate_schema() -> Value {
    let text = json!({"type":"string","minLength":1});
    json!({"type":"object","additionalProperties":false,"required":["label","description","suggest_viewed","items"],
    "properties":{"label":text,"description":text,"suggest_viewed":{"type":"boolean"},"items":{"type":"array","description":"Reading stops in the intended review order.","minItems":1,"items":{
        "type":"object","additionalProperties":false,"required":["label","description","paths"],
        "properties":{"label":text,"description":text,"paths":{"type":"array","description":"Files in the intended reading order. Exact paths retain this order; folders expand in frozen inventory order at their position, with first occurrence winning. Explain the sequence in the item description.","minItems":1,"items":text}}
    }}}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(handler: &GuidedMcpHandler, name: &str, arguments: Value) -> Value {
        let response = dispatch(&mut handler.lock().unwrap(), name, arguments).unwrap();
        assert!(!response.is_error, "{}", response.value);
        response.value
    }

    /// Small candidates stay typed; Unicode fragments reconstruct a large candidate exactly.
    #[test]
    fn bucket_recovery_pages_preserve_unicode_and_complete_content() {
        let paths: Vec<String> = (0..3803)
            .map(|index| format!("assets/界面-{index}.png"))
            .collect();
        let handler = GuidedMcpHandler::new(GuidedReviewRun {
            run_id: "run",
            snapshot_id: "snapshot",
            changed_paths: &paths,
        });
        let candidate = json!({"label":"Assets","description":"Read the format notes and compare generated images.","suggest_viewed":false,
            "items":[{"label":"Images","description":"These related outputs share the same source format.","paths":paths}]});
        let accepted = call(
            &handler,
            "guide_replace_bucket",
            json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":"assets","operation_id":"write","expected_revision":0,"candidate":candidate}),
        );
        assert_eq!(accepted["status"], "accepted");
        let mut offset = 0;
        let mut reconstructed = String::new();
        loop {
            let page = call(
                &handler,
                "guide_read_bucket",
                json!({"bucket_id":"assets","offset":offset,"limit":500}),
            );

            assert_eq!(page["revision"], 1);
            assert!(page.to_string().len() < 128 * 1024);
            reconstructed.push_str(page["candidate_json_fragment"].as_str().unwrap());
            let Some(next) = page["next_offset"].as_u64() else {
                break;
            };
            assert!(next > offset);
            offset = next;
        }
        assert_eq!(
            serde_json::from_str::<Value>(&reconstructed).unwrap(),
            candidate
        );

        // Default reads keep ordinary candidates directly usable as objects.
        let small = json!({"label":"One image","description":"Read this image.","suggest_viewed":false,
            "items":[{"label":"Image","description":"Inspect the changed image.","paths":[paths[0]]}]});
        let accepted = call(
            &handler,
            "guide_replace_bucket",
            json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":"small","operation_id":"small","expected_revision":1,"candidate":small}),
        );
        assert_eq!(accepted["status"], "accepted");
        let page = call(&handler, "guide_read_bucket", json!({"bucket_id":"small"}));
        assert_eq!(page["candidate"], small);
        assert!(page["next_offset"].is_null());
    }

    /// Draft pagination preserves every stable bucket identity while bounding label previews.
    #[test]
    fn draft_summary_pages_are_bounded_and_recover_all_ids() {
        let paths = vec!["src/lib.rs".to_owned()];
        let handler = GuidedMcpHandler::new(GuidedReviewRun {
            run_id: "run",
            snapshot_id: "snapshot",
            changed_paths: &paths,
        });
        for revision in 0..100 {
            let accepted = call(
                &handler,
                "guide_replace_bucket",
                json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":format!("bucket-{revision}"),"operation_id":format!("write-{revision}"),"expected_revision":revision,
                "candidate":{"label":"界面".repeat(400),"description":"Read the behavior.","suggest_viewed":false,"items":[{"label":"Code","description":"Inspect the contract.","paths":paths}]}}),
            );
            assert_eq!(accepted["status"], "accepted");
        }
        let mut offset = 0;
        let mut identities = std::collections::HashSet::new();
        loop {
            let page = call(
                &handler,
                "guide_read_draft",
                json!({"offset":offset,"limit":256}),
            );

            assert!(page.to_string().len() < 128 * 1024);
            assert_eq!(page["revision"], 100);
            for bucket in page["buckets"].as_array().unwrap() {
                assert!(bucket["label"].as_str().unwrap().chars().count() <= 96);
                identities.insert(bucket["bucket_id"].as_str().unwrap().to_owned());
            }
            let Some(next) = page["next_offset"].as_u64() else {
                break;
            };
            assert!(next > offset);
            offset = next;
        }
        assert_eq!(identities.len(), 100);
    }

    /// Resource rejections stay replayable and cannot bypass an existing operation identity.
    #[test]
    fn oversized_retries_keep_the_operation_identity() {
        let paths = vec!["src/lib.rs".to_owned()];
        let handler = GuidedMcpHandler::new(GuidedReviewRun {
            run_id: "run",
            snapshot_id: "snapshot",
            changed_paths: &paths,
        });
        let mut arguments = json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":"code","expected_revision":0,"operation_id":"accepted",
            "candidate":{"label":"Code","description":"Read the changed behavior.","suggest_viewed":false,
                "items":[{"label":"Contract","description":"Inspect the contract.","paths":paths}]}});
        let accepted = call(&handler, "guide_replace_bucket", arguments.clone());
        assert_eq!(accepted["revision"], 1);
        let mut ordinary = arguments.clone();

        // Changing an existing operation's payload remains a conflict even when it is oversized.
        arguments["candidate"]["items"][0]["paths"][0] = json!("\u{1}".repeat(160 * 1024));
        let conflict = dispatch(
            &mut handler.lock().unwrap(),
            "guide_replace_bucket",
            arguments.clone(),
        )
        .unwrap();
        assert_eq!(conflict.value["issues"][0]["code"], "operation_conflict");

        // A new oversized operation records its rejection and replays that exact response.
        arguments["operation_id"] = json!("oversized");
        arguments["expected_revision"] = json!(1);
        let rejected = dispatch(
            &mut handler.lock().unwrap(),
            "guide_replace_bucket",
            arguments.clone(),
        )
        .unwrap();
        assert!(rejected.is_error);
        assert_eq!(rejected.value["issues"][0]["code"], "resource_limit");

        // Later accepted work and sealing cannot change the response to a rejected operation.
        ordinary["expected_revision"] = json!(1);
        ordinary["operation_id"] = json!("repair");
        assert_eq!(
            call(&handler, "guide_replace_bucket", ordinary)["revision"],
            2
        );
        let finished = call(
            &handler,
            "guide_finish",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":2,
                "operation_id":"finish","ordered_bucket_ids":["code"]}),
        );
        assert_eq!(finished["status"], "ready");
        let replayed = dispatch(
            &mut handler.lock().unwrap(),
            "guide_replace_bucket",
            arguments,
        )
        .unwrap();
        assert_eq!(rejected.value, replayed.value);
        assert_eq!(replayed.value["revision"], 1);
    }

    /// Every candidate text field and allocated selector slot is bounded before semantic validation.
    #[test]
    fn oversized_candidate_fields_and_path_arrays_preserve_the_accepted_bucket() {
        let paths = vec!["src/lib.rs".to_owned()];
        let handler = GuidedMcpHandler::new(GuidedReviewRun {
            run_id: "run",
            snapshot_id: "snapshot",
            changed_paths: &paths,
        });
        let mut arguments = json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":"code","expected_revision":0,"operation_id":"accepted",
            "candidate":{"label":"Code","description":"Read the changed behavior.","suggest_viewed":false,
                "items":[{"label":"Contract","description":"Inspect the contract.","paths":paths}]}});
        call(&handler, "guide_replace_bucket", arguments.clone());
        let accepted = arguments["candidate"].clone();
        arguments["expected_revision"] = json!(1);

        // Escaped text is small in memory but over budget in each position on the wire.
        for (index, pointer) in [
            "/label",
            "/description",
            "/items/0/label",
            "/items/0/description",
            "/items/0/paths/0",
        ]
        .iter()
        .enumerate()
        {
            arguments["candidate"] = accepted.clone();
            *arguments["candidate"].pointer_mut(pointer).unwrap() =
                json!("\u{1}".repeat(160 * 1024));
            arguments["operation_id"] = json!(format!("escaped-{index}"));
            let rejected = dispatch(
                &mut handler.lock().unwrap(),
                "guide_replace_bucket",
                arguments.clone(),
            )
            .unwrap();
            assert!(rejected.is_error);
            assert_eq!(rejected.value["issues"][0]["code"], "resource_limit");
            assert_eq!(rejected.value["revision"], 1);
        }

        // Empty selectors still occupy String slots, even though their wire encoding is tiny.
        arguments["candidate"] = accepted.clone();
        arguments["candidate"]["items"][0]["paths"] = json!(vec![""; 50_000]);
        arguments["operation_id"] = json!("selector-slots");
        let rejected = dispatch(
            &mut handler.lock().unwrap(),
            "guide_replace_bucket",
            arguments,
        )
        .unwrap();
        assert_eq!(rejected.value["issues"][0]["code"], "resource_limit");
        assert_eq!(rejected.value["revision"], 1);
        assert_eq!(
            call(&handler, "guide_read_bucket", json!({"bucket_id":"code"}))["candidate"],
            accepted
        );
    }

    /// Exactly 900 KiB is accepted; one more encoded byte leaves the prior bucket intact.
    #[test]
    fn candidate_wire_limit_is_inclusive() {
        const MAX_CANDIDATE_BYTES: usize = 900 * 1024;
        let paths = vec!["src/lib.rs".to_owned()];
        let handler = GuidedMcpHandler::new(GuidedReviewRun {
            run_id: "run",
            snapshot_id: "snapshot",
            changed_paths: &paths,
        });
        let mut arguments = json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":"code","expected_revision":0,"operation_id":"limit",
            "candidate":{"label":"Code","description":"","suggest_viewed":false,
                "items":[{"label":"Contract","description":"Inspect the contract.","paths":paths}]}});
        let padding = MAX_CANDIDATE_BYTES - arguments["candidate"].to_string().len();
        arguments["candidate"]["description"] = json!("x".repeat(padding));
        assert_eq!(
            arguments["candidate"].to_string().len(),
            MAX_CANDIDATE_BYTES
        );
        assert_eq!(
            call(&handler, "guide_replace_bucket", arguments.clone())["revision"],
            1
        );

        // The replacement has one extra encoded byte and must remain atomic.
        arguments["candidate"]["description"] = json!("x".repeat(padding + 1));
        arguments["expected_revision"] = json!(1);
        arguments["operation_id"] = json!("over-limit");
        let rejected = dispatch(
            &mut handler.lock().unwrap(),
            "guide_replace_bucket",
            arguments,
        )
        .unwrap();
        assert_eq!(rejected.value["issues"][0]["code"], "resource_limit");
        let state = handler.lock().unwrap();
        assert_eq!(state.draft.draft().revision, 1);
        assert_eq!(
            state.draft.bucket("code").unwrap().description.len(),
            padding
        );
    }

    /// Private wire shapes preserve required fields and unknown-field rejection at both levels.
    #[test]
    fn candidate_wire_shapes_reject_missing_and_unknown_fields() {
        let paths = vec!["src/lib.rs".to_owned()];
        let handler = GuidedMcpHandler::new(GuidedReviewRun {
            run_id: "run",
            snapshot_id: "snapshot",
            changed_paths: &paths,
        });
        let original = json!({"run_id":"run","snapshot_id":"snapshot","bucket_id":"code","expected_revision":0,"operation_id":"write",
            "candidate":{"label":"Code","description":"Read the changed behavior.","suggest_viewed":false,
                "items":[{"label":"Contract","description":"Inspect the contract.","paths":paths}]}});
        for pointer in ["/candidate", "/candidate/items/0"] {
            let mut unknown = original.clone();
            unknown.pointer_mut(pointer).unwrap()["unexpected"] = json!(true);
            assert!(
                dispatch(
                    &mut handler.lock().unwrap(),
                    "guide_replace_bucket",
                    unknown
                )
                .is_err()
            );
            let mut missing = original.clone();
            missing
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove("description");
            assert!(
                dispatch(
                    &mut handler.lock().unwrap(),
                    "guide_replace_bucket",
                    missing
                )
                .is_err()
            );
        }
        assert_eq!(handler.lock().unwrap().draft.draft().revision, 0);
        assert_eq!(
            call(&handler, "guide_replace_bucket", original)["revision"],
            1
        );
    }
}
