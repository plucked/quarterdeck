/// What kind of statement a callout makes, which decides its color and label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutTone {
    /// A qualification or side remark the reader should not miss.
    Note,
    /// A limit of the change, with its cause and effect.
    Constraint,
    /// The central consequence of the change, in one or two sentences.
    Verdict,
    /// What was verified and how, or what the author reports.
    Evidence,
}

impl CalloutTone {
    /// The small label above the callout.
    pub fn label(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Constraint => "Constraint",
            Self::Verdict => "In short",
            Self::Evidence => "Evidence",
        }
    }
}
