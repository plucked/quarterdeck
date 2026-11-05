use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, IntoElement, SharedString, Stateful, div,
    prelude::*,
};
use qc_file_tree::{FacetChoice, FacetCount};

use super::chip_click::ChipClick;
use super::pull_request_detail::PullRequestDetail;
use crate::theme;

/// Type chips shown before "more"; picked or hidden ones always show.
const COLLAPSED_TYPE_CHIPS: usize = 8;
/// Width of a chip row's label column, so both rows' chips line up.
const ROW_LABEL_WIDTH: f32 = 34.;

impl PullRequestDetail {
    /// The filter under the progress bar: the viewed switch, the kind and
    /// type chip rows with live counts, and a line saying what is shown.
    pub(super) fn render_filter_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(self.render_hide_viewed_switch(cx))
            .child(self.render_kind_row(cx))
            .child(self.render_type_row(cx))
            .child(self.render_filter_summary(cx))
    }

    /// `☐ Hide viewed (12)`: hides viewed files on top of the chips.
    fn render_hide_viewed_switch(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_on = self.file_tree().filter().hide_viewed;
        let viewed = self.file_review().viewed_count();
        div()
            .id("hide-viewed")
            .flex()
            .items_center()
            .gap_1p5()
            .text_xs()
            .cursor_pointer()
            .text_color(theme::muted())
            .hover(|style| style.text_color(theme::text()))
            .on_click(cx.listener(|detail, _, _, cx| detail.toggle_hide_viewed(cx)))
            .child(if is_on { "☑" } else { "☐" })
            .child(format!("Hide viewed ({viewed})"))
    }

    /// The kind chips: Code, Tests, Generated, Moved (those present).
    fn render_kind_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let filter = self.file_tree().filter();
        let chips: Vec<AnyElement> = self
            .file_tree()
            .kind_counts()
            .iter()
            .map(|count| {
                let kind = count.value;
                render_chip(
                    format!("kind-{}", kind.label()).into(),
                    kind.label(),
                    count,
                    filter.kinds.choice_of(&kind),
                )
                .on_click(cx.listener(move |detail, event: &ClickEvent, _, cx| {
                    detail.click_kind(kind, chip_click_of(event), cx);
                }))
                .into_any_element()
            })
            .collect();
        render_chip_row("Kind", chips)
    }

    /// The type chips, most files first: the most common, any picked or
    /// hidden, and a "more" chip for the rest.
    fn render_type_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let filter = self.file_tree().filter();
        let counts = self.file_tree().type_counts();
        let is_expanded = self.file_tree().are_types_expanded();

        // The most common, plus every chip that is set, so a set chip never hides behind "more".
        let shown: Vec<&FacetCount<String>> = counts
            .iter()
            .enumerate()
            .filter(|(index, count)| {
                is_expanded
                    || *index < COLLAPSED_TYPE_CHIPS
                    || filter.file_types.choice_of(&count.value) != FacetChoice::Open
            })
            .map(|(_, count)| count)
            .collect();
        let hidden_count = counts.len() - shown.len();
        let mut chips: Vec<AnyElement> = shown
            .into_iter()
            .map(|count| {
                let file_type = count.value.clone();
                render_chip(
                    format!("type-{file_type}").into(),
                    &count.value,
                    count,
                    filter.file_types.choice_of(&count.value),
                )
                .on_click(cx.listener(move |detail, event: &ClickEvent, _, cx| {
                    detail.click_file_type(&file_type, chip_click_of(event), cx);
                }))
                .into_any_element()
            })
            .collect();

        // The more / less chip, when there is more than the collapsed row holds.
        let more_label = match (is_expanded, hidden_count) {
            (true, _) if counts.len() > COLLAPSED_TYPE_CHIPS => Some("less".to_owned()),
            (false, 1..) => Some(format!("+{hidden_count} more")),
            (true | false, _) => None,
        };
        if let Some(label) = more_label {
            chips.push(
                div()
                    .id("type-more")
                    .px_1()
                    .text_xs()
                    .text_color(theme::accent())
                    .cursor_pointer()
                    .on_click(cx.listener(|detail, _, _, cx| detail.toggle_types_expanded(cx)))
                    .child(label)
                    .into_any_element(),
            );
        }
        render_chip_row("Type", chips)
    }

    /// `Showing 312 of 1390 · Moved · Reset` while anything narrows the
    /// files; otherwise how the chips work.
    fn render_filter_summary(&self, cx: &mut Context<Self>) -> AnyElement {
        let filter = self.file_tree().filter();
        let parts = filter.describe();
        if parts.is_empty() {
            return div()
                .text_xs()
                .text_color(theme::muted())
                .child("Click a chip to show only it · ⌥-click to hide it")
                .into_any_element();
        }
        let (shown, total) = self.file_tree().shown_of_total();
        div()
            .flex()
            .gap_1p5()
            .text_xs()
            .text_color(theme::muted())
            .child(div().flex_1().min_w_0().truncate().child(format!(
                "Showing {shown} of {total} · {}",
                parts.join(" · ")
            )))
            .when(filter.has_chips_set(), |summary| {
                summary.child(
                    div()
                        .id("filter-reset")
                        .flex_none()
                        .text_color(theme::accent())
                        .cursor_pointer()
                        .on_click(cx.listener(|detail, _, _, cx| detail.clear_filter_chips(cx)))
                        .child("Reset"),
                )
            })
            .into_any_element()
    }
}

/// A plain click picks; an option-click hides.
fn chip_click_of(event: &ClickEvent) -> ChipClick {
    if event.modifiers().alt {
        ChipClick::Hide
    } else {
        ChipClick::Pick
    }
}

/// A row of chips after its label.
fn render_chip_row(label: &'static str, chips: Vec<AnyElement>) -> Div {
    div()
        .flex()
        .gap_1()
        .child(
            div()
                .flex_none()
                .w(gpui::px(ROW_LABEL_WIDTH))
                .pt_0p5()
                .text_xs()
                .text_color(theme::muted())
                .child(label),
        )
        .child(div().flex_1().flex().flex_wrap().gap_1().children(chips))
}

/// One chip with what picking it would show: picked chips stand out, a
/// hidden one is struck through, and one that would show nothing is dimmed.
fn render_chip<T>(
    id: SharedString,
    label: &str,
    count: &FacetCount<T>,
    choice: FacetChoice,
) -> Stateful<Div> {
    let number = match choice {
        FacetChoice::Hidden => count.total,
        FacetChoice::Open | FacetChoice::Only => count.matching,
    };
    let chip = div()
        .id(id)
        .flex()
        .gap_1()
        .px_1p5()
        .rounded_md()
        .border_1()
        .text_xs()
        .cursor_pointer()
        .child(label.to_owned())
        .child(div().text_color(theme::muted()).child(number.to_string()));
    match choice {
        FacetChoice::Only => chip
            .border_color(theme::accent())
            .bg(theme::selected())
            .text_color(theme::text())
            .font_weight(FontWeight::SEMIBOLD),
        FacetChoice::Hidden => chip
            .border_color(theme::border())
            .text_color(theme::muted())
            .line_through(),
        FacetChoice::Open => chip
            .border_color(theme::border())
            .text_color(theme::text())
            .hover(|style| style.bg(theme::hover()))
            .when(count.matching == 0, |chip| chip.opacity(0.45)),
    }
}
