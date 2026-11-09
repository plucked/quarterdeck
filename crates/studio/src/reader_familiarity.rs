/// How well the reader knows the area a pull request changes, which sets
/// how much onboarding the brief gives before the change itself: context
/// that helps a newcomer only slows down someone who knows the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReaderFamiliarity {
    /// Knows the code and its domain: only the change, no background.
    Expert,
    /// Has a rough overview: the mechanisms the change touches explained.
    #[default]
    Overview,
    /// New to the area: onboarding to it before the change.
    Newcomer,
}

impl ReaderFamiliarity {
    pub const ALL: [Self; 3] = [Self::Expert, Self::Overview, Self::Newcomer];

    /// The choice as the reader picks it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Expert => "I know this code well",
            Self::Overview => "I know the area roughly",
            Self::Newcomer => "I'm new to this area",
        }
    }

    /// The reader as the footer and a report describe them, after "a
    /// reader who".
    pub fn reader_description(self) -> &'static str {
        match self {
            Self::Expert => "knows this code well",
            Self::Overview => "knows the area roughly",
            Self::Newcomer => "is new to this area",
        }
    }

    /// What the brief then does, in one line under the choice.
    pub fn effect(self) -> &'static str {
        match self {
            Self::Expert => {
                "Just the change: what is different, its scope and the order to read it."
            }
            Self::Overview => "Explains the mechanisms the change touches and the terms it uses.",
            Self::Newcomer => {
                "Starts with background on the area, from the surrounding code, before the change."
            }
        }
    }
}
