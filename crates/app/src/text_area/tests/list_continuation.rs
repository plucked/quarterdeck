use gpui::{Context, Entity, TestAppContext, VisualTestContext};

use super::super::markdown_format::MarkdownFormat;
use super::super::{TextArea, bind_text_area_keys};

/// A focused native input with the same text and action bindings as a review comment.
fn composer(cx: &mut TestAppContext) -> (Entity<TextArea>, &mut VisualTestContext) {
    cx.update(bind_text_area_keys);
    cx.add_window_view(|window, cx: &mut Context<TextArea>| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.focus(window);
        area
    })
}

/// A second Return on an empty item ends the list. Undo and redo restore the same items.
#[gpui::test]
fn return_continues_then_exits_a_rich_list(cx: &mut TestAppContext) {
    let (editor, visual) = composer(cx);
    editor.update(visual, |area, cx| MarkdownFormat::Bullets.apply(area, cx));
    visual.simulate_input("First");
    visual.simulate_keystrokes("enter");
    visual.simulate_input("Second");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "- First\n- Second");
    });
    visual.simulate_keystrokes("enter enter");
    visual.simulate_input("Outside");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "- First\n- Second\n\nOutside");
    });
    visual.simulate_keystrokes("cmd-z cmd-z");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "- First\n- Second\n- ");
    });
    visual.simulate_keystrokes("cmd-shift-z");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "- First\n- Second\n\n");
    });
}

/// Ordered lists progress, tasks reset and nested or quoted markers retain their container.
#[gpui::test]
fn return_preserves_list_kind_and_nesting(cx: &mut TestAppContext) {
    let (editor, visual) = composer(cx);
    for (source, expected) in [
        ("9) Hello", "9) Hello\n10) "),
        ("* [X] Done", "* [X] Done\n* [ ] "),
        ("4. [x] Checked", "4. [x] Checked\n5. [ ] "),
        ("- outer\n  + child", "- outer\n  + child\n  + "),
        ("> - quoted", "> - quoted\n> - "),
        ("- - nested", "- - nested\n  - "),
        ("- First\n\n- ", "- First\n\n"),
        ("- ", ""),
    ] {
        editor.update(visual, |area, cx| area.set_text(source, cx));
        visual.simulate_keystrokes("enter");
        editor.update(visual, |area, _| assert_eq!(area.text(), expected));
    }
}

/// A soft break remains in the same item, and source mode and literal code use plain newlines.
#[gpui::test]
fn literal_and_soft_breaks_in_lists(cx: &mut TestAppContext) {
    let (editor, visual) = composer(cx);
    editor.update(visual, |area, cx| area.set_text("- First", cx));
    visual.simulate_keystrokes("shift-enter");
    visual.simulate_input("continuation");
    visual.simulate_keystrokes("enter");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "- First\ncontinuation\n- ");
    });
    editor.update(visual, |area, cx| {
        area.set_text("```suggestion\n- literal\n```", cx);
        let caret = area.text().find("literal").unwrap() + "literal".len();
        area.selected_range = caret..caret;
    });
    visual.simulate_keystrokes("enter");
    editor.update(visual, |area, cx| {
        assert_eq!(area.text(), "```suggestion\n- literal\n\n```");
        area.set_github_markdown(false, cx);
        area.set_text("- First", cx);
    });
    visual.simulate_keystrokes("enter");
    editor.update(visual, |area, _| assert_eq!(area.text(), "- First\n"));
}

/// Splitting beside a space leaves valid emphasis on both items and retains the text.
#[gpui::test]
fn split_list_beside_space_preserves_styles(cx: &mut TestAppContext) {
    let (editor, visual) = composer(cx);
    for (offset, expected) in [
        (6, "- **ab**\n-  **cd**"),
        (5, "- **a**\n- **b cd**"),
        (7, "- **ab** \n- **cd**"),
    ] {
        // The source cursor is tested on both sides of the space between the two words.
        editor.update(visual, |area, cx| {
            area.set_text("- **ab cd**", cx);
            area.selected_range = offset..offset;
        });
        visual.simulate_keystrokes("enter");
        editor.update(visual, |area, _| assert_eq!(area.text(), expected));
    }
}

/// Splitting a bold link keeps each half formatted and leaves the caret before the remaining text.
#[gpui::test]
fn split_formatted_text_inside_a_list_item(cx: &mut TestAppContext) {
    let (editor, visual) = composer(cx);
    editor.update(visual, |area, cx| {
        area.set_text("- **[café猫](https://example.org)**", cx);
        let caret = area.text().find('猫').unwrap();
        area.selected_range = caret..caret;
    });
    visual.simulate_keystrokes("enter");
    visual.simulate_input("é");
    editor.update(visual, |area, _| {
        assert_eq!(
            area.text(),
            "- **[café](https://example.org)**\n- **[é猫](https://example.org)**"
        );
    });
}

/// A split can put a literal backtick at either edge of an inline code span.
#[gpui::test]
fn split_inline_code_beside_backticks(cx: &mut TestAppContext) {
    let (editor, visual) = composer(cx);
    for (raw, left, right) in [
        ("- ``a`b``", "a", "`b"),
        ("- ``a`b``", "a`", "b"),
        ("- `a``b`", "a`", "`b"),
        ("- **[``a`b``](url)**", "a`", "b"),
    ] {
        let caret = raw.find('a').unwrap() + left.len();
        editor.update(visual, |area, cx| {
            area.set_text(raw, cx);
            area.selected_range = caret..caret;
        });
        visual.simulate_keystrokes("enter");
        editor.update(visual, |area, _| {
            assert_eq!(area.editing_text(), format!("• {left}\n• {right}"));
            let projected = area.markdown.rich.as_ref().unwrap();
            let code = projected
                .marks
                .iter()
                .filter(|mark| mark.style == qd_markdown::InlineStyle::CODE)
                .map(|mark| &area.text()[mark.inner.clone()])
                .collect::<Vec<_>>();
            assert_eq!(code, [left, right]);
            assert_eq!(
                area.to_visible(area.cursor_offset()),
                "• ".len() * 2 + left.len() + 1
            );
            if raw.contains("(url)") {
                assert_eq!(area.text().matches("(url)").count(), 2);
            }
        });
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| {
            assert_eq!(area.text(), raw);
            assert_eq!(area.selected_range, caret..caret);
        });
    }
}
