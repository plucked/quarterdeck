use gpui::{AnyElement, Context, FontWeight, IntoElement, div, prelude::*, px, svg};

use super::file_tree_rendering::render_file_icon;
use super::pull_request_detail::PullRequestDetail;
use super::walkthrough_item_id::WalkthroughItemId;
use crate::controls::{CheckState, Checkbox, NumberFlow, TextTooltip};
use crate::theme;

impl PullRequestDetail {
    /// Icons distinguish files from groups; guidance wraps within the same review stop.
    pub(super) fn render_walkthrough_item(
        &self,
        id: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.walkthrough.item(id) else {
            return div().into_any_element();
        };
        let is_group = item.files.len() > 1;
        let (label, subtitle, icon) = self.walkthrough_item_identity(item);
        div()
            .id(gpui::SharedString::from(format!(
                "walkthrough-item-{}-{}",
                id.bucket, id.item
            )))
            .w_full()
            .px_3()
            .py_3()
            .flex()
            .flex_col()
            .gap_2()
            .border_t_1()
            .border_color(theme::border())
            .when(is_group, |row| row.bg(theme::panel()))
            .when(self.walkthrough.selected_group == Some(id), |row| {
                row.bg(theme::selected())
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()))
            .on_click(cx.listener(move |detail, _, _, cx| detail.select_walkthrough_item(id, cx)))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap_1()
                    .child(
                        div()
                            .flex_none()
                            .w(px(16.))
                            .pt(px(2.))
                            .text_size(px(10.))
                            .text_color(theme::subtle())
                            .child(NumberFlow::label((id.item + 1).to_string())),
                    )
                    .child(div().flex_none().pt(px(1.)).child(icon))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_xs()
                            .child(
                                div()
                                    .id("item-label")
                                    .font_weight(FontWeight::MEDIUM)
                                    .when(!is_group, gpui::Styled::truncate)
                                    .tooltip(TextTooltip::builder(label.clone()))
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::subtle())
                                    .when(!is_group, gpui::Styled::truncate)
                                    .child(subtitle),
                            ),
                    )
                    .child(
                        div()
                            .id("item-viewed")
                            .flex_none()
                            .p_1()
                            .tooltip(TextTooltip::builder(format!(
                                "Toggle all {} files viewed, including hidden files",
                                item.files.len()
                            )))
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                cx.stop_propagation();
                                detail.toggle_walkthrough_viewed(id, cx);
                            }))
                            .child(Checkbox::new(progress_check(item.viewed, item.files.len()))),
                    ),
            )
            .when(!item.description.is_empty(), |row| {
                row.child(
                    div()
                        .pl(px(20.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(theme::muted())
                        .child(item.description.clone()),
                )
            })
            .into_any_element()
    }
    /// Single files keep their path identity; groups name their complete membership.
    fn walkthrough_item_identity(
        &self,
        item: &super::walkthrough_item::WalkthroughItem,
    ) -> (String, String, AnyElement) {
        match item.files.as_slice() {
            [file] => {
                let loaded = &self.loaded_files()[*file];
                let path = loaded.filename();
                (
                    path.rsplit('/').next().unwrap_or_default().to_owned(),
                    path.rsplit_once('/').map_or("", |parts| parts.0).to_owned(),
                    render_file_icon(loaded.facets.icon, item.viewed == 1).into_any_element(),
                )
            }
            _ => (
                item.label.clone(),
                format!(
                    "File group · {} files{}",
                    item.files.len(),
                    if item.shown < item.files.len() {
                        " · some hidden"
                    } else {
                        ""
                    }
                ),
                render_group_icon().into_any_element(),
            ),
        }
    }
}

/// A stacked-file glyph is shared by grouped stops and their overview heading.
pub(super) fn render_group_icon() -> impl IntoElement {
    div().flex_none().w(px(24.)).flex().justify_center().child(
        svg()
            .path("icons/file-group.svg")
            .size(px(16.))
            .text_color(theme::accent()),
    )
}

/// Shared tri-state meaning for complete grouped membership.
fn progress_check(viewed: usize, total: usize) -> CheckState {
    if viewed == 0 {
        CheckState::Unchecked
    } else if viewed == total {
        CheckState::Checked
    } else {
        CheckState::Mixed
    }
}
