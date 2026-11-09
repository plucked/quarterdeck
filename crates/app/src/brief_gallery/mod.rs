//! The brief gallery (Window → Brief Gallery): every status card and every
//! widget of a brief, on a sample compiled into the app, drawn by
//! `brief_view` like the Brief pane, to work on how briefs look without
//! running an agent.
//!
//! Files:
//! - `open_brief_gallery.rs`: opens the window, or brings the open one to the front.
//! - `brief_gallery.rs`: `BriefGallery`, the window's view: the sample, its rows, folded reasoning.
//! - `gallery_rendering.rs`: the gallery's rows, each part under a label naming it.
//! - `gallery_row.rs`: `GalleryRow`, one row of the gallery's list.
//! - `sample_brief.rs`: `SampleBrief`, the status cards and a checked, prepared sample brief.
//! - `sample_diffs.rs`: `SampleDiffs`, the diffs the sample quotes, built from compiled-in patches.
//! - `samples/`: the sample studio (an agent answer, as JSON) and the patches it quotes.

mod brief_gallery;
mod gallery_rendering;
mod gallery_row;
mod open_brief_gallery;
mod sample_brief;
mod sample_diffs;

pub use open_brief_gallery::open_brief_gallery;
