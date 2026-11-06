use gpui::{AnyElement, Div, FontWeight, SharedString, Stateful, div, prelude::*, px};
use qd_file_tree::{FacetChoice, FacetCount};

use crate::theme;

/// Chips a collapsed row shows before "more"; picked or hidden ones always show.
pub const COLLAPSED_CHIPS: usize = 8;
/// Width of a chip row's label column, so the rows' chips line up.
const ROW_LABEL_WIDTH: f32 = 50.;
/// Size of a chip row's label.
const ROW_LABEL_TEXT_SIZE: f32 = 10.;
/// Nudges the label down to the chips' text line.
const ROW_LABEL_TOP: f32 = 3.;

/// The [`COLLAPSED_CHIPS`] values with the most items (ties keep the
/// counts' order), which a collapsed row shows.
pub fn most_common_values(counts: &[FacetCount<String>]) -> Vec<&str> {
    let mut by_size: Vec<&FacetCount<String>> = counts.iter().collect();
    by_size.sort_by_key(|count| std::cmp::Reverse(count.total));
    by_size
        .into_iter()
        .take(COLLAPSED_CHIPS)
        .map(|count| count.value.as_str())
        .collect()
}

/// A row of chips after its small uppercase label.
pub fn render_chip_row(label: &'static str, chips: Vec<AnyElement>) -> Div {
    div()
        .flex()
        .items_start()
        .gap_1()
        .child(
            div()
                .flex_none()
                .w(px(ROW_LABEL_WIDTH))
                .pt(px(ROW_LABEL_TOP))
                .text_size(px(ROW_LABEL_TEXT_SIZE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::subtle())
                .child(label.to_uppercase()),
        )
        // Lines start at the top and keep their own height, so a chip never stretches.
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_start()
                .content_start()
                .gap_1()
                .children(chips),
        )
}

/// One chip with what picking it would show: picked chips stand out, a
/// hidden one is struck through, and one that would show nothing is dimmed.
/// `leading` goes before the label (an author's avatar). The caller adds
/// the click.
pub fn render_chip<T>(
    id: SharedString,
    label: &str,
    count: &FacetCount<T>,
    choice: FacetChoice,
    leading: Option<AnyElement>,
) -> Stateful<Div> {
    let number = match choice {
        FacetChoice::Hidden => count.total,
        FacetChoice::Open | FacetChoice::Only => count.matching,
    };
    let chip = div()
        .id(id)
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_0p5()
        .rounded_full()
        .border_1()
        .text_xs()
        .cursor_pointer()
        .children(leading)
        .child(label.to_owned())
        .child(div().text_color(theme::subtle()).child(number.to_string()));
    match choice {
        FacetChoice::Only => chip
            .border_color(theme::accent())
            .bg(theme::selected())
            .text_color(theme::text())
            .font_weight(FontWeight::MEDIUM),
        FacetChoice::Hidden => chip
            .border_color(theme::border())
            .text_color(theme::subtle())
            .line_through(),
        FacetChoice::Open => chip
            .border_color(theme::border())
            .bg(theme::surface())
            .text_color(theme::muted())
            .hover(|style| {
                style
                    .border_color(theme::border_strong())
                    .text_color(theme::text())
            })
            .when(count.matching == 0, |chip| chip.opacity(0.45)),
    }
}

/// The `+3 more` / `less` link closing a chip row; the caller adds the click.
pub fn render_more_chip(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_1()
        .py_0p5()
        .text_xs()
        .text_color(theme::accent())
        .cursor_pointer()
        .child(label.into())
}
