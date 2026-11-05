/// The display a window was on, as the operating system's stable display
/// UUID in text form (macOS keeps it across reconnects and restarts). Set
/// by the app from gpui's display; only ever compared for equality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayUuid(String);

impl DisplayUuid {
    pub fn new(uuid: impl Into<String>) -> Self {
        Self(uuid.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
