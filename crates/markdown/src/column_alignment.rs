/// How a table column's cells align, from the `:---:` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlignment {
    /// No colon given; renders like `Left`.
    Unspecified,
    Left,
    Center,
    Right,
}

impl ColumnAlignment {
    pub(crate) fn from_parser(alignment: pulldown_cmark::Alignment) -> Self {
        match alignment {
            pulldown_cmark::Alignment::None => Self::Unspecified,
            pulldown_cmark::Alignment::Left => Self::Left,
            pulldown_cmark::Alignment::Center => Self::Center,
            pulldown_cmark::Alignment::Right => Self::Right,
        }
    }
}
