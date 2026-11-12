//! The review studio: what an AI agent may write to explain a pull request
//! (a lead and pages of widgets in reading order), the prompt and schema it gets, a
//! headless Claude Code run that writes it, and the checks that keep only
//! widgets whose code anchors exist in the diff. UI-free and blocking; the
//! app runs it on the background executor and draws the widgets.
//!
//! Files:
//! - `studio.rs`: `Studio`, the lead and the pages.
//! - `studio_page.rs`: `StudioPage`, one step of the walk: title, summary, widgets.
//! - `studio_widget.rs`: `StudioWidget`, one block: section, callout, code, table, scope, reading order, graph or sequence.
//! - `callout_tone.rs`: `CalloutTone`, note, constraint, verdict or evidence.
//! - `studio_table.rs`: `StudioTable`, a compact table of Markdown cells.
//! - `table_role.rs`: `TableRole`, a mapping or a numbered trace.
//! - `change_scope.rs`: `ChangeScope`, what changes, what stays the same, what is mechanical.
//! - `studio_tour.rs`: `StudioTour`, the order to read the change in.
//! - `tour_step.rs`, `step_weight.rs`: `TourStep` (one piece, linked to its lines) and `StepWeight` (core, supporting, mechanical).
//! - `studio_graph.rs`: `StudioGraph`, boxes in boundaries joined by labelled arrows.
//! - `graph_node.rs`, `graph_edge.rs`: `GraphNode` (a box) and `GraphEdge` (an arrow).
//! - `node_role.rs`: `NodeRole`, what a box or lane stands for (actor, state owner, transport, store, constraint, external).
//! - `edge_style.rs`: `EdgeStyle`, control, data, async or optional.
//! - `diagram_change.rs`: `DiagramChange`, a part that stayed, was added or was removed.
//! - `studio_sequence.rs`: `StudioSequence`, lanes and messages in time order.
//! - `sequence_lane.rs`, `sequence_message.rs`: `SequenceLane` and `SequenceMessage`.
//! - `message_style.rs`: `MessageStyle`, call, reply or async.
//! - `code_anchor.rs`: `CodeAnchor`, lines of one file on one side that a widget points at.
//! - `code_side.rs`: `CodeSide`, base or head.
//! - `anchor_targets.rs`: `AnchorTargets`, the lines a diff shows, which anchors are checked against.
//! - `checked_studio.rs`: `CheckedStudio`, a studio and the widgets left out of it.
//! - `rejected_widget.rs`: `RejectedWidget`, a widget left out and why.
//! - `parse_studio.rs`: reads an agent's structured output into a checked studio.
//! - `wire_graph.rs`: `WireGraph`, a graph as the agent writes it, and its checks (ids to indexes, sizes, labels, anchors, arrows only where both boxes are).
//! - `wire_sequence.rs`: `WireSequence`, a sequence as the agent writes it, and its checks.
//! - `wire_role.rs`: `WireRole`, a box's or lane's role as the agent writes it.
//! - `diagram_checks.rs`: checks both diagrams share: ids to indexes, labels written.
//! - `wire_anchor.rs`: an anchor as the agent writes it, checked against the diff.
//! - `studio_schema.rs` + `studio_schema.json`: the JSON Schema of that output.
//! - `studio_guide.rs` + `studio_guide.md`: the instructions appended to the agent's system prompt.
//! - `studio_request.rs`: `StudioRequest`, the pull request and files a studio is about.
//! - `code_access.rs`: `CodeAccess`, the diffs only or a checked-out workspace.
//! - `reader_familiarity.rs`: `ReaderFamiliarity`, how well the reader knows the area (sets the onboarding).
//! - `request_layer.rs`: `RequestLayer`, a stack layer a combined brief spans.
//! - `studio_request_file.rs`: `StudioRequestFile`, one changed file of it.
//! - `file_listing.rs`: `FileListing`, a file's diff or why it is left out.
//! - `studio_prompt.rs`: the agent's user message, with numbered diffs.
//! - `studio_prompt_message.rs`: `StudioPrompt`, that message and the lines it lists.
//! - `workspace_diff.rs`: `WorkspaceDiff`, a changed file's numbered diff written to the workspace's `diffs/`.
//! - `workspace_source.rs`: `WorkspaceSource`, the repository and head commit a review workspace checks out.
//! - `prepare_workspace.rs`: `prepare_workspace`, a shallow fetch of the head commit into `head/`, with the pull request's diff as `change.diff`.
//! - `review_workspace.rs`: `ReviewWorkspace`, that folder, removed when dropped.
//! - `workspace_progress.rs`: `WorkspaceProgress`, fetching or unpacking.
//! - `workspace_error.rs`: `WorkspaceError`, why a workspace could not be prepared.
//! - `workspace_root.rs`: `workspace_root`, the cache folder workspaces go in.
//! - `clean_orphan_workspaces.rs`: removes workspaces left by an app process that is gone.
//! - `brief_record.rs`: `BriefRecord`, everything about how one brief was made.
//! - `write_brief_report.rs`: `write_brief_report` and `report_root`, a folder reporting a brief for later tuning.
//! - `utc_stamp.rs`: `utc_stamp`, a time as a UTC file-name stamp and readable text, for a report's folder.
//! - `shown_page.rs`: `ShownPage`, the page of a brief a reader had open when they reported it.
//! - `tool_call.rs`: `ToolCall`, a tool call of a run as readable text, read from its transcript.
//! - `find_claude.rs`: finds the `claude` executable.
//! - `smart_filter.rs`, `smart_filter_guide.rs`: ordered review buckets, validation and the agent contract.
//! - `run_claude.rs`: runs Claude Code headless, confined, and follows its event stream.
//! - `text_tail.rs`: `tail`, the end of a tool's error output for an error message.
//! - `output_lines.rs`: `OutputLines`, a child process's output as lines read on a thread, so waits notice a cancel.
//! - `stop_child.rs`: `stop_child`, `SIGTERM`, a grace period, then `SIGKILL`.
//! - `new_process_folder.rs`: `new_process_folder`, a folder named after this process and the time.
//! - `claude_event.rs`: one line of that stream.
//! - `agent_progress.rs`: `AgentProgress`, how far a run has come.
//! - `agent_answer.rs`: `AgentAnswer`, a finished run's output, cost and session.
//! - `agent_run_error.rs`: `AgentRunError`, why a run produced nothing.

