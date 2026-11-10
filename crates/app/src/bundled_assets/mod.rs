//! The fonts and icons compiled into the binary, and how gpui gets them.
//!
//! Files:
//! - `bundled_fonts.rs`: registers the fonts built into the binary (`Schibsted Grotesk`, `JetBrains Mono`).
//! - `bundled_icons.rs`: `BundledIcons`, the asset source serving the SVG icons built into the binary.
//! - `file_icon_svgs.rs`: the SVG of each file icon in the file tree, and which one the active theme draws.

mod bundled_fonts;
mod bundled_icons;
mod file_icon_svgs;

pub use bundled_fonts::load_bundled_fonts;
pub use bundled_icons::{BundledIcons, GITHUB_MARK};
pub use file_icon_svgs::file_icon_path;
