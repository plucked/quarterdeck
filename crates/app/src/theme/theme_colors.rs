use gpui::{Rgba, rgb, rgba};
use qd_syntax::SyntaxKind;

use super::palette::Palette;

/// Contrast of secondary text (metadata, placeholders) on the background.
const MUTED_CONTRAST: f32 = 6.;
/// Contrast of tertiary text (hints, counts) on the background: WCAG AA.
const SUBTLE_CONTRAST: f32 = 4.5;
/// How far secondary text may move from the text color toward the
/// background, at most, while searching for its contrast.
const MAX_TEXT_FADE: f32 = 0.6;
const TEXT_FADE_STEP: f32 = 0.01;
/// The scrollbar thumb: the text color at this opacity, at rest and dragged.
const SCROLLBAR_ALPHA: u32 = 0x33;
const SCROLLBAR_DRAGGED_ALPHA: u32 = 0x66;
/// The tint over lines a comment is being written about: the added color at this opacity.
const COMMENT_RANGE_ALPHA: u32 = 0x26;
/// Five steps from the palette's cool blue to its warm orange.
const FOLDER_HEAT_STOPS: [f32; 5] = [0., 0.25, 0.5, 0.75, 1.];
/// Midpoints selecting the nearest heat color, from cold to warm.
const FOLDER_HEAT_BOUNDARIES: [f64; 4] = [0.125, 0.375, 0.625, 0.875];
/// Steps toward the text color when a heat color needs more contrast.
const HEAT_CONTRAST_STEPS: u16 = 100;

/// Every color the views use, worked out once from a `Palette`. The
/// surfaces step from the background toward the text color, so raised
/// things are lighter in a dark theme and darker in a light one.
pub struct ThemeColors {
    pub background: Rgba,
    pub sidebar: Rgba,
    pub panel: Rgba,
    pub surface: Rgba,
    pub border: Rgba,
    pub border_strong: Rgba,
    pub hover: Rgba,
    pub selected: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub subtle: Rgba,
    pub line_number: Rgba,
    pub accent: Rgba,
    pub on_accent: Rgba,
    pub scrollbar_thumb: Rgba,
    pub scrollbar_thumb_active: Rgba,
    pub text_selection: Rgba,
    pub comment_range: Rgba,
    pub code_background: Rgba,
    pub success: Rgba,
    pub danger: Rgba,
    pub added: Rgba,
    pub removed: Rgba,
    pub added_background: Rgba,
    pub removed_background: Rgba,
    pub added_emphasis: Rgba,
    pub removed_emphasis: Rgba,
    pub diff_filler: Rgba,
    pub hunk_background: Rgba,
    pub renamed: Rgba,
    /// Folder name and count colors, readable on panels, count capsules and hovered rows.
    folder_heat: [Rgba; 5],
    /// Syntax colors in `SyntaxKind::ALL` order.
    syntax: [Rgba; 7],
    /// Whether the theme is dark: pictures drawn in near-white (some file
    /// icons) have a darker variant for light themes.
    pub is_dark: bool,
}

