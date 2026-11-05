use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use qd_cache::ColorTheme;

use super::palettes::palette_of;
use super::theme_colors::ThemeColors;

/// The active theme as its position in `ColorTheme::ALL`. A process-wide
/// value rather than a gpui global: views read colors through plain
/// functions (`theme::text()`) in hundreds of places that have no context
/// at hand. `AppSettings` keeps it in step with the saved preference.
static ACTIVE_THEME: AtomicUsize = AtomicUsize::new(DEFAULT_POSITION);
const DEFAULT_POSITION: usize = 4;

/// Makes `theme` the one every color function reads; windows show it on
/// their next redraw.
pub fn set_active_theme(theme: ColorTheme) {
    ACTIVE_THEME.store(position_of(theme), Ordering::Relaxed);
}

/// The colors of the active theme.
pub fn active_colors() -> &'static ThemeColors {
    &all_colors()[ACTIVE_THEME.load(Ordering::Relaxed)]
}

/// The colors of `theme`, active or not (the settings preview draws any).
pub fn colors_of(theme: ColorTheme) -> &'static ThemeColors {
    &all_colors()[position_of(theme)]
}

/// Every theme's colors, worked out on first use.
fn all_colors() -> &'static [ThemeColors; 8] {
    static ALL_COLORS: OnceLock<[ThemeColors; 8]> = OnceLock::new();
    ALL_COLORS
        .get_or_init(|| ColorTheme::ALL.map(|theme| ThemeColors::from_palette(palette_of(theme))))
}

fn position_of(theme: ColorTheme) -> usize {
    ColorTheme::ALL
        .iter()
        .position(|candidate| *candidate == theme)
        .unwrap_or(DEFAULT_POSITION)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Graphite, the default preference, is what shows before one is set.
    #[test]
    fn starts_with_the_default_theme() {
        assert_eq!(ColorTheme::ALL[DEFAULT_POSITION], ColorTheme::default());
    }
}
