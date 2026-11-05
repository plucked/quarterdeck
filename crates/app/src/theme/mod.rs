//! The app's colors and fonts. Eight color themes, the "Proofreader
//! Palettes" (four light, four dark; Graphite by default), each built so
//! every color clears a contrast target against its own background, with
//! diffs in blue (added) and orange (removed) that stay apart for red-green
//! colour blindness; green and red only mark success and danger. Views take
//! every color from the functions here, which read the active theme, and
//! every font from the constants.
//!
//! Files:
//! - `palette.rs`: `Palette`, a theme as designed: name, purpose, its colors.
//! - `palettes.rs`: the eight palettes and `palette_of`.
//! - `theme_colors.rs`: `ThemeColors`, every color the views use, worked out from a palette.
//! - `active_theme.rs`: which theme is active; `set_active_theme`, `colors_of`.
//! - `color_tokens.rs`: the color functions views call (`text()`, `syntax(kind)`, …).
//! - `theme_fonts.rs`: the interface and code fonts.

mod active_theme;
mod color_tokens;
mod palette;
mod palettes;
mod theme_colors;
mod theme_fonts;

pub use active_theme::{colors_of, set_active_theme};
pub use color_tokens::*;
pub use palettes::palette_of;
pub use theme_colors::ThemeColors;
pub use theme_fonts::{MONO_FONT, UI_FONT};
