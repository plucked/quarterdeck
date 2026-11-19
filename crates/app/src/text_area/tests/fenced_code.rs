use gpui::TestAppContext;

use super::super::markdown_format::MarkdownFormat;
use super::super::{TextArea, bind_text_area_keys};

/// Pasting a fence in rich mode widens the delimiters, and undo restores the complete original proposal.
#[gpui::test]
fn rich_suggestion_edits_preserve_the_fence(cx: &mut TestAppContext) {
    cx.update(bind_text_area_keys);
    let (editor, visual) = cx.add_window_view(|window, cx| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.insert_suggestion("seed", cx);
        area.focus(window);
        area
    });
    visual.update(|_, app| {
        app.write_to_clipboard(gpui::ClipboardItem::new_string("example\n```\nend".into()));
    });
    visual.simulate_keystrokes("cmd-v");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "````suggestion\nexample\n```\nend\n````");
        assert_eq!(area.editing_text(), "example\n```\nend");
    });
    visual.simulate_keystrokes("cmd-z");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "```suggestion\nseed\n```");
    });
    visual.simulate_keystrokes("cmd-z");
    editor.update(visual, |area, _| {
        assert_eq!(area.text(), "");
        assert!(area.suggestion_anchors_range());
    });
}
/// Toolbar, keyboard and clipboard operations must all preserve the literal proposed code.
#[gpui::test]
fn formatted_clipboard_and_inline_controls_leave_code_literal(cx: &mut TestAppContext) {
    cx.update(crate::app_commands::install_app_commands);
    let (editor, visual) = cx.add_window_view(|window, cx| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.set_text("**limit**", cx);
        area.selected_range = 2..7;
        area.focus(window);
        area
    });
    visual.simulate_keystrokes("cmd-c");
    visual.update(|_, cx| {
        let entry = cx.read_from_clipboard().unwrap();
        assert_eq!(entry.text().as_deref(), Some("limit"));
        assert!(entry.metadata().unwrap().contains("**limit**"));
    });
    for body in ["```suggestion\nseed\n```", "~~~rust\nseed\n~~~"] {
        visual.update(|window, cx| {
            editor.update(cx, |area, cx| {
                area.set_text(body, cx);
                let start = body.find("seed").unwrap();
                area.selected_range = start..start + "seed".len();
                for format in [
                    MarkdownFormat::Bold,
                    MarkdownFormat::Italic,
                    MarkdownFormat::Strike,
                    MarkdownFormat::Code,
                    MarkdownFormat::Link,
                ] {
                    assert!(!area.allows_rich_format(format));
                    format.apply(area, cx);
                }
                area.open_rich_link(window, cx);
                assert!(area.markdown.link.is_none());
                assert_eq!(area.text(), body);
            });
        });
        visual.simulate_keystrokes("cmd-v");
        editor.update(visual, |area, _| {
            assert_eq!(area.text(), body.replace("seed", "limit"));
        });
    }
}

/// The first edit of a compact empty fence creates the missing line break. The current
/// source style survives subsequent paste, widening and undo.
#[gpui::test]
fn edits_compact_and_unfinished_fences(cx: &mut TestAppContext) {
    cx.update(bind_text_area_keys);
    let (editor, visual) = cx.add_window_view(|window, cx| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.focus(window);
        area
    });
    for (body, typed, expected) in [
        (
            "Intro\n\n~~~suggestion\r\n~~~",
            "~~~",
            "Intro\n\n~~~~suggestion\r\n~~~\r\n~~~~",
        ),
        ("```suggestion", "a", "```suggestion\na"),
        ("```suggestion\n\n```", "a", "```suggestion\na\n```"),
    ] {
        editor.update(visual, |area, cx| {
            area.set_text(body, cx);
            area.insert_suggestion("unused", cx);
        });
        visual.update(|_, cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(typed.into()));
        });
        visual.simulate_keystrokes("cmd-v");
        editor.update(visual, |area, _| assert_eq!(area.text(), expected));
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), body));
    }
}

