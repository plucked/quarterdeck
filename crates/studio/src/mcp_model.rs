/// Retains each artifact's established Claude model selection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpModel {
    ClaudeDefault,
    OpusHigh,
}
