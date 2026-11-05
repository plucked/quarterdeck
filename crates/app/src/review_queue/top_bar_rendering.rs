use gpui::{ClickEvent, Context, FontWeight, IntoElement, div, prelude::*, px};
use qc_cache::PanelVisibility;

use super::ReviewQueue;
use crate::button::Button;
use crate::panel_toggle::{PanelEdge, PanelToggle};
use crate::pull_request_detail::PullRequestDetail;
use crate::theme;

/// Height of the bar; the window's title bar is drawn by the app, so the
/// macOS traffic lights sit inside it (`launch_app` places them).
pub const TOP_BAR_HEIGHT: f32 = 40.;
/// Room left of the bar's content for the traffic lights.
const TRAFFIC_LIGHTS_WIDTH: f32 = 84.;
/// The name in the middle of the bar.
const APP_TITLE: &str = "Quick Check";

impl ReviewQueue {
    /// The bar across the top of the window, in place of the system title
    /// bar: the sidebar toggle and refresh on the left, the app's name in the middle, and the side panel toggle on the
    /// right. Its empty parts drag the window; a double click zooms it.
    pub(super) fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_side_panel_shown = self.detail().read(cx).is_side_panel_shown();
        let has_side_panel = self.detail().read(cx).has_pull_request();
        div()
            .id("top-bar")
            .relative()
            .flex_none()
            .h(px(TOP_BAR_HEIGHT))
            .flex()
            .items_center()
            .gap_2()
            .pl(px(TRAFFIC_LIGHTS_WIDTH))
            .pr_2()
            .bg(theme::sidebar())
            .border_b_1()
            .border_color(theme::border())
            // A double click on an empty part does what the system title bar would.
            .on_click(|event: &ClickEvent, window, _| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::muted())
                    .child(APP_TITLE),
            )
            .child(
                PanelToggle::new(
                    "toggle-sidebar",
                    PanelEdge::Left,
                    self.sidebar_visibility() == PanelVisibility::Shown,
                )
                .on_click(cx.listener(|queue, _, _, cx| queue.toggle_sidebar(cx))),
            )
            .child(
                Button::new("refresh", "↻ Refresh")
                    .ghost()
                    .on_click(cx.listener(|queue, _, _, cx| queue.refresh(cx))),
            )
            .child(div().flex_1())
            .child(
                PanelToggle::new("toggle-side-panel", PanelEdge::Right, is_side_panel_shown)
                    .disabled(!has_side_panel)
                    .on_click(cx.listener(|queue, _, _, cx| {
                        queue
                            .detail()
                            .update(cx, PullRequestDetail::toggle_file_tree);
                    })),
            )
    }
}
