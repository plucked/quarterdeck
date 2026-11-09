use qd_studio::ReaderFamiliarity;

use super::brief_diffs::BriefDiffs;
use super::brief_handlers::BriefHandlers;

/// What drawing a brief's widgets needs from the view showing them: the
/// diffs code is quoted from, the reader picked now, and what clicks do.
pub struct BriefWidgetContext<'a> {
    pub diffs: &'a dyn BriefDiffs,
    /// The reader familiarity picked now, for writing the brief again.
    pub reader: ReaderFamiliarity,
    pub handlers: &'a BriefHandlers,
}
