use gpui::{
    ClipboardItem, Context, Entity, Modifiers, Render, Subscription, TestAppContext, Window, div,
    prelude::*, px,
};
use qd_markdown::{InlineStyle, MarkdownDocument};

use super::super::comment_history::EditSource;
use super::super::markdown_format::MarkdownFormat;
use super::super::{TextArea, TextAreaEvent};
use crate::app_commands::install_app_commands;
use crate::text_area::MarkdownEditor;

/// Hosts a composer and repaints it when its draft or formatting changes.
struct RichFixture {
    area: Entity<TextArea>,
    _subscription: Subscription,
}

impl Render for RichFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(590.))
            .child(MarkdownEditor::new(self.area.clone()))
    }
}

/// Opens a focused review draft with the application's keyboard shortcuts.
fn open(cx: &mut TestAppContext) -> (Entity<TextArea>, &mut gpui::VisualTestContext) {
    cx.update(install_app_commands);
    let area = cx.new(|cx| {
        let mut area = TextArea::comment("Type a comment", cx);
        area.set_github_markdown(true, cx);
        area
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        area.read(cx).focus(window);
        let subscription = cx.subscribe(&area, |_, _, _: &TextAreaEvent, cx| cx.notify());
        RichFixture {
            area: area.clone(),
            _subscription: subscription,
        }
    });
    (area, cx)
}

/// Typing runs render without delimiters across spaces and punctuation, including toggle-off and undo.
#[gpui::test]
fn typing_styles_and_paste(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    cx.simulate_keystrokes("cmd-b");
    for input in ["a", " ", "β", "!"] {
        cx.simulate_input(input);
    }
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.editing_text(), "a β!");
        assert_eq!(area.text(), "**a β!**");
        assert!(
            area.markdown
                .rich
                .as_ref()
                .unwrap()
                .runs
                .iter()
                .all(|run| run.style.contains(InlineStyle::STRONG))
        );
    });
    cx.simulate_keystrokes("cmd-b");
    cx.simulate_input(" plain");
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.editing_text(), "a β! plain");
        assert!(!area.rich_format_active(MarkdownFormat::Bold));
    });

    // Copy formatted text to another position; the system text is readable while in-app paste keeps styles.
    cx.simulate_keystrokes("cmd-a cmd-c right cmd-v");
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.editing_text(), "a β! plaina β! plain");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some("a β! plain")
        );
    });
    cx.simulate_keystrokes("cmd-z");
    cx.update(|_, cx| assert_eq!(area.read(cx).editing_text(), "a β! plain"));
}

/// Pending emphasis survives pasting paragraphs with surrounding whitespace.
#[gpui::test]
fn pasting_paragraphs_with_typing_style(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    cx.simulate_keystrokes("cmd-i");
    cx.update(|_, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(" Hello\n\nworld ".into()));
    });
    cx.simulate_keystrokes("cmd-v");
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.editing_text(), " Hello\n\nworld ");
        assert_eq!(area.text(), " *Hello*\n\n*world* ");
    });
    cx.simulate_keystrokes("cmd-z");
    cx.update(|_, cx| assert_eq!(area.read(cx).text(), ""));
}

/// Turning bold off splits the surrounding span while retaining its words.
#[gpui::test]
fn removing_a_style_between_words(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    cx.update(|_, cx| {
        area.update(cx, |area, cx| {
            area.set_text("**the cat sat**", cx);
            let start = area.text().find("cat").unwrap();
            area.selected_range = start..start + 3;
            MarkdownFormat::Bold.apply(area, cx);
            assert_eq!(area.text(), "**the** cat **sat**");
            assert_eq!(&area.text()[area.selected_range.clone()], "cat");

            area.set_text("**the sat**", cx);
            let caret = area.text().find("sat").unwrap();
            area.selected_range = caret..caret;
            MarkdownFormat::Bold.apply(area, cx);
        });
    });
    cx.simulate_input("cat ");
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.editing_text(), "the cat sat");
        assert_eq!(area.text(), "**the** cat **sat**");
    });
}

/// Clicks a visible composer tab or formatting control by its label.
fn click_control(cx: &mut gpui::VisualTestContext, label: &'static str) {
    let bounds = cx.debug_bounds(label).unwrap();
    cx.simulate_click(bounds.center(), Modifiers::default());
    cx.run_until_parked();
}

