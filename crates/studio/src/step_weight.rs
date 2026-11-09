/// How much of the change a reading-order step carries, so the reader
/// knows where to slow down: stated in words, never as a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepWeight {
    /// Where the change's behaviour lives; read closely.
    Core,
    /// Follows from the core: call sites, wiring, tests.
    Supporting,
    /// Renames, moves, formatting, generated files; skim.
    Mechanical,
}

impl StepWeight {
    /// The weight as the reader sees it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Supporting => "Supporting",
            Self::Mechanical => "Mechanical",
        }
    }
}
