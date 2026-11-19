use gpui::px;

use super::cached_review::{cached_services, pull_request};
use crate::pull_request_detail::PullRequestDetail;
use crate::pull_request_detail::changed_files::ChangedFiles;
use crate::pull_request_detail::loaded_file::LoadedFile;
use crate::pull_request_detail::scroll_anchor::ScrollAnchor;

/// Navigating a file's matches clears the sticky header even after a very tall wrapped row.
#[gpui::test]
fn diff_find_reveals_matches_after_wrapped_rows(cx: &mut gpui::TestAppContext) {
    use super::super::file_direction::FileDirection;
    use qd_cache::{DiffLayout, LineWrapping};
    use qd_github::PullRequestFile;

    // Place two occurrences after a line taller than the whole viewport.
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let file = PullRequestFile {
        filename: "a.txt".into(),
        previous_filename: None,
        status: "added".into(),
        additions: 103,
        deletions: 0,
        patch: Some(format!(
            "@@ -0,0 +1,103 @@\n+{}\n+first hit\n+second hit\n{}",
            "tall ".repeat(3000),
            "+tail\n".repeat(100)
        )),
        sha: None,
    };
    let (pane, visual) = cx.add_window_view(|_, cx| {
        let mut detail = PullRequestDetail::new(services, cx);
        detail.pull_request = Some(pull_request(1, "version"));
        detail.diff_settings.layout = DiffLayout::Unified;
        detail.diff_settings.wrapping = LineWrapping::Wrap;
        detail.changed_files = Some(ChangedFiles::Loaded(vec![LoadedFile::build(
            file,
            detail.diff_settings.diff_options(),
        )]));
        detail.rebuild_rows(ScrollAnchor::Top);
        detail
    });
    visual.update(|window, app| pane.update(app, |detail, cx| detail.open_diff_find(window, cx)));
    visual.simulate_input("hit");
    visual.run_until_parked();

    // Check the actual drawn row after forward navigation and a wrap back from the first result.
    let check_visible = |expected: usize, visual: &mut gpui::VisualTestContext| {
        visual.update(|_, app| {
            let detail = pane.read(app);
            let find = detail.diff_find.as_ref().unwrap();
            assert_eq!(find.current, expected);
            assert_eq!(find.matches.occurrences.len(), 2);
            let row = detail.rows().file_start(find.file).unwrap()
                + find.matches.occurrences[expected].row;
            let viewport = detail.list().viewport_bounds();
            let bounds = detail.list().bounds_for_item(row).unwrap();
            assert!(
                bounds.top() >= viewport.top() + px(38.),
                "{bounds:?} in {viewport:?}"
            );
            assert!(
                bounds.bottom() <= viewport.bottom(),
                "{bounds:?} in {viewport:?}"
            );
        });
    };
    check_visible(0, visual);
    visual.update(|window, app| {
        pane.update(app, |detail, cx| {
            detail.move_diff_match(FileDirection::Previous, window, cx);
        });
    });
    visual.run_until_parked();
    check_visible(1, visual);
}

/// The native keymap opens the diff search field from queue focus and Escape returns to the diff.
#[gpui::test]
fn find_shortcut_focuses_and_dismisses_the_search_field(cx: &mut gpui::TestAppContext) {
    use crate::app_commands::install_app_commands;
    use crate::app_update::AppUpdater;
    use crate::review_queue::ReviewQueue;
    use qd_cache::{FileSetRevision, PullRequestListKey};
    use qd_github::{PullRequestFile, PullRequestRevision};

    // Open a queue with one cached patch and leave keyboard focus on the queue.
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let pull_request = pull_request(1, "version");
    let file = PullRequestFile {
        filename: "a.rs".into(),
        previous_filename: None,
        status: "added".into(),
        additions: 2,
        deletions: 0,
        patch: Some("@@ -0,0 +1,2 @@\n+value\n+value".into()),
        sha: None,
    };
    let revision = FileSetRevision {
        commits: PullRequestRevision {
            head_sha: "head".into(),
            base_sha: "base".into(),
        },
        etag: None,
    };
    {
        let mut cache = services.cache.lock();
        cache
            .replace_list(
                PullRequestListKey::AUTHORED,
                std::slice::from_ref(&pull_request),
            )
            .unwrap();
        cache.put_files(&pull_request, &[file], &revision).unwrap();
    }
    cx.update(|app| {
        install_app_commands(app);
        AppUpdater::start(app);
    });
    let window = cx.add_window(|window, cx| ReviewQueue::new(services, window, cx));
    cx.run_until_parked();
    let mut visual = gpui::VisualTestContext::from_window(window.into(), cx);
    let find_focused = |visual: &mut gpui::VisualTestContext| {
        visual.update(|window, _| {
            window
                .context_stack()
                .iter()
                .any(|context| context.contains("DiffFind"))
        })
    };

    // Single-letter review shortcuts must type into Find, and Shift-Return must stay in the field.
    visual.simulate_keystrokes("cmd-f");
    assert!(find_focused(&mut visual));
    visual.simulate_keystrokes("v a l u e");
    visual.simulate_keystrokes("enter shift-enter");
    assert!(find_focused(&mut visual));

    // Reopening Find selects the query for replacement.
    visual.simulate_keystrokes("cmd-f cmd-c");
    let copied = cx.read(|app| app.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some("value"));
    visual.simulate_keystrokes("escape");
    assert!(!find_focused(&mut visual));
}
