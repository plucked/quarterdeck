/// Frozen application identity and changed paths for one guided run.
/// The host produces both identities and checks publication ownership later.
#[derive(Debug, Clone, Copy)]
pub struct GuidedReviewRun<'a> {
    pub run_id: &'a str,
    pub snapshot_id: &'a str,
    pub changed_paths: &'a [String],
}
