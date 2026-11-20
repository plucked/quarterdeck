use std::time::Duration;

use gpui::{
    AppContext as _, Context, Entity, IntoElement, Modifiers, Pixels, Render, TestAppContext,
    VisualTestContext, Window, div, point, prelude::*, px, size,
};
use qd_github::PullRequestBranches;

use super::cached_review::{cached_services, pull_request};
use crate::pull_request_detail::PullRequestDetail;

/// A title wider than its available space fills the space before and during hover.
#[gpui::test]
fn long_title_uses_its_slot_at_rest_and_on_hover(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let mut request = pull_request(1, "version");
    request.title =
        "feat: switch the transport to the new backend and keep settings working ".repeat(2);
    request.branches = Some(PullRequestBranches {
        head_ref: "feat/switch-transport-and-keep-settings".into(),
        base_ref: "main".into(),
    });
    let (_, visual) = cx.add_window_view(|_, cx| {
        HeaderTestView(cx.new(|cx| {
            let mut detail = PullRequestDetail::new(services, cx);
            detail.pull_request = Some(request);
            detail
        }))
    });
    visual.simulate_resize(size(px(4200.), px(700.)));
    visual.run_until_parked();
    let full_width = visual
        .debug_bounds("pull-request-title-1-line")
        .unwrap()
        .size
        .width;
    visual.simulate_resize(size(px(1000.), px(700.)));
    visual.run_until_parked();
    let preview = visual.debug_bounds("pull-request-title-1-line").unwrap();
    let slot = visual.debug_bounds("review-header-title-space").unwrap();
    assert_eq!(preview.size.width, slot.size.width);

    visual.simulate_mouse_move(preview.center(), None, Modifiers::none());
    finish_hover_animation(visual);
    let expanded = visual.debug_bounds("pull-request-title-1-line").unwrap();
    let expanded_slot = visual.debug_bounds("review-header-title-space").unwrap();
    assert!(expanded.size.width > preview.size.width);
    assert_eq!(expanded.size.width, full_width);
    assert!(expanded.size.width <= expanded_slot.size.width);
}

/// Gives the detail pane the same width constraint as the real review window.
struct HeaderTestView(Entity<PullRequestDetail>);

impl Render for HeaderTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().child(self.0.clone())
    }
}

/// Hovering either field frees the tabs' width, and clicking the title opens the pull request.
#[gpui::test]
fn header_fields_expand_and_restore_the_tabs(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let url = "https://github.com/owner/repo/pull/1";
    let mut request = pull_request(1, "version");
    request.html_url = url.into();
    request.branches = Some(PullRequestBranches {
        head_ref: "feat/example-with-a-long-branch-name".into(),
        base_ref: "main".into(),
    });
    let (_, visual) = cx.add_window_view(|_, cx| {
        let mut detail = PullRequestDetail::new(services, cx);
        detail.pull_request = Some(request);
        detail
    });
    visual.run_until_parked();
    let preview = visual.debug_bounds("pull-request-title-link").unwrap();
    let tabs = visual.debug_bounds("review-header-tabs").unwrap();

    // The short title already fits, so any extra width comes from the byline.
    visual.simulate_mouse_move(preview.center(), None, Modifiers::none());
    finish_hover_animation(visual);
    let expanded = visual.debug_bounds("pull-request-title-link").unwrap();
    assert!(expanded.size.width > preview.size.width);
    assert_eq!(
        visual
            .debug_bounds("review-header-tabs")
            .unwrap()
            .size
            .width,
        Pixels::ZERO
    );

    visual.simulate_click(preview.center(), Modifiers::none());
    assert_eq!(visual.opened_url().as_deref(), Some(url));

    // Leaving the header restores the tabs before the branch takes their space.
    visual.simulate_mouse_move(point(px(1.), px(100.)), None, Modifiers::none());
    finish_hover_animation(visual);
    assert_eq!(visual.debug_bounds("review-header-tabs"), Some(tabs));

    let branch = visual.debug_bounds("pull-request-branch-preview").unwrap();
    visual.simulate_mouse_move(branch.center(), None, Modifiers::none());
    finish_hover_animation(visual);
    assert!(
        visual
            .debug_bounds("pull-request-branch-preview")
            .unwrap()
            .size
            .width
            > branch.size.width
    );
    assert_eq!(
        visual
            .debug_bounds("review-header-tabs")
            .unwrap()
            .size
            .width,
        Pixels::ZERO
    );
}

/// Draws the settled end of the hover animation.
fn finish_hover_animation(visual: &mut VisualTestContext) {
    std::thread::sleep(Duration::from_millis(250));
    visual.update(|window, app| window.draw(app).clear());
}
