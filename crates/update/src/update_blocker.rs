/// Why the running app cannot update itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateBlocker {
    /// The executable runs outside an app bundle (`cargo run`).
    NotABundle,
    /// The bundle is not signed as a release (a development build).
    DevelopmentBuild,
    /// macOS runs the app from a read-only copy (App Translocation),
    /// because it was opened from the disk image or Downloads.
    Translocated,
    /// The folder holding the app cannot be written.
    ReadOnlyLocation,
}

impl UpdateBlocker {
    /// What to tell the user.
    pub fn describe(self) -> &'static str {
        match self {
            Self::NotABundle => "Updates are off: Quarterdeck is not running from its app bundle.",
            Self::DevelopmentBuild => "Updates are off in development builds.",
            Self::Translocated => {
                "Move Quarterdeck to the Applications folder and open it from there to get updates."
            }
            Self::ReadOnlyLocation => {
                "Updates need permission to write to the folder Quarterdeck is in."
            }
        }
    }
}
