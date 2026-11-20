use gpui::{Modifiers, TestAppContext, VisualTestContext};
use qd_cache::PullRequestListKey;

use super::cached_review::{cached_services, pull_request};
use crate::app_commands::install_app_commands;
use crate::app_update::AppUpdater;
use crate::review_queue::ReviewQueue;

/// Focus Mode expands the active pane and restores its original layout on exit.
#[gpui::test]
fn focus_mode_uses_the_window_and_restores_the_panels(cx: &mut TestAppContext) {
    // Open the review queue with a pull request from the offline cache.
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    services
        .cache
        .lock()
        .replace_list(PullRequestListKey::AUTHORED, &[pull_request(1, "version")])
        .unwrap();
    cx.update(|app| {
        install_app_commands(app);
        AppUpdater::start(app);
    });
    let window = cx.add_window(|window, cx| ReviewQueue::new(services, window, cx));
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    let original = visual.debug_bounds("review-main-column").unwrap();
    let original_content = visual.debug_bounds("review-content").unwrap();
    assert!(original_content.top() > original.top());
    let toolbar_button = visual.debug_bounds("focus-mode-toggle").unwrap();
    assert!(toolbar_button.bottom() <= original.top());

    // Use the hotkey to expand the pane, then use the exit button.
    visual.simulate_keystrokes("cmd-shift-f");
    let focused = visual.debug_bounds("review-main-column").unwrap();
    assert!(focused.size.width > original.size.width);
    assert_eq!(visual.debug_bounds("review-content"), Some(focused));
    let exit_button = visual.debug_bounds("focus-mode-toggle").unwrap();
    visual.simulate_click(exit_button.center(), Modifiers::none());

    assert_eq!(visual.debug_bounds("review-main-column"), Some(original));
    assert_eq!(
        visual.debug_bounds("review-content"),
        Some(original_content)
    );

    // The compact toolbar button also enters Focus Mode, and the hotkey exits it.
    visual.simulate_click(toolbar_button.center(), Modifiers::none());
    assert_eq!(visual.debug_bounds("review-main-column"), Some(focused));
    visual.simulate_keystrokes("cmd-shift-f");
    assert_eq!(visual.debug_bounds("review-main-column"), Some(original));
}
