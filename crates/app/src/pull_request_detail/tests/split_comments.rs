use gpui::{Context, Modifiers, MouseButton, point, px};
use qd_github::LineSide;

use super::cached_review::{cached_services, pull_request};
use crate::app_services::AppServices;
use crate::pull_request_detail::PullRequestDetail;
use crate::pull_request_detail::changed_files::ChangedFiles;
use crate::pull_request_detail::comment_range::CommentRange;
use crate::pull_request_detail::loaded_file::LoadedFile;
use crate::pull_request_detail::scroll_anchor::ScrollAnchor;

/// Different line numbers on the two sides make a misplaced comment anchor visible to the test.
fn split_comment_fixture(
    services: AppServices,
    patch: &str,
    whitespace: qd_diff::WhitespaceMode,
    cx: &mut Context<PullRequestDetail>,
) -> PullRequestDetail {
    use qd_cache::DiffLayout;
    use qd_github::PullRequestFile;

    let mut detail = PullRequestDetail::new(services, cx);
    detail.pull_request = Some(pull_request(1, "version"));
    detail.diff_settings.layout = DiffLayout::Split;
    detail.diff_settings.whitespace = whitespace;
    let file = PullRequestFile {
        filename: "file.txt".into(),
        previous_filename: None,
        status: "modified".into(),
        additions: 2,
        deletions: 2,
        patch: Some(patch.into()),
        sha: None,
    };
    let loaded = LoadedFile::build(file, detail.diff_settings.diff_options());
    detail.changed_files = Some(ChangedFiles::Loaded(vec![loaded]));
    detail.rebuild_rows(ScrollAnchor::Top);
    detail
}

/// Split gutter clicks, drag selection and Shift-extension all reach the same real composer.
#[gpui::test]
fn split_comment_buttons_target_their_lines_and_keep_ranges(cx: &mut gpui::TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let patch =
        "@@ -10,3 +20,3 @@\n shared context\n-old first\n-old second\n+new first\n+new second";
    let (pane, visual) = cx.add_window_view(|_, cx| {
        split_comment_fixture(services, patch, qd_diff::WhitespaceMode::Show, cx)
    });
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    // Both mirrored context buttons use the right-side coordinate; a deletion uses the old one.
    click("comment-old-line-3", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::New, None, 20, visual);
    click("comment-old-line-5", shift, visual);
    expect_anchor(&pane, LineSide::Old, Some(10), 11, visual);
    visual.update(|window, app| pane.update(app, |detail, cx| detail.close_editor(window, cx)));
    click("comment-old-line-4", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::Old, None, 11, visual);
    visual.update(|window, app| pane.update(app, |detail, cx| detail.close_editor(window, cx)));

    // Drag across two painted code cells and invoke the visible range action.
    drag("comment-new-line-4", "comment-new-line-5", visual);
    click("selected-lines-comment", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::New, Some(21), 22, visual);

    // A deleted line has no coordinate in this added range. Keep the editable draft on its original lines.
    click("comment-old-line-4", shift, visual);
    expect_anchor(&pane, LineSide::New, Some(21), 22, visual);

    // Shift-click extends the same draft and refreshes replacement source without editing its body.
    visual.simulate_input("keep my draft");
    click("comment-new-line-3", shift, visual);
    expect_anchor(&pane, LineSide::New, Some(20), 22, visual);
    visual.update(|_, app| {
        let editor = pane.read(app).review.editor.as_ref().unwrap();
        assert_eq!(editor.text.read(app).text(), "keep my draft");
        assert_eq!(
            editor.suggestion_source.as_deref(),
            Some("shared context\nnew first\nnew second")
        );
    });

    visual.update(|window, app| pane.update(app, |detail, cx| detail.close_editor(window, cx)));
    drag("comment-old-line-4", "comment-old-line-5", visual);
    click("comment-old-line-5", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::Old, Some(11), 12, visual);

    // A written replacement keeps its original range through a subsequent Shift-click.
    visual.update(|window, app| pane.update(app, |detail, cx| detail.close_editor(window, cx)));
    click("comment-new-line-4", Modifiers::default(), visual);
    visual.update(|_, app| {
        let editor = pane.read(app).review.editor.as_ref().unwrap();
        let source = editor.suggestion_source.clone().unwrap();
        editor
            .text
            .clone()
            .update(app, |area, cx| area.insert_suggestion(&source, cx));
    });
    visual.simulate_input("replacement");
    click("comment-new-line-3", shift, visual);
    expect_anchor(&pane, LineSide::New, None, 21, visual);
    visual.update(|_, app| {
        let editor = pane.read(app).review.editor.as_ref().unwrap();
        assert_eq!(
            editor.text.read(app).text(),
            "```suggestion\nreplacement\n```"
        );
    });
}

