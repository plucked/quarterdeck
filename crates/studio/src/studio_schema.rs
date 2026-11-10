/// The JSON Schema of the agent's output: a lead and pages of widgets in
/// reading order, for Claude Code's `--json-schema` (draft-07, re-prompting
/// on a mismatch). Every property is required and none are additional; a
/// value the agent may leave out is nullable instead of optional, so each
/// answer spells out every field.
pub const STUDIO_SCHEMA: &str = include_str!("studio_schema.json");
