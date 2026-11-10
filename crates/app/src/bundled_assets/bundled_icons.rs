//! `BundledIcons`: the app's asset source, serving the SVG icons built into
//! the binary to gpui's `svg()` element (single-color icons tinted by the
//! text color) and to `img()` (the file tree's full-color file icons).

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

use super::file_icon_svgs::{file_icon_paths, file_icon_svg};

/// The asset path of GitHub's mark (Octicons' `mark-github`, MIT), drawn in
/// the text color by `svg().path(GITHUB_MARK)`.
pub const GITHUB_MARK: &str = "icons/github-mark.svg";

/// Every bundled icon: its asset path and its SVG.
const ICONS: [(&str, &[u8]); 1] = [(
    GITHUB_MARK,
    include_bytes!("../../assets/icons/github-mark.svg"),
)];

/// Serves the icons compiled into the binary (`assets/icons`, the file
/// icons under `icons/files/`); any other path has no asset.
pub struct BundledIcons;

impl AssetSource for BundledIcons {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let svg = ICONS
            .iter()
            .find(|(icon_path, _)| *icon_path == path)
            .map(|(_, svg)| *svg)
            .or_else(|| file_icon_svg(path));
        Ok(svg.map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(icon_path, _)| *icon_path)
            .chain(file_icon_paths())
            .filter(|icon_path| icon_path.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}