/// Folded replacements look like shared context, but their raw patch contains two different lines.
#[gpui::test]
fn folded_replacements_keep_the_clicked_side(cx: &mut gpui::TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let patch = "@@ -10,3 +20,3 @@\n- let value = 1;\n+let value = 1;\n shared context\n-old value\n+new value";
    let (pane, visual) = cx.add_window_view(|_, cx| {
        let mut detail =
            split_comment_fixture(services, patch, qd_diff::WhitespaceMode::Ignore, cx);
        detail.diff_settings.double_click_to_comment = true;
        detail
    });
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    let close = |visual: &mut gpui::VisualTestContext| {
        visual.update(|window, app| pane.update(app, |detail, cx| detail.close_editor(window, cx)));
    };

    click("comment-old-line-3", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::Old, None, 10, visual);
    visual.update(|_, app| {
        assert!(
            pane.read(app)
                .review
                .editor
                .as_ref()
                .unwrap()
                .suggestion_source
                .is_none()
        );
    });
    visual.simulate_input("base-side note");

    // Neither a folded endpoint nor a folded existing anchor can move to the opposite side.
    click("comment-new-line-3", shift, visual);
    click("comment-new-line-6", shift, visual);
    expect_anchor(&pane, LineSide::Old, None, 10, visual);
    // Genuine context still translates to the side of the edited draft.
    click("comment-new-line-5", shift, visual);
    expect_anchor(&pane, LineSide::Old, Some(10), 11, visual);
    visual.update(|_, app| {
        assert_eq!(
            pane.read(app)
                .review
                .editor
                .as_ref()
                .unwrap()
                .text
                .read(app)
                .text(),
            "base-side note"
        );
    });
    close(visual);

    // Both split columns can drag across a folded replacement and genuine context.
    drag("comment-old-line-3", "comment-old-line-4", visual);
    click("selected-lines-comment", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::Old, Some(10), 11, visual);
    close(visual);
    drag("comment-new-line-3", "comment-new-line-4", visual);
    click("comment-new-line-3", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::New, Some(20), 21, visual);
    visual.update(|_, app| {
        assert_eq!(
            pane.read(app)
                .review
                .editor
                .as_ref()
                .unwrap()
                .suggestion_source
                .as_deref(),
            Some("let value = 1;\nshared context")
        );
    });
    close(visual);

    // A double click in the left cell and a gutter click in the right cell address the raw pair.
    let position =
        visual.debug_bounds("comment-old-line-3").unwrap().center() + point(px(80.), px(0.));
    visual.simulate_event(gpui::MouseDownEvent {
        position,
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: 2,
        first_mouse: false,
    });
    visual.simulate_event(gpui::MouseUpEvent {
        position,
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: 2,
    });
    expect_anchor(&pane, LineSide::Old, None, 10, visual);
    close(visual);
    click("comment-new-line-3", Modifiers::default(), visual);
    expect_anchor(&pane, LineSide::New, None, 20, visual);
}

/// Undoing a source-authored suggestion keeps its anchor fixed until it is restored with redo.
#[gpui::test]
fn markdown_suggestion_history_keeps_its_original_lines(cx: &mut gpui::TestAppContext) {
    cx.update(crate::app_commands::install_app_commands);
    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let patch =
        "@@ -10,3 +20,3 @@\n shared context\n-old first\n-old second\n+new first\n+new second";
    let (pane, visual) = cx.add_window_view(|_, cx| {
        let mut detail = split_comment_fixture(services, patch, qd_diff::WhitespaceMode::Show, cx);
        detail.set_comment_destination(qd_cache::NoteDestination::GitHub, cx);
        detail
    });

    // Start in Markdown exactly as a reviewer pasting an existing proposed change would.
    click("comment-new-line-4", Modifiers::default(), visual);
    click("Markdown", Modifiers::default(), visual);
    let source = "```suggestion\nreplacement\n```";
    visual.update(|_, app| app.write_to_clipboard(gpui::ClipboardItem::new_string(source.into())));
    visual.simulate_keystrokes("cmd-v cmd-z");
    visual.update(|_, app| {
        let editor = pane.read(app).review().editor.as_ref().unwrap();
        assert_eq!(editor.text.read(app).text(), "");
        assert!(editor.text.read(app).suggestion_anchors_range());
    });

    // A Shift-click while the proposal is undone must not move its replacement range.
    click(
        "comment-new-line-3",
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
        visual,
    );
    expect_anchor(&pane, LineSide::New, None, 21, visual);
    visual.update(|window, app| {
        pane.read(app)
            .review()
            .editor
            .as_ref()
            .unwrap()
            .text
            .read(app)
            .focus(window);
    });
    visual.simulate_keystrokes("cmd-shift-z");
    visual.update(|_, app| {
        let editor = pane.read(app).review().editor.as_ref().unwrap();
        assert_eq!(editor.text.read(app).text(), source);
    });
    expect_anchor(&pane, LineSide::New, None, 21, visual);
}

