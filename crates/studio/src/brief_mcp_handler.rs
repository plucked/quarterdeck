use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

use qd_agent_mcp::{McpHandler, McpToolDescriptor, McpToolResult};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::brief_draft::{BriefDraft, BriefDraftResponse, FinishedBrief};
use crate::brief_mcp_guide::BRIEF_TOOL_NAMES;
use crate::guided_review::DraftStatus;
use crate::mcp_json_page::{MAX_PAGE_CHARACTERS, mcp_json_page};
use crate::run_mcp_handler::RunMcpHandler;
use crate::{AgentProgress, BriefRun, CodeSide, GuidedToolAudit};

const MAX_AUDIT_ENTRIES: usize = 512;
const MAX_PROGRESS_ENTRIES: usize = 64;
const MAX_SUMMARY_PAGE_BYTES: usize = 48 * 1024;
const MAX_LIST_ENTRIES: usize = 256;
const DEFAULT_LIST_ENTRIES: usize = 32;
const MAX_TITLE_PREVIEW: usize = 96;
const MAX_LEAD_PREVIEW: usize = 256;
const STUDIO_SCHEMA: &str = include_str!("studio_schema.json");

/// Serializes private Brief authoring and revocation for one immutable run.
pub(crate) struct BriefMcpHandler {
    state: Mutex<BriefHandlerState>,
}

/// Mutable authoring data, bounded diagnostics and cancellation share one lock.
struct BriefHandlerState {
    draft: BriefDraft,
    closed: bool,
    audit: VecDeque<GuidedToolAudit>,
    progress: VecDeque<AgentProgress>,
}

/// A readable tool outcome whose named fields distinguish validation from transport failure.
struct BriefToolResponse {
    is_error: bool,
    value: Value,
}

impl BriefToolResponse {
    /// Wraps a bounded read without inventing a domain mutation status.
    fn read(value: Value) -> Self {
        Self {
            is_error: false,
            value,
        }
    }
}

/// Replacing the lead carries the same revision and retry identity as pages.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeadArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    operation_id: String,
    lead: String,
}

/// A full page candidate is validated by the shared schema and anchor domain.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    operation_id: String,
    page_id: String,
    candidate: Value,
}

/// Removing a page requires current revision and an idempotent operation key.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoveArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    operation_id: String,
    page_id: String,
}

/// Validation supplies the complete final order without sealing the draft.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    ordered_page_ids: Vec<String>,
}

/// Finish freezes one valid order and returns its replayable validation receipt.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishArguments {
    run_id: String,
    snapshot_id: String,
    expected_revision: u64,
    operation_id: String,
    ordered_page_ids: Vec<String>,
}

impl BriefMcpHandler {
    /// Captures exactly the anchor targets and identity supplied by the host.
    pub(crate) fn new(run: BriefRun<'_>) -> Self {
        Self {
            state: Mutex::new(BriefHandlerState {
                draft: BriefDraft::new(run.run_id.into(), run.snapshot_id.into(), run.targets),
                closed: false,
                audit: VecDeque::new(),
                progress: VecDeque::new(),
            }),
        }
    }

