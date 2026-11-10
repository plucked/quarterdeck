//! Small reusable pieces of UI drawn the same way wherever they show, and the
//! handler helper the option controls share.
//!
//! Files:
//! - `button.rs`: `Button`, a small text button.
//! - `checkbox.rs`: `Checkbox`, a drawn checkbox (checked, unchecked or mixed).
//! - `segmented_control.rs`: `SegmentedControl`, a row of exclusive options.
//! - `view_option.rs`: `view_option`, an option's select handler passing its value to the view.
//! - `fold_chevron.rs`: `FoldChevron`, the button folding a group.
//! - `resize_handle.rs`: `ResizeHandle`, a divider dragged to resize the panel beside it.
//! - `text_tooltip.rs`: `TextTooltip`, a short text shown over an element on hover.
//! - `section_label.rs`: `SectionLabel`, the small uppercase subtle label heading a section.
//! - `pulsing_status.rs`: `PulsingStatus`, an accent status that pulses while it lasts, and the shared dot pulse.
//! - `repo_monogram.rs`: `RepoMonogram`, a repository's colored letter (sidebar, settings).
//! - `pill.rs`: `Pill` + `PillStyle`, a short label on a fully rounded shape (badges, counts, status).

mod button;
mod checkbox;
mod fold_chevron;
mod pill;
mod pulsing_status;
mod repo_monogram;
mod resize_handle;
mod section_label;
mod segmented_control;
mod text_tooltip;
mod view_option;

pub use button::Button;
pub use checkbox::{CheckState, Checkbox};
pub use fold_chevron::{FOLD_CHEVRON_SIZE, FoldChevron, FoldState};
pub use pill::{Pill, PillStyle};
pub use pulsing_status::{PulsingStatus, dot_pulse};
pub use repo_monogram::RepoMonogram;
pub use resize_handle::{ResizeEdge, ResizeHandle};
pub use section_label::SectionLabel;
pub use segmented_control::SegmentedControl;
pub use text_tooltip::TextTooltip;
pub use view_option::view_option;
