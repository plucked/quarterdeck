/// A bounded excerpt of complete source around a selected content-search match.
pub(super) struct SearchPreview {
    pub file: usize,
    pub first_line: usize,
    pub selected_line: u32,
    pub lines: Vec<String>,
}
