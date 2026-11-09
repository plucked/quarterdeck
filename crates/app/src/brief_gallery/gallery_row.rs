use crate::brief_view::BriefPageRow;

/// One row of the gallery's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryRow {
    /// What the gallery is, and where its sample comes from.
    Intro,
    /// A group's heading.
    Heading(&'static str),
    /// Sample status card `index`.
    Status(usize),
    /// The sample brief's version line with its Report button, and the
    /// report panel while open.
    Version,
    /// The sample brief's page tabs.
    PageTabs,
    /// A row of the sample brief's shown page.
    Page(BriefPageRow),
    /// Why there is no sample: the compiled-in studio no longer checks.
    Broken,
}