/// Every pasted line stays literal inside a quote, list item or combination of the two.
#[gpui::test]
fn pasting_lines_inside_nested_code_keeps_the_container(cx: &mut TestAppContext) {
    cx.update(bind_text_area_keys);
    let (editor, visual) = cx.add_window_view(|window, cx| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.focus(window);
        area
    });
    let paste = "first\n**second**\n```\nλ";
    visual.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(paste.into()));
    });
    for (original, expected) in [
        (
            "> ```rust\n> seed\n> ```",
            "> ````rust\n> first\n> **second**\n> ```\n> λ\n> ````",
        ),
        (
            "- item\n  ```rust\n  seed\n  ```",
            "- item\n  ````rust\n  first\n  **second**\n  ```\n  λ\n  ````",
        ),
        (
            "1. > ```rust\n   > ```",
            "1. > ````rust\n   > first\n   > **second**\n   > ```\n   > λ\n   > ````",
        ),
        (">```rust", ">````rust\n> first\n> **second**\n> ```\n> λ"),
    ] {
        // Replace the visible payload through native paste, including the first edit of an empty block.
        editor.update(visual, |area, cx| {
            area.set_text(original, cx);
            area.selected_range = area.markdown.rich.as_ref().unwrap().blocks.fences()[0]
                .code
                .clone();
        });
        visual.simulate_keystrokes("cmd-v");
        editor.update(visual, |area, _| {
            assert_eq!(area.text(), expected);
            let rich = area.markdown.rich.as_ref().unwrap();
            assert_eq!(rich.blocks.fences().len(), 1);
            assert!(rich.marks.is_empty());
            assert_eq!(
                area.cursor_offset(),
                expected.find('λ').unwrap() + 'λ'.len_utf8()
            );
        });

        // History treats the continued lines and widened delimiters as one edit.
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), original));
        visual.simulate_keystrokes("cmd-shift-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), expected));
    }
}

/// Spaces and tabs on an under-indented quote or list payload line remain literal after parsing.
#[gpui::test]
fn editing_nested_code_keeps_literal_indentation(cx: &mut TestAppContext) {
    use qd_markdown::{MarkdownBlock, MarkdownDocument};

    cx.update(bind_text_area_keys);
    let (editor, visual) = cx.add_window_view(|window, cx| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.focus(window);
        area
    });
    for (original, insert, expected) in [
        (
            ">~~~rust\n>seed\n>~~~",
            " first\n second",
            " first\n second",
        ),
        (
            "> ~~~rust\n> one\n>seed\n> ~~~",
            "\tfirst\n\tsecond",
            "one\n\tfirst\n\tsecond",
        ),
        (
            ">~~~rust\n>before>seed\n>~~~",
            " middle\n end",
            "before> middle\n end",
        ),
        (">~~~rust\n>seed keep\n>~~~", "", " keep"),
        (
            "- item\n    ~~~rust\n  seed\n    ~~~",
            " first\n second",
            " first\n second",
        ),
        (
            ">   ~~~rust\n> seed\n>   ~~~",
            "\tfirst\n second",
            "\tfirst\n second",
        ),
        (
            ">   ~~~rust\n  > seed\n>   ~~~",
            " first\n second",
            " first\n second",
        ),
    ] {
        // Put the selection at the physical quote, on a later line, or after a literal angle bracket.
        editor.update(visual, |area, cx| {
            area.set_text(original, cx);
            let start = original.find("seed").unwrap();
            area.selected_range = start..start + "seed".len();
        });
        if insert.is_empty() {
            visual.simulate_keystrokes("backspace");
        } else {
            visual.update(|_, app| {
                app.write_to_clipboard(gpui::ClipboardItem::new_string(insert.into()));
            });
            visual.simulate_keystrokes("cmd-v");
        }

        // Compare the literal content GitHub would receive after Markdown consumes its quote padding.
        editor.update(visual, |area, _| {
            let document = MarkdownDocument::parse(area.text());
            let nested = match document.blocks() {
                [MarkdownBlock::BlockQuote(blocks)] => blocks,
                [MarkdownBlock::List(list)] => &list.items[0].blocks,
                other => panic!("expected one quote or list in {other:?}"),
            };
            let Some(MarkdownBlock::CodeBlock { code, .. }) = nested.last() else {
                panic!("expected a code block in {nested:?}");
            };
            assert_eq!(code, expected);
        });
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), original));
    }
}

/// Deleting the suffix of a marker line keeps the exposed run inside the proposed code.
#[gpui::test]
fn deleting_text_after_a_literal_fence_keeps_the_suggestion(cx: &mut TestAppContext) {
    cx.update(bind_text_area_keys);
    let original = "```suggestion\nhello\n```lang\n```\n\nend";
    let (editor, visual) = cx.add_window_view(|window, cx| {
        let mut area = TextArea::comment("", cx);
        area.set_github_markdown(true, cx);
        area.set_text(original, cx);
        let start = original.find("lang").unwrap();
        area.selected_range = start..start + "lang".len();
        area.focus(window);
        area
    });

    visual.simulate_keystrokes("backspace");
    editor.update(visual, |area, _| {
        let fence = area.markdown.suggestion.as_ref().unwrap();
        assert_eq!(&area.text()[fence.code.clone()], "hello\n```");
        assert_eq!(area.text(), "````suggestion\nhello\n```\n````\n\nend");
    });
    visual.simulate_keystrokes("cmd-z");
    editor.update(visual, |area, _| assert_eq!(area.text(), original));
}
