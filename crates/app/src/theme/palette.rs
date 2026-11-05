/// One color theme as designed: a name, what it is for, and its colors as
/// `0xRRGGBB`. `ThemeColors` derives every color the views use from it.
pub struct Palette {
    pub name: &'static str,
    /// What the theme suits, shown under its name in the picker.
    pub purpose: &'static str,
    pub is_dark: bool,
    /// The diff's ground.
    pub background: u32,
    /// The sidebar, gutters and code blocks.
    pub panel: u32,
    /// A current-line tint: headers and the side panel.
    pub line: u32,
    pub selection: u32,
    pub border: u32,
    pub text: u32,
    /// Line numbers (3:1 or more).
    pub line_number: u32,
    pub comment: u32,
    pub keyword: u32,
    pub string: u32,
    pub constant: u32,
    /// Function names, links and the accent.
    pub definition: u32,
    pub type_name: u32,
    pub error: u32,
    /// The added-line mark (blue).
    pub added: u32,
    /// The removed-line mark (orange).
    pub removed: u32,
    pub added_line: u32,
    pub added_words: u32,
    pub removed_line: u32,
    pub removed_words: u32,
}
