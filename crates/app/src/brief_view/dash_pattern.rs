/// How a dashed diagram line breaks up: dashes of `dash` px with `gap` px
/// between them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashPattern {
    pub dash: f32,
    pub gap: f32,
}

impl DashPattern {
    /// Async arrows and replies.
    pub const DASHED: Self = Self { dash: 5., gap: 4. };
    /// Optional arrows and lifelines.
    pub const DOTTED: Self = Self { dash: 2., gap: 3. };
}