/// Switching from Markdown maps either end of a hidden selection onto the visible draft.
#[gpui::test]
fn switching_modes_moves_hidden_selections_to_visible_text(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    let raw = "See [café](https://example.org) today";
    for (start, end, expected) in [
        (
            "example",
            "example",
            "See [café](https://example.org)! today",
        ),
        ("example", "org", "See [café](https://example.org)! today"),
        ("fé", "org", "See [ca!](https://example.org) today"),
    ] {
        click_control(cx, "Markdown");
        cx.update(|_, cx| {
            area.update(cx, |area, cx| {
                area.set_text(raw, cx);
                area.selected_range = raw.find(start).unwrap()..raw.find(end).unwrap();
                area.selection_reversed = true;
            });
        });
        click_control(cx, "Rich text");
        cx.simulate_input("!");
        cx.update(|_, cx| assert_eq!(area.read(cx).text(), expected));
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| {
            let area = area.read(cx);
            assert_eq!(area.text(), raw);
            if start == "fé" {
                assert!(area.selection_reversed);
                assert_eq!(&area.text()[area.selected_range.clone()], "fé");
            } else {
                assert!(area.selected_range.is_empty());
            }
        });
    }
}

/// Formatting a mixed selection keeps its text, direction and undo history.
#[gpui::test]
fn formatting_mixed_inline_spans_preserves_text_selection_and_undo(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    for (format, raw, selected, merged) in [
        ("Bold", "**álpha** βeta", "ha βeta", "**álpha βeta**"),
        ("Bold", "álpha **βeta**", "álpha βe", "**álpha βeta**"),
        ("Italic", "*αβ* cat *dog*", "β cat do", "*αβ cat dog*"),
        (
            "Strikethrough",
            "~~álpha~~ [βeta](here)",
            "ha βeta",
            "~~álpha [βeta](here)~~",
        ),
        ("Inline code", "`άλφα` βeta", "φα βeta", "`άλφα βeta`"),
        ("Bold", "`άλφα` βeta", "φα βe", "`άλ`**`φα` βe**ta"),
        ("Bold", "**άλφα βeta**", "φα βe", "**άλ**φα βe**ta**"),
    ] {
        let visible = cx.update(|_, cx| {
            area.update(cx, |area, cx| {
                area.set_text(raw, cx);
                let visible = area.editing_text().to_owned();
                let start = visible.find(selected).unwrap();
                area.selected_range = area.source_range(start..start + selected.len());
                area.selection_reversed = true;
                visible
            })
        });
        click_control(cx, format);
        cx.update(|_, cx| {
            let area = area.read(cx);
            assert_eq!(area.editing_text(), visible);
            assert_eq!(area.text(), merged);
            assert_eq!(&visible[area.visible_range(&area.selected_range)], selected);
            assert!(area.selection_reversed);
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| {
            let area = area.read(cx);
            assert_eq!(area.text(), raw);
            assert!(area.selection_reversed);
            assert_eq!(&visible[area.visible_range(&area.selected_range)], selected);
        });
    }
}

/// Replacement and deletion keep partial suggestions fenced through undo.
#[gpui::test]
fn replacing_a_selection_across_suggestion_boundaries(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    let raw = "álpha\n\n```suggestion\naβc\n```\n\nomega";
    for (start, end, insert, expected) in [
        ("pha", "β", "", "ál\n```suggestion\nβc\n```\n\nomega"),
        ("β", "ega", "λ", "álpha\n\n```suggestion\naλ\n```\nega"),
        ("pha", "ega", "", "álega"),
    ] {
        let selection = raw.find(start).unwrap()..raw.find(end).unwrap();
        cx.update(|_, cx| {
            area.update(cx, |area, cx| {
                area.set_text(raw, cx);
                area.selected_range = selection.clone();
                area.selection_reversed = true;
            });
        });
        if insert.is_empty() {
            cx.simulate_keystrokes("backspace");
        } else {
            cx.simulate_input(insert);
        }
        cx.update(|_, cx| {
            let area = area.read(cx);
            assert_eq!(area.text(), expected);
            assert!(!area.editing_text().contains("```"));
            assert_eq!(
                area.markdown.suggestion.is_some(),
                expected.contains("```suggestion")
            );
            assert!(area.suggestion_anchors_range());
        });
        cx.simulate_keystrokes("cmd-z");
        cx.update(|_, cx| {
            let area = area.read(cx);
            assert_eq!(area.text(), raw);
            assert_eq!(area.selected_range, selection);
            assert!(area.selection_reversed);
        });
    }
}

/// A pasted closing marker stays inside the suggestion when the selection runs into prose.
#[gpui::test]
fn paste_across_a_closing_fence_keeps_the_complete_proposal(cx: &mut TestAppContext) {
    let (area, visual) = open(cx);
    for marker in ["```", "~~~"] {
        let original = format!("intro\n\n{marker}suggestion\nalpha\n{marker}\n\nend");
        let pasted = format!("p\n{marker}\nλ");
        let selection = original.find("pha").unwrap()..original.find("end").unwrap() + 1;
        visual.update(|_, app| {
            area.update(app, |area, cx| {
                area.set_text(&original, cx);
                area.selected_range = selection.clone();
            });
            app.write_to_clipboard(ClipboardItem::new_string(pasted.clone()));
        });
        visual.simulate_keystrokes("cmd-v");
        visual.update(|_, app| {
            let editor = area.read(app);
            let fence = editor.markdown.suggestion.as_ref().unwrap();
            assert_eq!(&editor.text()[fence.code.clone()], format!("al{pasted}"));
            assert_eq!(&editor.text()[fence.outer.end..], "\nnd");
            assert_eq!(
                editor.markdown.rich.as_ref().unwrap().blocks.fences().len(),
                1
            );
        });
        visual.simulate_keystrokes("cmd-z");
        visual.update(|_, app| {
            assert_eq!(area.read(app).text(), original);
            assert_eq!(area.read(app).selected_range, selection);
        });
    }
}

/// A paste that starts inside quoted code keeps every inserted line inside the same block.
#[gpui::test]
fn paste_from_nested_code_into_prose_retains_the_quote(cx: &mut TestAppContext) {
    let (editor, visual) = open(cx);
    let original = "> ```rust\n> seed\n> ```\n\nend";
    visual.update(|_, app| {
        editor.update(app, |area, cx| {
            area.set_text(original, cx);
            area.selected_range = original.find("ed").unwrap()..original.find("nd").unwrap();
        });
        app.write_to_clipboard(ClipboardItem::new_string("line\n```\n**λ**".into()));
    });
    visual.simulate_keystrokes("cmd-v");
    visual.update(|_, app| {
        let area = editor.read(app);
        assert_eq!(
            area.text(),
            "> ````rust\n> seline\n> ```\n> **λ**\n> ````\nnd"
        );
        let rich = area.markdown.rich.as_ref().unwrap();
        assert_eq!(rich.blocks.fences().len(), 1);
        assert!(rich.marks.is_empty());
    });
    visual.simulate_keystrokes("cmd-z");
    visual.update(|_, app| assert_eq!(editor.read(app).text(), original));
}

/// An edit entering nested code restores its opening row's original Markdown container.
#[gpui::test]
fn paste_from_prose_into_nested_code_retains_the_opening_container(cx: &mut TestAppContext) {
    let (editor, visual) = open(cx);
    for (raw, expected) in [
        (
            "intro\n\n> ```rust\n> seed\n> ```\n\nafter",
            "inX\n> ```rust\n> ed\n> ```\n\nafter",
        ),
        (
            "intro\n\n- ```rust\n  seed\n  ```\n\nafter",
            "inX\n- ```rust\n  ed\n  ```\n\nafter",
        ),
    ] {
        visual.update(|_, app| {
            editor.update(app, |area, cx| {
                area.set_text(raw, cx);
                area.selected_range = raw.find("tro").unwrap()..raw.find("ed").unwrap();
            });
            app.write_to_clipboard(ClipboardItem::new_string("X".into()));
        });
        visual.simulate_keystrokes("cmd-v");
        visual.update(|_, app| {
            let area = editor.read(app);
            assert_eq!(area.text(), expected);
            let rich = area.markdown.rich.as_ref().unwrap();
            assert_eq!(rich.blocks.fences().len(), 1);
            let fence = &rich.blocks.fences()[0];
            assert_eq!(&area.text()[fence.code.clone()], "ed");
            assert_eq!(&area.text()[fence.outer.end..], "\n\nafter");
        });
        visual.simulate_keystrokes("cmd-z");
        visual.update(|_, app| assert_eq!(editor.read(app).text(), raw));
    }
}

/// Removing a suffix from literal backticks must keep them inside the proposed replacement.
#[gpui::test]
fn deletion_across_a_fence_can_expose_a_literal_closing_marker(cx: &mut TestAppContext) {
    let (editor, visual) = open(cx);
    for (original, start, end, expected_code, after) in [
        (
            "```suggestion\nhello\n```lang\n```\n\nend",
            "lang",
            "nd",
            "hello\n```",
            "\nnd",
        ),
        (
            "intro\n\n```suggestion\nlang```\nhello\n```\n\nend",
            "tro",
            "```\nhello",
            "```\nhello",
            "\n\nend",
        ),
    ] {
        visual.update(|_, app| {
            editor.update(app, |area, cx| {
                area.set_text(original, cx);
                area.selected_range = original.find(start).unwrap()..original.find(end).unwrap();
            });
        });
        visual.simulate_keystrokes("backspace");
        visual.update(|_, app| {
            let area = editor.read(app);
            let fence = area.markdown.suggestion.as_ref().unwrap();
            assert_eq!(&area.text()[fence.code.clone()], expected_code);
            assert_eq!(&area.text()[fence.outer.end..], after);
            assert_eq!(
                area.markdown.rich.as_ref().unwrap().blocks.fences().len(),
                1
            );
        });
        visual.simulate_keystrokes("cmd-z");
        visual.update(|_, app| assert_eq!(editor.read(app).text(), original));
    }
}

/// The link editor takes an address, preserves the selected label, and source edits survive mode switches.
#[gpui::test]
fn link_edit_and_source_roundtrip(cx: &mut TestAppContext) {
    let (area, cx) = open(cx);
    cx.simulate_input("Docs");
    cx.simulate_keystrokes("cmd-a cmd-k");
    cx.simulate_input("https://example.org");
    cx.simulate_keystrokes("enter");
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.text(), "[Docs](https://example.org)");
        assert_eq!(area.editing_text(), "Docs");
        assert!(
            area.markdown.rich.as_ref().unwrap().runs[0]
                .style
                .contains(InlineStyle::LINK)
        );
    });
    cx.simulate_keystrokes("cmd-k");
    cx.simulate_input("https://example.org/new");
    cx.simulate_keystrokes("enter");
    // Editing the body invalidates the original source selection captured by the URL field.
    cx.simulate_keystrokes("cmd-k");
    cx.update(|window, cx| {
        area.update(cx, |area, cx| {
            assert!(area.markdown.link.is_some());
            area.focus(window);
            area.edit_input(0..area.text().len(), "new draft", EditSource::Typing, cx);
            assert!(area.markdown.link.is_none());
        });
    });
    click_control(cx, "Markdown");
    let raw = "<details>custom</details>\n\n[Docs](https://example.org/new)";
    cx.simulate_keystrokes("cmd-a");
    cx.simulate_input(raw);
    click_control(cx, "Rich text");
    click_control(cx, "Preview");
    cx.update(|_, cx| {
        let area = area.read(cx);
        assert_eq!(area.text(), raw);
        assert_eq!(
            area.markdown.preview.as_deref(),
            Some(&MarkdownDocument::parse(raw))
        );
    });
}

