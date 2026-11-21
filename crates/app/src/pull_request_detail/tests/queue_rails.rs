use gpui::{Modifiers, TestAppContext, VisualTestContext};
use qd_cache::PullRequestListKey;
use qd_github::{PullRequest, PullRequestBranches, PullRequestRevision};

use super::cached_review::{cached_services, pull_request};
use crate::app_commands::install_app_commands;
use crate::app_update::AppUpdater;
use crate::review_queue::ReviewQueue;

fn request(number: u64, head: &str, base: &str) -> PullRequest {
    let mut request = pull_request(number, "version");
    request.branches = Some(PullRequestBranches {
        head_ref: head.into(),
        base_ref: base.into(),
        head_repository: Some(request.repo.full_name()),
        default_branch: Some("main".into()),
        revision: Some(PullRequestRevision {
            head_sha: head.into(),
            base_sha: base.into(),
        }),
    });
    request
}

/// Exercise a cached launch through the normal queue and GPUI hit testing.
#[gpui::test]
fn linear_queue_aligns_titles_and_opens_the_next_layer(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let mut visual = queue_window(
        cx,
        directory.path(),
        &[
            request(1, "a", "main"),
            request(2, "b", "a"),
            request(3, "c", "b"),
        ],
    );
    let first = visual.debug_bounds("queue-layer-title-1").unwrap();
    let second = visual.debug_bounds("queue-layer-title-2").unwrap();
    let third = visual.debug_bounds("queue-layer-title-3").unwrap();
    assert_eq!(first.left(), second.left());
    assert_eq!(second.left(), third.left());
    assert!(first.top() < second.top() && second.top() < third.top());
    visual.simulate_click(first.center(), Modifiers::none());
    assert!(visual.debug_bounds("pull-request-title-1-line").is_some());
    visual.simulate_keystrokes("}");
    assert!(visual.debug_bounds("pull-request-title-2-line").is_some());
}

/// A fork indents both siblings equally, nests their descendants and can be reviewed by click.
#[gpui::test]
fn forked_queue_aligns_siblings_and_selects_the_other_tip(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let mut visual = queue_window(
        cx,
        directory.path(),
        &[
            request(1, "a", "main"),
            request(2, "b", "a"),
            request(3, "c", "b"),
            request(4, "d", "a"),
        ],
    );
    let root = visual.debug_bounds("queue-layer-title-1").unwrap();
    let child = visual.debug_bounds("queue-layer-title-2").unwrap();
    let grandchild = visual.debug_bounds("queue-layer-title-3").unwrap();
    let sibling = visual.debug_bounds("queue-layer-title-4").unwrap();
    assert_eq!(child.left(), sibling.left());
    assert!(root.left() < child.left() && child.left() < grandchild.left());
    visual.simulate_click(sibling.center(), Modifiers::none());
    assert!(visual.debug_bounds("pull-request-title-4-line").is_some());
}

/// An offline queue with the fixture's real SQLite round trip and normal launch selection.
fn queue_window(
    cx: &mut TestAppContext,
    directory: &std::path::Path,
    requests: &[PullRequest],
) -> VisualTestContext {
    let services = cached_services(cx, directory);
    services
        .cache
        .lock()
        .replace_list(PullRequestListKey::AUTHORED, requests)
        .unwrap();
    cx.update(|app| {
        install_app_commands(app);
        AppUpdater::start(app);
    });
    let window = cx.add_window(|window, cx| ReviewQueue::new(services, window, cx));
    cx.run_until_parked();
    VisualTestContext::from_window(window.into(), cx)
}