/// Saved local ranges export the original changes on their side and keep revealed context above.
#[gpui::test]
fn folded_ranges_save_and_export_original_patch_lines(cx: &mut gpui::TestAppContext) {
    use crate::ui_state::UiStateRecorder;
    use qd_cache::{NoteAnchor, NoteDestination, ReviewScope, UiStateStore};

    let directory = tempfile::tempdir().unwrap();
    let services = cached_services(cx, directory.path());
    let (pane, visual) = cx.add_window_view(|_, cx| folded_export_fixture(services, cx));

    // Save a real drag selection on each side, then copy both notes through the normal export.
    for (from, to, body) in [
        ("comment-old-line-4", "comment-old-line-6", "Base note"),
        ("comment-new-line-4", "comment-new-line-6", "Head note"),
    ] {
        drag(from, to, visual);
        click(from, Modifiers::default(), visual);
        visual.simulate_input(body);
        visual.update(|window, app| {
            pane.update(app, |detail, cx| {
                let editor = detail.review().editor.as_ref().expect(body);
                assert_eq!(editor.text.read(cx).text(), body);
                detail.save_editor(window, cx);
            });
        });
        visual.run_until_parked();
    }
    let markdown = visual.update(|_, app| {
        pane.update(app, PullRequestDetail::copy_local_review);
        app.global_mut::<UiStateRecorder>().flush_on_quit();
        app.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap()
    });

    // The persisted anchors and excerpts must match the corresponding block in the clipboard.
    let notes = UiStateStore::open(&directory.path().join("cache"))
        .unwrap()
        .review_notes(&ReviewScope::PullRequest(1), NoteDestination::Local)
        .unwrap();
    assert_eq!(notes.len(), 2, "{markdown}");
    for (note, side, first, last, excerpt, label, body) in [
        (
            &notes[0],
            LineSide::Old,
            10,
            12,
            " expanded above\n-\tlet value = 1;\n-   let cost = 2;  \n shared context\n",
            "Lines 10–12 (old)",
            "Base note",
        ),
        (
            &notes[1],
            LineSide::New,
            20,
            22,
            " expanded above\n+let value = 1;  \n+\tlet cost = 2;\n shared context\n",
            "Lines 20–22",
            "Head note",
        ),
    ] {
        assert_eq!(
            note.anchor,
            NoteAnchor::Line {
                side,
                line: last,
                start_line: Some(first)
            }
        );
        assert_eq!(note.excerpt.as_deref(), Some(excerpt));
        assert_eq!(note.body, body);
        assert!(
            markdown.contains(&format!(
                "### {label}\nOriginal diff `base...head`\n\n```diff\n{excerpt}```\n\n{body}"
            )),
            "{markdown}"
        );
    }
}

/// Two folded replacements have different whitespace on each side and revealed context above them.
fn folded_export_fixture(
    services: AppServices,
    cx: &mut Context<PullRequestDetail>,
) -> PullRequestDetail {
    use crate::pull_request_detail::loaded_file::FileContext;
    use qd_cache::{NoteDestination, ReviewScope};
    use qd_diff::{RevealedLines, SourceLines};

    let patch = "@@ -10,4 +20,4 @@\n-\tlet value = 1;\n-   let cost = 2;  \n+let value = 1;  \n+\tlet cost = 2;\n shared context\n-old value\n+new value";
    let mut detail = split_comment_fixture(services, patch, qd_diff::WhitespaceMode::Ignore, cx);
    detail.viewed_scope = Some(ReviewScope::PullRequest(1));
    detail.shown_head_sha = Some("head".into());
    detail.shown_base_sha = Some("base".into());
    detail
        .review
        .comments
        .head_sha
        .clone_from(&detail.shown_head_sha);
    detail
        .review
        .comments
        .base_sha
        .clone_from(&detail.shown_base_sha);
    detail.set_comment_destination(NoteDestination::Local, cx);
    let source = format!(
        "{}expanded above\nlet value = 1;  \n\tlet cost = 2;\nshared context\nnew value",
        "\n".repeat(18)
    );
    let mut revealed = RevealedLines::default();
    revealed.reveal(19..20);
    let expanded = detail.loaded_files()[0]
        .with_context(FileContext {
            source: SourceLines::parse(&source).into(),
            revealed,
        })
        .unwrap();
    detail.changed_files = Some(ChangedFiles::Loaded(vec![expanded]));
    detail.rebuild_rows(ScrollAnchor::Top);
    detail
}

fn expect_anchor(
    pane: &gpui::Entity<PullRequestDetail>,
    side: LineSide,
    first: Option<u32>,
    last: u32,
    visual: &mut gpui::VisualTestContext,
) {
    visual.update(|_, app| {
        assert_eq!(
            pane.read(app).comment_range(),
            Some(CommentRange::ending_at(0, side, last, first))
        );
    });
}

fn click(id: &'static str, modifiers: Modifiers, visual: &mut gpui::VisualTestContext) {
    let at = visual
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("missing {id}"))
        .center();
    visual.simulate_mouse_move(at, None, modifiers);
    visual.simulate_click(at, modifiers);
    visual.run_until_parked();
}

fn drag(from: &'static str, to: &'static str, visual: &mut gpui::VisualTestContext) {
    let first = visual.debug_bounds(from).unwrap().center() + point(px(80.), px(0.));
    let last = visual.debug_bounds(to).unwrap().center() + point(px(93.), px(0.));
    visual.simulate_mouse_down(first, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(last, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(last, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
}
