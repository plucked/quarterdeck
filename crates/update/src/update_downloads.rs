use std::path::PathBuf;

/// The app's cache folder name under `~/Library/Caches`.
const CACHE_DIRECTORY: &str = "de.plucked.quarterdeck";

/// Where downloaded releases are kept until they are unpacked:
/// `~/Library/Caches/de.plucked.quarterdeck/updates`.
pub fn update_download_directory() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join(CACHE_DIRECTORY).join("updates"))
}