/// Inline formatting spans line contents while lists, headings and nested quotes keep their markers.
#[gpui::test]
fn bold_across_block_prefixes_preserves_the_selection_and_undo(cx: &mut TestAppContext) {
    let (area, visual) = open(cx);
    for (raw, expected, select_all) in [
        ("- **alpha**\n- beta", "- **alpha**\n- **beta**", false),
        ("- alpha\n- beta", "- **alpha**\n- **beta**", true),
        (
            "> > alpha\n> > 1. beta",
            "> > **alpha**\n> > 1. **beta**",
            true,
        ),
        (
            "- [ ] alpha\n- [x] beta",
            "- [ ] **alpha**\n- [x] **beta**",
            true,
        ),
        ("# alpha\n\nbeta", "# **alpha**\n\n**beta**", true),
    ] {
        visual.update(|_, app| {
            area.update(app, |area, cx| {
                area.set_text(raw, cx);
                if !select_all {
                    let start = area.editing_text().find("ha").unwrap();
                    area.selected_range = area.source_range(start..area.editing_text().len());
                    area.selection_reversed = true;
                }
            });
        });
        if select_all {
            visual.simulate_keystrokes("cmd-a");
        }
        let (visible, selected, original_range) = visual.update(|_, app| {
            let area = area.read(app);
            (
                area.editing_text().to_owned(),
                area.visible_range(&area.selected_range),
                area.selected_range.clone(),
            )
        });
        visual.simulate_keystrokes("cmd-b");
        visual.update(|_, app| {
            let area = area.read(app);
            assert_eq!(area.text(), expected);
            assert_eq!(area.editing_text(), visible);
            assert_eq!(area.visible_range(&area.selected_range), selected);
            assert_eq!(area.selection_reversed, !select_all);
            assert!(area.rich_format_active(MarkdownFormat::Bold));
            let rich = area.markdown.rich.as_ref().unwrap();
            for word in ["alpha", "beta"] {
                let start = visible.find(word).unwrap();
                assert!(
                    (start..start + word.len()).all(|byte| rich
                        .runs
                        .iter()
                        .any(|run| run.range.contains(&byte)
                            && run.style.contains(InlineStyle::STRONG)))
                );
            }
        });
        visual.simulate_keystrokes("cmd-z");
        visual.update(|_, app| {
            let area = area.read(app);
            assert_eq!(area.text(), raw);
            assert_eq!(area.selected_range, original_range);
            assert_eq!(area.selection_reversed, !select_all);
        });
        if select_all {
            visual.simulate_keystrokes("cmd-b cmd-b");
            visual.update(|_, app| assert_eq!(area.read(app).text(), raw));
        }
    }
}
