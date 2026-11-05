use qd_cache::LineWrapping;

/// What every diff row of one file shares: the line-number gutter's width,
/// whether long lines wrap, and, when they scroll instead, how far the
/// file's lines are scrolled sideways (pixels from the left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffRowStyle {
    pub gutter_width: f32,
    pub wrapping: LineWrapping,
    pub horizontal_offset: f32,
}
