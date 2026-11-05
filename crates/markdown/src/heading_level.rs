/// A heading's level, `#` (One) to `######` (Six).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl HeadingLevel {
    pub(crate) fn from_parser(level: pulldown_cmark::HeadingLevel) -> Self {
        match level {
            pulldown_cmark::HeadingLevel::H1 => Self::One,
            pulldown_cmark::HeadingLevel::H2 => Self::Two,
            pulldown_cmark::HeadingLevel::H3 => Self::Three,
            pulldown_cmark::HeadingLevel::H4 => Self::Four,
            pulldown_cmark::HeadingLevel::H5 => Self::Five,
            pulldown_cmark::HeadingLevel::H6 => Self::Six,
        }
    }
}