    /// A poisoned draft is unavailable, because partially changed state is unsafe to publish.
    fn lock(&self) -> Result<MutexGuard<'_, BriefHandlerState>, String> {
        self.state
            .lock()
            .map_err(|_| "The brief authoring state is unavailable.".into())
    }

    /// Returns bounded typed diagnostics for durable publication and session reports.
    pub(crate) fn audit(&self) -> Vec<GuidedToolAudit> {
        self.lock()
            .map(|state| state.audit.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Resolves a successful terminal receipt to the exact sealed studio and artifact.
    pub(crate) fn finished_studio(&self, receipt: &str) -> Result<FinishedBrief, String> {
        self.lock()?.draft.finished_studio(receipt)
    }
}

impl RunMcpHandler for BriefMcpHandler {
    /// Closes authoring under the same lock that accepts revisions.
    fn close(&self) {
        if let Ok(mut state) = self.lock() {
            state.closed = true;
        }
    }

    /// Returns authoritative domain progress independently of the model transcript.
    fn drain_progress(&self) -> Vec<AgentProgress> {
        self.lock()
            .map(|mut state| state.progress.drain(..).collect())
            .unwrap_or_default()
    }
}

impl McpHandler for BriefMcpHandler {
    /// Advertises only the fixed, explicitly authorized Brief authoring catalog.
    fn tools(&self) -> Vec<McpToolDescriptor> {
        BRIEF_TOOL_NAMES
            .iter()
            .map(|name| descriptor(name))
            .collect()
    }

    /// Turns argument and domain failures into actionable text plus structured data.
    fn call_tool(&self, name: &str, arguments: Value) -> McpToolResult {
        // Every operation shares the state lock with cancellation and finishing.
        let result = self.lock().and_then(|mut state| {
            if state.closed {
                return Err("This run is closed; no further authoring is allowed.".into());
            }
            dispatch(&mut state, name, arguments)
        });

        // Claude may substitute structured content for text, so both include guidance.
        match result {
            Ok(response) => {
                let text = response.value.to_string();
                if response.is_error {
                    McpToolResult::failure(text, response.value)
                } else {
                    McpToolResult::success(text, response.value)
                }
            }
            Err(message) => {
                let value = json!({"status":"rejected","issues":[{"code":"invalid_request","message":message}],
                    "repair":"Read brief_context and brief_read_draft, then resend the documented arguments. Rejected calls do not change the draft."});
                McpToolResult::failure(value.to_string(), value)
            }
        }
    }
}

/// Routes bounded reads and typed revision commands into the private draft.
fn dispatch(
    state: &mut BriefHandlerState,
    name: &str,
    arguments: Value,
) -> Result<BriefToolResponse, String> {
    // Reads expose only this run's frozen anchors and accepted draft.
    if let Some(response) = read_request(&state.draft, name, &arguments)? {
        return Ok(BriefToolResponse::read(response));
    }

    // Validate run identity before the domain can change or seal any accepted content.
    let response = match name {
        "brief_set_lead" => {
            let arguments: LeadArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(&state.draft, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.set_lead(
                arguments.expected_revision,
                &arguments.operation_id,
                arguments.lead,
            )
        }
        "brief_replace_page" => {
            let arguments: PageArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(&state.draft, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.replace_page(
                &arguments.page_id,
                arguments.expected_revision,
                &arguments.operation_id,
                arguments.candidate,
            )
        }
        "brief_remove_page" => {
            let arguments: RemoveArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(&state.draft, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.remove_page(
                &arguments.page_id,
                arguments.expected_revision,
                &arguments.operation_id,
            )
        }
        "brief_validate" => {
            let arguments: ValidateArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(&state.draft, &arguments.run_id, &arguments.snapshot_id)?;
            state
                .draft
                .validate_brief(arguments.expected_revision, &arguments.ordered_page_ids)
        }
        "brief_finish" => {
            let arguments: FinishArguments =
                serde_json::from_value(arguments).map_err(|error| error.to_string())?;
            check_scope(&state.draft, &arguments.run_id, &arguments.snapshot_id)?;
            state.draft.finish_brief(
                arguments.expected_revision,
                &arguments.ordered_page_ids,
                &arguments.operation_id,
            )
        }
        _ => return Err("Unknown Brief tool.".into()),
    };
    let value = record_response(state, name, &response);
    Ok(BriefToolResponse {
        is_error: response.is_error(),
        value,
    })
}

/// Handles paginated recovery and frozen anchor discovery without mutation.
fn read_request(
    draft: &BriefDraft,
    name: &str,
    arguments: &Value,
) -> Result<Option<Value>, String> {
    match name {
        "brief_context" => {
            require_keys(arguments, &[], &[])?;
            Ok(Some(draft_json(draft, 0, DEFAULT_LIST_ENTRIES)))
        }
        "brief_read_draft" => {
            require_keys(arguments, &[], &["offset", "limit"])?;
            Ok(Some(draft_json(
                draft,
                optional_usize(arguments, "offset", 0)?,
                optional_usize(arguments, "limit", DEFAULT_LIST_ENTRIES)?,
            )))
        }
        "brief_anchor_paths" => {
            require_keys(arguments, &["offset", "limit"], &[])?;
            let page = draft.anchor_paths(
                usize_argument(arguments, "offset")?,
                usize_argument(arguments, "limit")?,
            );
            Ok(Some(
                json!({"paths":page.paths,"total":page.total,"next_offset":page.next_offset}),
            ))
        }
        "brief_anchor_ranges" => {
            require_keys(arguments, &["path", "side"], &["offset", "limit"])?;
            let path = string_argument(arguments, "path")?;
            let side = code_side(string_argument(arguments, "side")?)?;
            let ranges = draft
                .anchor_ranges(path, side)
                .ok_or("Unknown anchor path; read brief_anchor_paths.")?;
            let offset = optional_usize(arguments, "offset", 0)?;
            let limit =
                optional_usize(arguments, "limit", MAX_LIST_ENTRIES)?.clamp(1, MAX_LIST_ENTRIES);
            let selected: Vec<_> = ranges
                .iter()
                .skip(offset)
                .take(limit)
                .map(|range| json!({"start_line":range.start(),"end_line":range.end()}))
                .collect();
            let next = offset.saturating_add(selected.len());
            Ok(Some(
                json!({"path":path,"side":side.as_str(),"ranges":selected,"range_count":ranges.len(),"offset":offset,"next_offset":(next<ranges.len()).then_some(next),"maximum_anchor_span":80}),
            ))
        }
        "brief_read_page" => {
            require_keys(arguments, &["page_id"], &["offset", "limit"])?;
            let page_id = string_argument(arguments, "page_id")?;
            let candidate = draft
                .page(page_id)
                .ok_or("Unknown page_id; read brief_read_draft.")?;
            Ok(Some(mcp_json_page(
                candidate,
                json!({"page_id":page_id,"revision":draft.draft().revision}),
                optional_usize(arguments, "offset", 0)?,
                optional_usize(arguments, "limit", MAX_PAGE_CHARACTERS)?,
            )))
        }
        "brief_read_lead" => {
            require_keys(arguments, &[], &["offset", "limit"])?;
            let view = draft.draft();
            Ok(Some(mcp_json_page(
                &json!({"lead":view.lead}),
                json!({"revision":view.revision}),
                optional_usize(arguments, "offset", 0)?,
                optional_usize(arguments, "limit", MAX_PAGE_CHARACTERS)?,
            )))
        }
        _ => Ok(None),
    }
}

/// Stores a bounded validation history and native progress under the mutation lock.
fn record_response(
    state: &mut BriefHandlerState,
    name: &str,
    response: &BriefDraftResponse,
) -> Value {
    state.audit.push_back(GuidedToolAudit {
        tool_name: name.into(),
        status: status_text(response.status).into(),
        revision: response.revision,
        issue_codes: response
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect(),
    });
    if state.audit.len() > MAX_AUDIT_ENTRIES {
        state.audit.pop_front();
    }
    state.progress.push_back(AgentProgress::ReviewingBrief {
        pages: state.draft.draft().pages.len(),
        issues: response.issue_count,
    });
    if state.progress.len() > MAX_PROGRESS_ENTRIES {
        state.progress.pop_front();
    }

    // Anchor diagnostics preserve the page/widget location and bounded valid ranges.
    json!({"status":status_text(response.status),"revision":response.revision,"issue_count":response.issue_count,
        "issues":response.issues.iter().map(|issue| json!({"code":issue.code,"page_id":issue.page_id,"widget_index":issue.widget_index,
            "location":issue.location,"message":issue.message,"path":issue.path,"side":issue.side.map(CodeSide::as_str),
            "allowed_ranges":issue.allowed_ranges.iter().map(|range| json!({"start_line":range.start(),"end_line":range.end()})).collect::<Vec<_>>(),
            "range_count":issue.range_count})).collect::<Vec<_>>(),
        "validation_receipt":response.receipt,"publication_status":"pending",
        "next_step":"Repair the complete rejected page using exact anchor ranges and schema diagnostics. Newly accepted commands return the new revision; after a retry read brief_read_draft for the latest revision. Validate complete ordering, finish, and echo only its validation_receipt in the final structured answer."})
}

/// Pages summaries while keeping large accepted strings available through full recovery.
fn draft_json(draft: &BriefDraft, offset: usize, limit: usize) -> Value {
    let view = draft.draft();
    let mut pages = Vec::new();
    let mut bytes = 0;
    for page in view
        .pages
        .iter()
        .skip(offset)
        .take(limit.clamp(1, MAX_LIST_ENTRIES))
    {
        let title: String = page.title.chars().take(MAX_TITLE_PREVIEW).collect();
        let summary =
            json!({"page_id":page.page_id,"title":title,"widget_count":page.widget_count});
        let encoded = summary.to_string().len();
        if bytes + encoded > MAX_SUMMARY_PAGE_BYTES {
            break;
        }
        bytes += encoded;
        pages.push(summary);
    }
    let next = offset.saturating_add(pages.len());
    let anchor_paths = draft.anchor_paths(0, DEFAULT_LIST_ENTRIES);
    let lead: String = view.lead.chars().take(MAX_LEAD_PREVIEW).collect();
    json!({"run_id":view.run_id,"snapshot_id":view.snapshot_id,"revision":view.revision,"sealed":view.sealed,
        "lead_preview":lead,"lead_truncated":view.lead.chars().count()>MAX_LEAD_PREVIEW,
        "pages":pages,"page_count":view.pages.len(),"offset":offset,"next_offset":(next<view.pages.len()).then_some(next),
        "anchor_paths":{"paths":anchor_paths.paths,"total":anchor_paths.total,"next_offset":anchor_paths.next_offset},
        "next_step":"Use these exact run/snapshot identities and the current expected_revision. Titles and lead are abbreviated previews; recover full accepted content with brief_read_page and brief_read_lead, following next_offset at one revision. Discover exact citable ranges with brief_anchor_ranges. Each accepted edit needs a fresh operation_id; retries reuse the same key and payload."})
}

/// Refuses another run or revision snapshot before invoking a domain operation.
fn check_scope(draft: &BriefDraft, run_id: &str, snapshot_id: &str) -> Result<(), String> {
    let view = draft.draft();
    if view.run_id != run_id || view.snapshot_id != snapshot_id {
        return Err(
            "run_id or snapshot_id differs from this frozen review; read brief_context.".into(),
        );
    }
    Ok(())
}

/// Rejects unknown or missing keys instead of silently accepting misspelled reads.
fn require_keys(arguments: &Value, required: &[&str], optional: &[&str]) -> Result<(), String> {
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

/// Reads an explicit string without coercing malformed protocol input.
fn string_argument<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    arguments[key]
        .as_str()
        .ok_or_else(|| format!("{key} must be a string."))
}

/// Converts nonnegative JSON counts without truncation or architecture-dependent wrapping.
fn usize_argument(arguments: &Value, key: &str) -> Result<usize, String> {
    arguments[key]
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .ok_or_else(|| format!("{key} must be a nonnegative integer."))
}

/// Supplies a default only when the optional field is absent, never when malformed.
fn optional_usize(arguments: &Value, key: &str, default: usize) -> Result<usize, String> {
    if arguments.get(key).is_none() {
        return Ok(default);
    }
    usize_argument(arguments, key)
}

/// Parses the exact side names used by the existing Studio anchor schema.
fn code_side(side: &str) -> Result<CodeSide, String> {
    match side {
        "base" => Ok(CodeSide::Base),
        "head" => Ok(CodeSide::Head),
        _ => Err("side must be base or head.".into()),
    }
}

/// Uses the same stable status names as guided-review and persisted audit rows.
fn status_text(status: DraftStatus) -> &'static str {
    match status {
        DraftStatus::Accepted => "accepted",
        DraftStatus::Rejected => "rejected",
        DraftStatus::Valid => "valid",
        DraftStatus::Ready => "ready",
    }
}

/// Advertises the existing full page schema, including root definitions for references.
fn descriptor(name: &str) -> McpToolDescriptor {
    let text = json!({"type":"string","minLength":1});
    let integer = json!({"type":"integer","minimum":0});
    let mut properties = serde_json::Map::new();
    let description = match name {
        "brief_context" => {
            "Read the frozen run identity, draft revision and initial citable path inventory."
        }
        "brief_read_draft" => {
            properties.insert("offset".into(), integer.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_LIST_ENTRIES}),
            );
            "Read bounded accepted page summaries and abbreviated lead. Follow next_offset; recover full content using brief_read_page and brief_read_lead."
        }
        "brief_anchor_paths" => {
            properties.insert("offset".into(), integer.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_LIST_ENTRIES}),
            );
            "Read a page of exact citable file paths; follow next_offset until null."
        }
        "brief_anchor_ranges" => {
            properties.insert("path".into(), text.clone());
            properties.insert(
                "side".into(),
                json!({"type":"string","enum":["base","head"]}),
            );
            properties.insert("offset".into(), integer.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_LIST_ENTRIES}),
            );
            "Read exact visible line ranges on one file side. Both anchor endpoints must lie in these ranges; maximum span is 80 lines."
        }
        "brief_read_page" | "brief_read_lead" => {
            if name == "brief_read_page" {
                properties.insert("page_id".into(), text.clone());
            }
            properties.insert("offset".into(), integer.clone());
            properties.insert(
                "limit".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_PAGE_CHARACTERS}),
            );
            "Recover accepted candidate JSON (lead as an object with lead). Optional offset/limit count Unicode scalars. Follow next_offset at one revision; concatenate JSON fragments when paged."
        }
        _ => mutation_schema(name, &mut properties, &text, integer),
    };
    let required: Vec<_> = properties
        .keys()
        .filter(|key| {
            !matches!(
                name,
                "brief_read_draft" | "brief_anchor_ranges" | "brief_read_page" | "brief_read_lead"
            ) || !matches!(key.as_str(), "offset" | "limit")
        })
        .cloned()
        .collect();
    let mut input_schema = json!({"type":"object","additionalProperties":false,"required":required,"properties":properties});
    if name == "brief_replace_page" {
        let schema: Value =
            serde_json::from_str(STUDIO_SCHEMA).expect("compiled Studio schema is valid");
        input_schema["$defs"] = schema["$defs"].clone();
    }
    McpToolDescriptor {
        name: name.into(),
        description: description.into(),
        input_schema,
        output_schema: Some(json!({"type":"object"})),
        read_only: !matches!(
            name,
            "brief_set_lead" | "brief_replace_page" | "brief_remove_page" | "brief_finish"
        ),
    }
}

