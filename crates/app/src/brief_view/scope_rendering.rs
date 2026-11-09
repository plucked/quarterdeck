use std::sync::Arc;

use gpui::{AnyElement, FontWeight, IntoElement, div, prelude::*, px};
use qd_markdown::MarkdownDocument;

use super::brief_rendering::markdown;
use crate::theme;

/// The narrowest a scope list gets before the lists stack.
const SCOPE_GROUP_MIN_WIDTH: f32 = 220.;

/// The scope: up to three labelled lists (what changes, what stays the
/// same, what is mechanical and can be skimmed) side by side when there is
/// room; empty lists are left out.
pub fn render_scope(
    widget: usize,
    changes: &[Arc<MarkdownDocument>],
    unchanged: &[Arc<MarkdownDocument>],
    mechanical: &[Arc<MarkdownDocument>],
) -> AnyElement {
    let groups = [
        ("What changes", "changes", changes),
        ("What stays the same", "unchanged", unchanged),
        ("Mechanical (skim)", "mechanical", mechanical),
    ];
    div()
        .flex()
        .flex_wrap()
        .gap_3()
        .children(
            groups
                .into_iter()
                .filter(|(_, _, items)| !items.is_empty())
                .map(|(label, part, items)| {
                    div()
                        .flex_1()
                        .min_w(px(SCOPE_GROUP_MIN_WIDTH))
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .p_3()
                        .rounded_md()
                        .bg(theme::surface())
                        .border_1()
                        .border_color(theme::border())
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme::subtle())
                                .child(label.to_uppercase()),
                        )
                        .children(items.iter().enumerate().map(move |(index, item)| {
                            div()
                                .flex()
                                .gap_2()
                                .child(div().flex_none().text_color(theme::subtle()).child("•"))
                                .child(div().flex_1().min_w_0().child(markdown(
                                    format!("brief-{widget}-{part}-{index}"),
                                    item,
                                )))
                        }))
                }),
        )
        .into_any_element()
}
