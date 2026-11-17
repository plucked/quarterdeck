/// The data searched by the Files field.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) enum SearchScope {
    #[default]
    Paths,
    Contents,
}

impl SearchScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Paths => "Paths",
            Self::Contents => "Contents",
        }
    }
}
