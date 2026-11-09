use super::anchor_targets::AnchorTargets;
use super::workspace_diff::WorkspaceDiff;

/// The user message for the agent, the lines its anchors may cite, and,
/// with a workspace, every changed file's numbered diff for the
/// workspace's `diffs/` folder (the message lists only what fits its
/// budget; anchors may cite any of them).
pub struct StudioPrompt {
    pub text: String,
    pub targets: AnchorTargets,
    /// Every diff, listed or not, numbered; empty without a workspace.
    pub workspace_diffs: Vec<WorkspaceDiff>,
}
