/// The instructions appended to the agent's system prompt: the brief is a
/// presentation of the change, not a review; how to build up context and
/// order the reading, which widget suits which reader task, how anchors are
/// numbered, and the language rules. Adapted from the Review Studio skill's
/// authoring and representation guides and the research on presenting
/// changes to reviewers.
pub const STUDIO_GUIDE: &str = include_str!("studio_guide.md");