impl ThemeColors {
    /// Works out the views' colors from `palette`: its own colors where it
    /// has one, mixes of them for the rest, and secondary text faded from
    /// the text color only as far as its contrast target allows.
    pub fn from_palette(palette: &Palette) -> Self {
        let raise = |amount: f32| rgb(mix(palette.background, palette.text, amount));
        let faded_text = |contrast: f32| rgb(fade_to_contrast(palette, contrast));
        Self {
            background: rgb(palette.background),
            sidebar: rgb(palette.panel),
            panel: rgb(palette.line),
            surface: raise(0.06),
            border: rgb(palette.border),
            border_strong: rgb(mix(palette.border, palette.text, 0.18)),
            hover: raise(0.09),
            selected: rgb(palette.selection),
            text: rgb(palette.text),
            muted: faded_text(MUTED_CONTRAST),
            subtle: faded_text(SUBTLE_CONTRAST),
            line_number: rgb(palette.line_number),
            accent: rgb(palette.definition),
            on_accent: rgb(palette.background),
            scrollbar_thumb: rgba(palette.text << 8 | SCROLLBAR_ALPHA),
            scrollbar_thumb_active: rgba(palette.text << 8 | SCROLLBAR_DRAGGED_ALPHA),
            text_selection: rgb(palette.selection),
            comment_range: rgba(palette.added << 8 | COMMENT_RANGE_ALPHA),
            code_background: rgb(palette.panel),
            success: rgb(palette.string),
            danger: rgb(palette.error),
            added: rgb(palette.added),
            removed: rgb(palette.removed),
            added_background: rgb(palette.added_line),
            removed_background: rgb(palette.removed_line),
            added_emphasis: rgb(palette.added_words),
            removed_emphasis: rgb(palette.removed_words),
            diff_filler: rgb(mix(palette.background, palette.panel, 0.6)),
            hunk_background: rgb(mix(palette.panel, palette.added, 0.06)),
            renamed: rgb(palette.keyword),
            folder_heat: FOLDER_HEAT_STOPS.map(|amount| rgb(folder_heat_color(palette, amount))),
            syntax: [
                palette.comment,
                palette.keyword,
                palette.string,
                palette.constant,
                palette.definition,
                palette.type_name,
                palette.error,
            ]
            .map(rgb),
            is_dark: palette.is_dark,
        }
    }

    /// The color of a token of `kind`.
    pub fn syntax(&self, kind: SyntaxKind) -> Rgba {
        let index = SyntaxKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or_default();
        self.syntax[index]
    }

    /// The nearest cold-to-warm color for a folder's relative activity (0 to 1).
    pub fn folder_heat(&self, amount: f64) -> Rgba {
        let index = FOLDER_HEAT_BOUNDARIES
            .iter()
            .position(|boundary| amount <= *boundary)
            .unwrap_or(FOLDER_HEAT_BOUNDARIES.len());
        self.folder_heat[index]
    }
}

/// A palette-derived blue-to-orange color, moved toward the text color
/// just far enough to keep small folder labels readable at rest and on hover.
fn folder_heat_color(palette: &Palette, amount: f32) -> u32 {
    let color = mix(palette.added, palette.removed, amount);
    let hover = mix(palette.background, palette.text, 0.09);
    let surface = mix(palette.background, palette.text, 0.06);
    for step in 0..=HEAT_CONTRAST_STEPS {
        let candidate = mix(
            color,
            palette.text,
            f32::from(step) / f32::from(HEAT_CONTRAST_STEPS),
        );
        if contrast_ratio(candidate, palette.line) >= SUBTLE_CONTRAST
            && contrast_ratio(candidate, hover) >= SUBTLE_CONTRAST
            && contrast_ratio(candidate, surface) >= SUBTLE_CONTRAST
        {
            return candidate;
        }
    }
    palette.text
}

/// The text color moved toward the background as far as it can go (up to
/// `MAX_TEXT_FADE`) while keeping `contrast` against the background.
fn fade_to_contrast(palette: &Palette, contrast: f32) -> u32 {
    let mut fade = MAX_TEXT_FADE;
    while fade > 0. {
        let candidate = mix(palette.text, palette.background, fade);
        if contrast_ratio(candidate, palette.background) >= contrast {
            return candidate;
        }
        fade -= TEXT_FADE_STEP;
    }
    palette.text
}

/// `from` moved `amount` (0 to 1) of the way to `to`, per channel in sRGB.
fn mix(from: u32, to: u32, amount: f32) -> u32 {
    let from = from.to_be_bytes();
    let to = to.to_be_bytes();
    let channel = |index: usize| {
        let start = f32::from(from[index]);
        let end = f32::from(to[index]);
        // In range by construction: a point between two bytes.
        let mixed = (start + (end - start) * amount).round().clamp(0., 255.);
        u32::from(byte_of(mixed))
    };
    channel(1) << 16 | channel(2) << 8 | channel(3)
}

