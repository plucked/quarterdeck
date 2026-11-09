use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Cancels an agent run when dropped: the run's blocking loops (the
/// checkout's git, then the agent) look at the shared flag at least every
/// 200 ms and stop their process (`SIGTERM`, then `SIGKILL` after 2 s). Held next to
/// the run's task, so replacing or dropping the run also stops the process
/// (dropping the task alone would leave it running).
pub struct RunCancel(Arc<AtomicBool>);

impl RunCancel {
    /// A guard and the flag it sets, for the run.
    pub fn new() -> (Self, Arc<AtomicBool>) {
        let flag = Arc::new(AtomicBool::new(false));
        (Self(flag.clone()), flag)
    }
}

impl Drop for RunCancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}
