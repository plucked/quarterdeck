use gpui::Task;
use qd_studio::{AgentProgress, SmartFilter};

use super::run_cancel::RunCancel;

/// One scope's saved plan and optional replacement in flight. Keeping the
/// old plan separate means cancellation or failure never loses its buckets.
#[derive(Default)]
pub(super) struct SmartFilterState {
    pub filter: Option<SmartFilter>,
    pub progress: Option<AgentProgress>,
    pub error: Option<String>,
    pub run: Option<(Task<()>, RunCancel)>,
}