/// A channel value already rounded into 0–255 as a byte.
fn byte_of(value: f32) -> u8 {
    (0..=u8::MAX)
        .find(|byte| f32::from(*byte) >= value)
        .unwrap_or(u8::MAX)
}

/// The WCAG 2 contrast ratio of two `0xRRGGBB` colors (1 to 21).
fn contrast_ratio(first: u32, second: u32) -> f32 {
    let (first, second) = (relative_luminance(first), relative_luminance(second));
    let (lighter, darker) = (first.max(second), first.min(second));
    (lighter + 0.05) / (darker + 0.05)
}

/// WCAG 2 relative luminance of an sRGB color.
fn relative_luminance(color: u32) -> f32 {
    let [_, red, green, blue] = color.to_be_bytes();
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
}

#[cfg(test)]
mod tests {
    use qd_cache::ColorTheme;

    use super::super::palettes::palette_of;
    use super::*;

    /// Every heat step stays readable on folders, count capsules and hovered rows in all themes.
    #[test]
    fn folder_heat_colors_are_readable_in_every_theme() {
        for theme in ColorTheme::ALL {
            let palette = palette_of(theme);
            let hover = mix(palette.background, palette.text, 0.09);
            let surface = mix(palette.background, palette.text, 0.06);
            for amount in FOLDER_HEAT_STOPS {
                let color = folder_heat_color(palette, amount);
                assert!(
                    contrast_ratio(color, palette.line) >= SUBTLE_CONTRAST,
                    "{}: {amount}",
                    palette.name
                );
                assert!(
                    contrast_ratio(color, hover) >= SUBTLE_CONTRAST,
                    "{} hovered: {amount}",
                    palette.name
                );
                assert!(
                    contrast_ratio(color, surface) >= SUBTLE_CONTRAST,
                    "{} count capsule: {amount}",
                    palette.name
                );
            }
            let colors = ThemeColors::from_palette(palette);
            assert_ne!(
                colors.folder_heat(0.),
                colors.folder_heat(1.),
                "{}",
                palette.name
            );
        }
    }

    /// Black on white is 21:1, and the formula is symmetric.
    #[test]
    fn computes_wcag_contrast() {
        assert!((contrast_ratio(0x00_00_00, 0xff_ff_ff) - 21.).abs() < 0.01);
        assert!(
            (contrast_ratio(0x1c_1e_21, 0xe5_e3_df) - contrast_ratio(0xe5_e3_df, 0x1c_1e_21)).abs()
                < f32::EPSILON
        );
    }

    /// Mixing goes from one color to the other.
    #[test]
    fn mixes_per_channel() {
        assert_eq!(mix(0x00_00_00, 0xff_ff_ff, 0.), 0x00_00_00);
        assert_eq!(mix(0x00_00_00, 0xff_ff_ff, 1.), 0xff_ff_ff);
        assert_eq!(mix(0x00_00_00, 0xc8_64_00, 0.5), 0x64_32_00);
    }

    /// In every theme, text, secondary and tertiary text meet their targets.
    #[test]
    fn every_theme_keeps_text_readable() {
        for theme in ColorTheme::ALL {
            let palette = palette_of(theme);
            let background = palette.background;
            let muted = fade_to_contrast(palette, MUTED_CONTRAST);
            let subtle = fade_to_contrast(palette, SUBTLE_CONTRAST);
            assert!(
                contrast_ratio(palette.text, background) >= 7.,
                "{}",
                palette.name
            );
            assert!(
                contrast_ratio(muted, background) >= MUTED_CONTRAST,
                "{}",
                palette.name
            );
            assert!(
                contrast_ratio(subtle, background) >= SUBTLE_CONTRAST,
                "{}",
                palette.name
            );
            // Secondary text is still a step back from the text.
            assert_ne!(muted, palette.text, "{}", palette.name);
        }
    }
}
