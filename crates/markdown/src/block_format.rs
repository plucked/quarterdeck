/// Parsed source prefixes of editable Markdown blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockFormat {
    Heading,
    Quote,
    Bullets,
    Numbered,
    Checklist,
}
