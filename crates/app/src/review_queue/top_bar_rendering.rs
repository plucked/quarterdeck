use gpui::{ClickEvent, Context, FontWeight, IntoElement, div, prelude::*, px, relative};
use qd_cache::PanelVisibility;

use super::ReviewQueue;
use super::panel_toggle::{PanelEdge, PanelToggle};
use crate::app_update::AppUpdater;
use crate::controls::Button;
use crate::pull_request_detail::PullRequestDetail;
use crate::theme;

/// Height of one native macOS traffic light.
pub const TRAFFIC_LIGHT_HEIGHT: f32 = 14.;
/// Height of the bar, with 7 px above and below the traffic lights.
pub const TOP_BAR_HEIGHT: f32 = TRAFFIC_LIGHT_HEIGHT + 14.;
/// Room left of the bar's content for the traffic lights.
const TRAFFIC_LIGHTS_WIDTH: f32 = 84.;
/// The name in the middle of the bar.
const APP_TITLE: &str = "Quarterdeck";

impl ReviewQueue {
    /// The bar across the top of the window, in place of the system title
    /// bar: the sidebar toggle and refresh on the left, the app's name in the
    /// middle, and on the right the restart into a ready update (when there
    /// is one) and the side panel toggle. Its empty parts drag the window.
    /// A double click zooms it.
    pub(super) fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let detail = self.detail().read(cx);
        let is_side_panel_shown = detail.is_side_panel_shown();
        let has_pull_request = detail.has_pull_request();
        let activity = detail.render_activity_indicator();
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
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .line_height(px(TRAFFIC_LIGHT_HEIGHT))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::muted())
                    .child(
                        div()
                            .relative()
                            .child(APP_TITLE)
                            .children(activity.map(|indicator| {
                                // The app name stays centered as the indicator comes and goes.
                                div()
                                    .absolute()
                                    .left(relative(1.))
                                    .top_0()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .ml_1()
                                    .child(indicator)
                            })),
                    ),
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
                    .compact()
                    .on_click(cx.listener(|queue, _, _, cx| queue.refresh(cx))),
            )
            .child(div().flex_1())
            .children(ready_version.map(|version| {
                Button::new(
                    "restart-to-update",
                    format!("Restart to update to {version}"),
                )
                .primary()
                .compact()
                .on_click(move |_, _, cx| updater.update(cx, AppUpdater::restart_to_update))
            }))
            .child(Self::render_focus_mode_toggle(false, !has_pull_request, cx))
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
}
