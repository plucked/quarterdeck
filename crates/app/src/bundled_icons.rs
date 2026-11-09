//! `BundledIcons`: the app's asset source, serving the SVG icons built into
//! the binary to gpui's `svg()` element.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

/// The asset path of GitHub's mark (Octicons' `mark-github`, MIT), drawn in
/// the text color by `svg().path(GITHUB_MARK)`.
pub const GITHUB_MARK: &str = "icons/github-mark.svg";

/// Every bundled icon: its asset path and its SVG.
const ICONS: [(&str, &[u8]); 1] = [(
    GITHUB_MARK,
    include_bytes!("../assets/icons/github-mark.svg"),
)];

/// Serves the icons compiled into the binary (`assets/icons`); any other
/// path has no asset.
pub struct BundledIcons;

impl AssetSource for BundledIcons {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(icon_path, _)| *icon_path == path)
            .map(|(_, svg)| Cow::Borrowed(*svg)))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(icon_path, _)| icon_path.starts_with(path))
            .map(|(icon_path, _)| SharedString::from(*icon_path))
            .collect())
    }
}
