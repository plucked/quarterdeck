use crate::AnchorTargets;

/// Frozen run identity and eligible diff anchors for one brief authoring run.
#[derive(Clone)]
pub struct BriefRun<'a> {
    pub run_id: &'a str,
    pub snapshot_id: &'a str,
    pub targets: AnchorTargets,
}
