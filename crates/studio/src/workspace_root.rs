use std::path::PathBuf;

/// The app's cache folder name, as macOS keys it by bundle identifier.
const CACHE_DIRECTORY: &str = "de.plucked.quarterdeck";
/// The folder in it holding review workspaces.
const WORKSPACES_DIRECTORY: &str = "review-workspaces";

/// Where review workspaces are made:
/// `~/Library/Caches/de.plucked.quarterdeck/review-workspaces`; `None`
/// when the system has no cache folder.
pub fn workspace_root() -> Option<PathBuf> {
    Some(
        dirs::cache_dir()?
            .join(CACHE_DIRECTORY)
            .join(WORKSPACES_DIRECTORY),
    )
}
