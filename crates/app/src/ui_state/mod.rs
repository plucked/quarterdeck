//! Remembers what the UI showed (window placement, the review queue's
//! selection and scroll) in the local database, so a rebuild-and-relaunch
//! returns to the same place. Every step falls back to defaults: nothing
//! saved, a row that no longer reads, or a value that no longer fits (a
//! window off every display, a pull request that left the queue) restores
//! nothing for that part, never an error.
//!
//! Files:
//! - `ui_state_recorder.rs`: `UiStateRecorder`, the gpui global that reads saved state and batches writes.
//! - `pending_ui_state.rs`: `PendingUiState`, the changes recorded since the last write, and how they are written.
//! - `ui_state_store_reads.rs`: reads (and the brief write) through the shared store on the background executor.
//! - `shared_ui_state_store.rs`: `SharedUiStateStore`, the store shared with background reads and writes; its lock is used as is when poisoned.
//! - `review_change.rs`: `ReviewChange`, a change to the review notes waiting to be written.
//! - `window_placement_conversion.rs`: converts between gpui window bounds and the saved placement.

mod pending_ui_state;
mod review_change;
mod shared_ui_state_store;
mod ui_state_recorder;
mod ui_state_store_reads;
mod window_placement_conversion;

pub use review_change::ReviewChange;
pub use shared_ui_state_store::SharedUiStateStore;
pub use ui_state_recorder::UiStateRecorder;
pub use ui_state_store_reads::{
    read_brief, read_brief_index, read_review_notes, read_viewed_files, write_brief,
};
pub use window_placement_conversion::{placement_from_window, window_bounds_from_placement};
