use gpui::{Context, Div, FontWeight, IntoElement, Stateful, div, prelude::*, px};
use qd_cache::ColorTheme;
use qd_syntax::SyntaxKind;

use super::setting_row::SettingRow;
use super::settings_window::SettingsWindow;
use super::theme_preview::render_theme_preview;
use crate::app_settings::AppSettings;
use crate::controls::{SectionLabel, SegmentedControl};
use crate::theme::{self, colors_of, palette_of};

/// Height of a theme card's color strip.
const SWATCH_HEIGHT: f32 = 40.;
/// Width of one bar in a swatch, a token's color.
const SWATCH_BAR_WIDTH: f32 = 14.;
/// Size of a color swatch in the folder heat legend.
const HEAT_SWATCH_WIDTH: f32 = 20.;
const HEAT_SWATCH_HEIGHT: f32 = 8.;
/// The token colors a swatch shows, left to right.
const SWATCH_KINDS: [SyntaxKind; 4] = [
    SyntaxKind::Keyword,
    SyntaxKind::String,
    SyntaxKind::Definition,
    SyntaxKind::Constant,
];

impl SettingsWindow {
    /// The Appearance page: the eight themes as cards, light then dark (a
    /// click uses one at once), and a preview of the hovered theme, else
    /// the active one.
    pub(super) fn render_appearance_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = AppSettings::color_theme(cx);
        let previewed = self.previewed_theme().unwrap_or(active);
        let heatmap_enabled = AppSettings::folder_heatmap(cx);
        let (light, dark): (Vec<ColorTheme>, Vec<ColorTheme>) = ColorTheme::ALL
            .into_iter()
            .partition(|color_theme| !palette_of(*color_theme).is_dark);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().text_xs().text_color(theme::muted()).child(
                "Light themes read best in a lit room; dark ones suit a dim room. \
                         Hover a theme to preview it, click to use it.",
            ))
            .child(Self::render_theme_group("Light", &light, active, cx))
            .child(Self::render_theme_group("Dark", &dark, active, cx))
            .child(SettingRow::new(
                "Folder heatmap",
                "Color folders from cold to warm by the number of changed files in their hierarchy matching the current filters. Includes added, removed, moved and binary files.",
                SegmentedControl::new("folder-heatmap")
                    .option("Off", !heatmap_enabled, |_, cx| AppSettings::set_folder_heatmap(cx, false))
                    .option("On", heatmap_enabled, |_, cx| AppSettings::set_folder_heatmap(cx, true)),
            ))
            .when(heatmap_enabled, |page| page.child(render_folder_heat_preview(previewed)))
            .child(
                div().pt_2().child(
                    SectionLabel::new(format!("Preview · {}", palette_of(previewed).name))
                        .weight(FontWeight::NORMAL),
                ),
            )
            .child(render_theme_preview(previewed))
    }

    /// A labelled row of theme cards.
    fn render_theme_group(
        label: &'static str,
        color_themes: &[ColorTheme],
        active: ColorTheme,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut cards = Vec::with_capacity(color_themes.len());
        for color_theme in color_themes {
            cards.push(Self::render_theme_card(*color_theme, active, cx));
        }
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(SectionLabel::new(label).weight(FontWeight::NORMAL))
            .child(div().flex().gap_2().children(cards))
    }

    /// One theme: a strip in its own colors, its name and what it suits.
    /// The active theme is outlined in the accent.
    fn render_theme_card(
        color_theme: ColorTheme,
        active: ColorTheme,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let palette = palette_of(color_theme);
        let colors = colors_of(color_theme);
        let is_active = color_theme == active;
        div()
            .id(palette.name)
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .rounded_lg()
            .overflow_hidden()
            .border_2()
            .cursor_pointer()
            .bg(theme::surface())
            .border_color(if is_active {
                theme::accent()
            } else {
                theme::border()
            })
            .when(!is_active, |card| {
                card.hover(|style| style.border_color(theme::border_strong()))
            })
            .on_hover(cx.listener(move |settings, is_hovered: &bool, _, cx| {
                settings.preview_theme(color_theme, *is_hovered, cx);
            }))
            .on_click(move |_, _, cx| AppSettings::set_color_theme(cx, color_theme))
            // The strip: the theme's ground with a line of text and its token colors.
            .child(
                div()
                    .h(px(SWATCH_HEIGHT))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap_1()
                    .px_2()
                    .bg(colors.background)
                    .border_b_1()
                    .border_color(colors.border)
                    .child(div().h(px(3.)).w(px(56.)).rounded_sm().bg(colors.text))
                    .child(div().flex().gap_1().children(SWATCH_KINDS.map(|kind| {
                        div()
                            .h(px(3.))
                            .w(px(SWATCH_BAR_WIDTH))
                            .rounded_sm()
                            .bg(colors.syntax(kind))
                    })))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(div().h(px(3.)).w(px(24.)).rounded_sm().bg(colors.added))
                            .child(div().h(px(3.)).w(px(18.)).rounded_sm().bg(colors.removed)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .px_2()
                    .py_1p5()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(palette.name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::muted())
                            .truncate()
                            .child(palette.purpose),
                    ),
            )
    }
}

/// The heat scale in the hovered theme, on the same background as the file tree.
fn render_folder_heat_preview(color_theme: ColorTheme) -> impl IntoElement {
    let colors = colors_of(color_theme);
    div()
        .flex()
        .items_center()
        .gap_2()
        .p_2()
        .rounded_md()
        .bg(colors.panel)
        .text_xs()
        .text_color(colors.muted)
        .child("Cold · fewer files")
        .children([0., 0.25, 0.5, 0.75, 1.].map(|amount| {
            div()
                .flex_none()
                .w(px(HEAT_SWATCH_WIDTH))
                .h(px(HEAT_SWATCH_HEIGHT))
                .rounded_full()
                .bg(colors.folder_heat(amount))
        }))
        .child("Warm · more files")
}
