use gpui::{AnyElement, ClickEvent, Context, FontWeight, IntoElement, div, prelude::*, px};
use qd_cache::PanelVisibility;

use super::ReviewQueue;
use super::panel_toggle::{PanelEdge, PanelToggle};
use crate::app_update::AppUpdater;
use crate::controls::Button;
use crate::pull_request_detail::{DetailModeSwitch, PullRequestDetail};
use crate::theme;

/// Height of the bar; the window's title bar is drawn by the app, so the
/// macOS traffic lights sit inside it (`launch_app` places them).
pub const TOP_BAR_HEIGHT: f32 = 40.;
/// Room left of the bar's content for the traffic lights.
const TRAFFIC_LIGHTS_WIDTH: f32 = 84.;
/// The name in the middle of the bar while no pull request shows.
const APP_TITLE: &str = "Quarterdeck";

impl ReviewQueue {
    /// The bar across the top of the window, in place of the system title
    /// bar: the sidebar toggle and refresh on the left, the shown pull
    /// request's view tabs in the middle (the app's name without one), and on
    /// the right the restart into a ready update (when there is one) and the
    /// side panel toggle. Its empty parts drag the window; a double click
    /// zooms it.
    pub(super) fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_side_panel_shown = self.detail().read(cx).is_side_panel_shown();
        let has_pull_request = self.detail().read(cx).has_pull_request();
        let updater = AppUpdater::global(cx);
        let ready_version = updater
            .read(cx)
            .ready_update()
            .map(|ready| ready.release.version);
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
            // Under the buttons and across the whole bar, so its content is
            // centered on the window, not on the room between the buttons.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child(self.render_top_bar_middle(has_pull_request)),
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
            .children(ready_version.map(|version| {
                Button::new(
                    "restart-to-update",
                    format!("Restart to update to {version}"),
                )
                .primary()
                .on_click(move |_, _, cx| updater.update(cx, AppUpdater::restart_to_update))
            }))
            .child(
                PanelToggle::new("toggle-side-panel", PanelEdge::Right, is_side_panel_shown)
                    .disabled(!has_pull_request)
                    .on_click(cx.listener(|queue, _, _, cx| {
                        queue
                            .detail()
                            .update(cx, PullRequestDetail::toggle_file_tree);
                    })),
            )
    }

    /// The middle of the bar: the view tabs of the shown pull request, else
    /// the app's name.
    fn render_top_bar_middle(&self, has_pull_request: bool) -> AnyElement {
        if has_pull_request {
            return DetailModeSwitch::new(self.detail().clone()).into_any_element();
        }
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme::muted())
            .child(APP_TITLE)
            .into_any_element()
    }
}
