/// The JSON Schema of the agent's output: a lead and pages of widgets in
/// reading order. It is accepted by both Claude Code's `--json-schema` (draft-07,
/// re-prompting on a mismatch) and Codex's `--output-schema` (strict mode:
/// every property required, no additional properties, nullable instead of
/// optional).
pub const STUDIO_SCHEMA: &str = include_str!("studio_schema.json");