/// Defines the shared revision envelope and each complete mutation payload.
fn mutation_schema(
    name: &str,
    properties: &mut serde_json::Map<String, Value>,
    text: &Value,
    integer: Value,
) -> &'static str {
    properties.insert("run_id".into(), text.clone());
    properties.insert("snapshot_id".into(), text.clone());
    properties.insert("expected_revision".into(), integer);
    if name != "brief_validate" {
        properties.insert("operation_id".into(), text.clone());
    }
    match name {
        "brief_set_lead" => {
            properties.insert("lead".into(), text.clone());
        }
        "brief_replace_page" | "brief_remove_page" => {
            properties.insert("page_id".into(), text.clone());
        }
        _ => {
            properties.insert(
                "ordered_page_ids".into(),
                json!({"type":"array","items":text,"minItems":1}),
            );
        }
    }
    if name == "brief_replace_page" {
        let schema: Value =
            serde_json::from_str(STUDIO_SCHEMA).expect("compiled Studio schema is valid");
        properties.insert(
            "candidate".into(),
            schema["properties"]["pages"]["items"].clone(),
        );
    }
    match name {
        "brief_set_lead" => "Atomically replace the lead at the expected revision.",
        "brief_replace_page" => {
            "Validate and replace one whole page. Any invalid schema, widget or anchor rejects the entire page and leaves accepted content unchanged. Repair using exact diagnostics."
        }
        "brief_remove_page" => "Remove one accepted page at the expected revision.",
        "brief_validate" => {
            "Validate the complete lead, exact page ordering, every widget and every anchor without sealing."
        }
        _ => {
            "Validate and seal the complete Brief. Echo its validation_receipt in the final structured answer; durable publication remains a host operation."
        }
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::{DiffOptions, FileDiff};

    use super::*;
    use crate::AnchorTargets;

    /// A synthetic renamed file makes discovery and anchor repair observable.
    fn handler() -> BriefMcpHandler {
        let mut targets = AnchorTargets::default();
        targets.add_file(
            "src/lib.rs",
            Some("src/old.rs"),
            &FileDiff::build(
                "@@ -1,2 +1,2 @@\n old\n-before\n+after",
                DiffOptions::default(),
            ),
        );
        BriefMcpHandler::new(BriefRun {
            run_id: "run",
            snapshot_id: "snapshot",
            targets,
        })
    }

    /// Dispatches through argument validation and the same serialized adapter path.
    fn call(handler: &BriefMcpHandler, name: &str, arguments: Value) -> BriefToolResponse {
        dispatch(&mut handler.lock().unwrap(), name, arguments).unwrap()
    }

    /// The adapter preserves precise anchor repairs and seals exactly accepted page content.
    #[test]
    fn anchor_repair_and_finish_preserve_the_complete_artifact() {
        let handler = handler();
        let candidate = json!({"title":"Contract","summary":"Check ownership.","widgets":[
            {"kind":"code_excerpt","caption":"The change","anchor":{"path":"src/lib.rs","side":"head","start_line":99,"end_line":99}}
        ]});
        let arguments = json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":0,"operation_id":"invalid","page_id":"contract","candidate":candidate});
        let rejected = call(&handler, "brief_replace_page", arguments);
        assert!(rejected.is_error, "{}", rejected.value);
        assert_eq!(rejected.value["revision"], 0);
        assert_eq!(rejected.value["issues"][0]["page_id"], "contract");
        assert_ne!(
            rejected.value["issues"][0]["allowed_ranges"]
                .as_array()
                .unwrap()
                .len(),
            0
        );

        // Repair the whole candidate, then add the lead and validate the complete order.
        let mut repaired = candidate;
        repaired["widgets"][0]["anchor"]["start_line"] = json!(2);
        repaired["widgets"][0]["anchor"]["end_line"] = json!(2);
        let accepted = call(
            &handler,
            "brief_replace_page",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":0,"operation_id":"repair","page_id":"contract","candidate":repaired}),
        );
        assert!(!accepted.is_error, "{}", accepted.value);
        let lead = call(
            &handler,
            "brief_set_lead",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":1,"operation_id":"lead","lead":"Ownership changes explicitly."}),
        );
        assert!(!lead.is_error, "{}", lead.value);
        let valid = call(
            &handler,
            "brief_validate",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":2,"ordered_page_ids":["contract"]}),
        );
        assert!(!valid.is_error, "{}", valid.value);
        let finish = call(
            &handler,
            "brief_finish",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":2,"operation_id":"finish","ordered_page_ids":["contract"]}),
        );
        let receipt = finish.value["validation_receipt"].as_str().unwrap();
        let finished = handler.finished_studio(receipt).unwrap();
        assert_eq!(finished.artifact["pages"][0], repaired);
        assert_eq!(finished.studio.pages.len(), 1);
        assert_eq!(handler.audit().len(), 5);
        assert_ne!(handler.drain_progress().len(), 0);
    }

    /// Large Unicode pages and lead previews remain recoverable with bounded responses.
    #[test]
    fn accepted_content_recovery_is_lossless_and_schema_references_resolve() {
        let handler = handler();
        let text = "界面".repeat(10_000);
        let candidate = json!({"title":"Unicode","summary":"Read the explanation.","widgets":[{"kind":"section","heading":"Behavior","body":text}]});
        let accepted = call(
            &handler,
            "brief_replace_page",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":0,"operation_id":"page","page_id":"unicode","candidate":candidate}),
        );
        assert!(!accepted.is_error, "{}", accepted.value);
        let mut offset = 0;
        let mut reconstructed = String::new();
        loop {
            let page = call(
                &handler,
                "brief_read_page",
                json!({"page_id":"unicode","offset":offset,"limit":500}),
            )
            .value;
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

        // The nested page schema's local references resolve from the tool schema root.
        let tools = handler.tools();
        let replacement = tools
            .iter()
            .find(|tool| tool.name == "brief_replace_page")
            .unwrap();
        assert!(replacement.input_schema["$defs"]["code_excerpt"].is_object());
        let lead = call(
            &handler,
            "brief_set_lead",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":1,"operation_id":"lead","lead":text}),
        );
        assert!(!lead.is_error, "{}", lead.value);
        let preview = call(&handler, "brief_context", json!({})).value;
        assert_eq!(preview["lead_truncated"], true);
        assert!(preview["lead_preview"].as_str().unwrap().chars().count() <= MAX_LEAD_PREVIEW);
    }

    /// Operation identity remains authoritative even when the retried payload grows too large.
    #[test]
    fn oversized_retries_preserve_domain_replay_semantics() {
        let handler = handler();
        let accepted = call(
            &handler,
            "brief_set_lead",
            json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":0,"operation_id":"lead","lead":"Initial lead."}),
        );
        assert!(!accepted.is_error);
        let mut oversized = json!({"run_id":"run","snapshot_id":"snapshot","expected_revision":0,"operation_id":"lead","lead":"x".repeat(901*1024)});
        let conflict = call(&handler, "brief_set_lead", oversized.clone());
        assert_eq!(conflict.value["issues"][0]["code"], "operation_conflict");
        oversized["operation_id"] = json!("oversized");
        oversized["expected_revision"] = json!(1);
        let rejected = call(&handler, "brief_set_lead", oversized.clone());
        assert_eq!(rejected.value["issues"][0]["code"], "resource_limit");
        let replayed = call(&handler, "brief_set_lead", oversized);
        assert_eq!(rejected.value, replayed.value);
        assert_eq!(replayed.value["revision"], 1);
    }
}