mod agent_answer;
mod agent_progress;
mod agent_run_error;
mod anchor_targets;
mod brief_record;
mod callout_tone;
mod change_scope;
mod checked_studio;
mod claude_event;
mod clean_orphan_workspaces;
mod code_access;
mod code_anchor;
mod code_side;
mod diagram_change;
mod diagram_checks;
mod edge_style;
mod file_listing;
mod find_claude;
mod graph_edge;
mod graph_node;
mod message_style;
mod new_process_folder;
mod node_role;
mod output_lines;
mod parse_studio;
mod prepare_workspace;
mod reader_familiarity;
mod rejected_widget;
mod request_layer;
mod review_workspace;
mod run_claude;
mod sequence_lane;
mod sequence_message;
mod shown_page;
mod smart_filter;
mod smart_filter_guide;
mod step_weight;
mod stop_child;
mod studio;
mod studio_graph;
mod studio_guide;
mod studio_page;
mod studio_prompt;
mod studio_prompt_message;
mod studio_request;
mod studio_request_file;
mod studio_schema;
mod studio_sequence;
mod studio_table;
mod studio_tour;
mod studio_widget;
mod table_role;
mod text_tail;
mod tool_call;
mod tour_step;
mod utc_stamp;
mod wire_anchor;
mod wire_graph;
mod wire_role;
mod wire_sequence;
mod workspace_diff;
mod workspace_error;
mod workspace_progress;
mod workspace_root;
mod workspace_source;
mod write_brief_report;

pub use agent_answer::AgentAnswer;
pub use agent_progress::AgentProgress;
pub use agent_run_error::AgentRunError;
pub use anchor_targets::AnchorTargets;
pub use brief_record::BriefRecord;
pub use callout_tone::CalloutTone;
pub use change_scope::ChangeScope;
pub use checked_studio::CheckedStudio;
pub use clean_orphan_workspaces::clean_orphan_workspaces;
pub use code_access::CodeAccess;
pub use code_anchor::CodeAnchor;
pub use code_side::CodeSide;
pub use diagram_change::DiagramChange;
pub use edge_style::EdgeStyle;
pub use file_listing::FileListing;
pub use find_claude::find_claude;
pub use graph_edge::GraphEdge;
pub use graph_node::GraphNode;
pub use message_style::MessageStyle;
pub use node_role::NodeRole;
pub use parse_studio::parse_studio;
pub use prepare_workspace::{DIFF_FILE, HEAD_FOLDER, prepare_workspace};
pub use reader_familiarity::ReaderFamiliarity;
pub use rejected_widget::RejectedWidget;
pub use request_layer::RequestLayer;
pub use review_workspace::ReviewWorkspace;
pub use run_claude::{run_claude, run_smart_filter};
pub use sequence_lane::SequenceLane;
pub use sequence_message::SequenceMessage;
pub use shown_page::ShownPage;
pub use smart_filter::{SmartBucket, SmartFilter, parse_smart_buckets};
pub use step_weight::StepWeight;
pub use studio::Studio;
pub use studio_graph::StudioGraph;
pub use studio_page::StudioPage;
pub use studio_prompt::{smart_filter_prompt, studio_prompt};
pub use studio_prompt_message::StudioPrompt;
pub use studio_request::StudioRequest;
pub use studio_request_file::StudioRequestFile;
pub use studio_schema::STUDIO_SCHEMA;
pub use studio_sequence::StudioSequence;
pub use studio_table::StudioTable;
pub use studio_tour::StudioTour;
pub use studio_widget::StudioWidget;
pub use table_role::TableRole;
pub use tool_call::ToolCall;
pub use tour_step::TourStep;
pub use workspace_diff::WorkspaceDiff;
pub use workspace_error::WorkspaceError;
pub use workspace_progress::WorkspaceProgress;
pub use workspace_root::workspace_root;
pub use workspace_source::WorkspaceSource;
pub use write_brief_report::{REPORT_FILE, report_root, write_brief_report};
