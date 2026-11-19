use std::sync::Arc;

use gpui::{
    App, Context, Entity, Focusable as _, FontWeight, IntoElement, RenderOnce, Window, canvas, div,
    prelude::*, px, svg,
};
use qd_markdown::MarkdownDocument;

use super::markdown_format::MarkdownFormat;
use super::text_area_actions::{Bold, Cancel, InsertLink, Italic, Submit};
use super::{TextArea, TextAreaEvent};
use crate::controls::TextTooltip;
use crate::count_as_f32::count_as_f32;
use crate::markdown_view::MarkdownView;
use crate::theme;

const FORMAT_BUTTON_WIDTH: f32 = 26.;
const MORE_BUTTON_WIDTH: f32 = 52.;
/// Keep the four most-used formats beside the tabs, or wrap the toolbar to the next line.
const COMPACT_TOOLBAR_WIDTH: f32 = 4. * FORMAT_BUTTON_WIDTH + MORE_BUTTON_WIDTH;

/// Shows the longest prefix that fits, reserving the disclosure button only when it is needed.
fn visible_format_count(width: gpui::Pixels) -> usize {
    let count = MarkdownFormat::TOOLBAR.len();
    if width >= px(FORMAT_BUTTON_WIDTH * count_as_f32(count)) {
        return count;
    }
    (1..count)
        .take_while(|&visible| {
            px(MORE_BUTTON_WIDTH + FORMAT_BUTTON_WIDTH * count_as_f32(visible)) <= width
        })
        .count()
}

/// Presentation state that belongs to the draft and survives virtualized row recycling.
#[derive(Default)]
pub(super) struct MarkdownEditorState {
    pub(super) preview: Option<Arc<MarkdownDocument>>,
    pub(super) github: bool,
    /// Keeps the anchor fixed while this draft or its undo history contains a suggestion.
    pub(super) suggestion_in_history: bool,
    pub(super) suggestion: Option<qd_markdown::MarkdownFence>,
    pub(super) link: Option<super::rich_link::RichLink>,
    pub(super) source_mode: bool,
    pub(super) rich: Option<Arc<qd_markdown::RichDocument>>,
    pub(super) pending: Vec<(qd_markdown::InlineStyle, bool)>,
    toolbar: FormattingToolbar,
}

/// The format buttons that fit on the current row and the disclosure for the rest.
struct FormattingToolbar {
    visible: usize,
    expanded: bool,
}

impl Default for FormattingToolbar {
    fn default() -> Self {
        // The first paint uses the compact row until layout measures its actual width.
        Self {
            visible: 4,
            expanded: false,
        }
    }
}

/// The available presentations of one canonical Markdown draft.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ComposeMode {
    Rich,
    Source,
    Preview,
}

/// A comment composer with a rendered Markdown preview and selection-aware formatting tools.
#[derive(IntoElement)]
pub struct MarkdownEditor {
    area: Entity<TextArea>,
}

impl TextArea {
    /// Returns a draft to its writing surface after changing destination, preserving its selection.
    pub fn resume_writing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.markdown.preview.take().is_some() {
            cx.emit(TextAreaEvent::LayoutChanged);
            cx.notify();
        }
        self.focus(window);
    }
}

impl MarkdownEditor {
    pub fn new(area: Entity<TextArea>) -> Self {
        Self { area }
    }

    /// A presentation of the same draft. The tab padding gives way first in narrow panels.
    fn tab(&self, label: &'static str, selected: bool, mode: ComposeMode) -> impl IntoElement {
        let area = self.area.clone();
        div()
            .id(label)
            .debug_selector(|| label.to_owned())
            .min_w_0()
            .px_1p5()
            .justify_center()
            .whitespace_nowrap()
            .h(px(30.))
            .flex()
            .items_center()
            .border_b_2()
            .border_color(gpui::transparent_black())
            .text_xs()
            .text_color(theme::muted())
            .cursor_pointer()
            .when(selected, |tab| {
                tab.border_color(theme::accent())
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::text())
            })
            .hover(|style| style.text_color(theme::text()))
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                area.update(cx, |area, cx| {
                    if !selected {
                        area.markdown.preview = (mode == ComposeMode::Preview)
                            .then(|| Arc::new(MarkdownDocument::parse(area.text())));
                        if mode != ComposeMode::Preview {
                            area.markdown.github = true;
                            area.markdown.source_mode = mode == ComposeMode::Source;
                            area.refresh_rich();
                            area.normalize_rich_selection();
                            area.marked_range = None;
                        }
                        area.markdown.link = None;
                        area.markdown.pending.clear();
                        cx.emit(TextAreaEvent::LayoutChanged);
                        cx.notify();
                    }
                    area.focus(window);
                });
            })
            .child(label)
    }

    /// A theme-tinted button with a full name and shortcut on hover.
    fn format_button(&self, format: MarkdownFormat, cx: &App) -> impl IntoElement {
        let area = self.area.read(cx);
        let enabled = area.allows_rich_format(format);
        let active = enabled && area.rich_format_active(format);
        let apply = self.apply_format(format);
        let tooltip = match format {
            MarkdownFormat::Bold => "Bold (⌘B)",
            MarkdownFormat::Italic => "Italic (⌘I)",
            MarkdownFormat::Link => "Insert link (⌘K)",
            _ => format.label(),
        };
        div()
            .id(format.label())
            .debug_selector(move || format.label().to_owned())
            .flex_none()
            .size(px(FORMAT_BUTTON_WIDTH))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(theme::muted())
            .when(active, |button| button.bg(theme::selected()))
            .when(!enabled, |button| button.opacity(0.45))
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::hover()).text_color(theme::text()))
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        apply(window, cx);
                    })
            })
            .tooltip(TextTooltip::builder(tooltip))
            .child(
                svg()
                    .path(format.icon())
                    .size(px(15.))
                    .text_color(if active {
                        theme::accent()
                    } else {
                        theme::muted()
                    }),
            )
    }

    /// A toolbar click and its keyboard shortcut edit the same selected range.
    fn apply_format(&self, format: MarkdownFormat) -> impl Fn(&mut Window, &mut App) + 'static {
        let area = self.area.clone();
        move |window, cx| {
            area.update(cx, |area, cx| {
                if matches!(format, MarkdownFormat::Link) && area.markdown.rich.is_some() {
                    area.open_rich_link(window, cx);
                } else {
                    format.apply(area, cx);
                    area.focus(window);
                }
            });
        }
    }

    /// The disclosure is labeled even at the minimum panel width.
    fn more_button(&self, expanded: bool) -> impl IntoElement {
        let area = self.area.clone();
        div()
            .id("more-formats")
            .debug_selector(|| "more-formats".to_owned())
            .flex_none()
            .h(px(FORMAT_BUTTON_WIDTH))
            .w(px(MORE_BUTTON_WIDTH))
            .justify_center()
            .flex()
            .items_center()
            .gap_1()
            .rounded_md()
            .cursor_pointer()
            .text_xs()
            .text_color(theme::muted())
            .when(expanded, |button| {
                button.bg(theme::selected()).text_color(theme::text())
            })
            .hover(|style| style.bg(theme::hover()))
            .tooltip(TextTooltip::builder("More formatting options"))
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                area.update(cx, |area, cx| {
                    area.markdown.toolbar.expanded = !area.markdown.toolbar.expanded;
                    area.focus(window);
                    cx.emit(TextAreaEvent::LayoutChanged);
                    cx.notify();
                });
            })
            .child("More")
            .child(if expanded { "▴" } else { "▾" })
    }

    /// Measures the space allocated by flex layout, which also reflects sidebar and window resizing.
    fn toolbar(&self, visible: usize, expanded: bool, cx: &App) -> impl IntoElement {
        let area = self.area.downgrade();
        let probe = canvas(
            move |bounds, _, cx| {
                let count = visible_format_count(bounds.size.width);
                if count != visible {
                    cx.defer(move |cx| {
                        area.update(cx, |area, cx| {
                            if area.markdown.toolbar.visible == count {
                                return;
                            }
                            area.markdown.toolbar.visible = count;
                            if count == MarkdownFormat::TOOLBAR.len() {
                                area.markdown.toolbar.expanded = false;
                            }
                            cx.emit(TextAreaEvent::LayoutChanged);
                            cx.notify();
                        })
                        .ok();
                    });
                }
            },
            |_, (), _, _| {},
        )
        .absolute()
        .left_0()
        .top_0()
        .size_full();
        div()
            .relative()
            .flex()
            .items_center()
            .justify_end()
            .flex_grow()
            .min_w_0()
            .flex_basis(px(COMPACT_TOOLBAR_WIDTH))
            .children(
                MarkdownFormat::TOOLBAR[..visible]
                    .iter()
                    .map(|&format| self.format_button(format, cx)),
            )
            .when(visible < MarkdownFormat::TOOLBAR.len(), |toolbar| {
                toolbar.child(self.more_button(expanded))
            })
            .child(probe)
    }

    /// Read-only Markdown keeps the composer's Save and Cancel shortcuts available.
    fn preview(&self, document: Arc<MarkdownDocument>, cx: &App) -> impl IntoElement {
        let submit = self.area.clone();
        let cancel = self.area.clone();
        div()
            .key_context("TextArea")
            .track_focus(&self.area.focus_handle(cx))
            .on_action(move |_: &Submit, _, cx| {
                submit.update(cx, |_, cx| cx.emit(TextAreaEvent::Submit));
            })
            .on_action(move |_: &Cancel, _, cx| {
                cancel.update(cx, |_, cx| cx.emit(TextAreaEvent::Cancel));
            })
            .min_w_0()
            .min_h(px(80.))
            .px_3()
            .py_2()
            .text_sm()
            .when(document.is_empty(), |body| {
                body.text_color(theme::muted())
                    .child("Nothing to preview yet.")
            })
            .when(!document.is_empty(), |body| {
                body.child(MarkdownView::new("comment-preview", document))
            })
    }
}

impl RenderOnce for MarkdownEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let area = self.area.read(cx);
        let document = area.markdown.preview.clone();
        let writing = document.is_none();
        let writing_with_focus = writing && area.is_focused(window);
        let visible = area.markdown.toolbar.visible;
        let expanded = area.markdown.toolbar.expanded && visible < MarkdownFormat::TOOLBAR.len();

        // Keep the three modes together and wrap the format buttons as a group in narrow panels.
        let top = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_x_1()
            .px_1p5()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .max_w_full()
                    .child(self.tab(
                        "Rich text",
                        writing && !area.markdown.source_mode,
                        ComposeMode::Rich,
                    ))
                    .child(self.tab(
                        "Markdown",
                        writing && area.markdown.source_mode,
                        ComposeMode::Source,
                    ))
                    .child(self.tab("Preview", !writing, ComposeMode::Preview)),
            )
            .when(writing, |header| {
                header.child(self.toolbar(visible, expanded, cx))
            });
        let header = div()
            .flex()
            .flex_col()
            .bg(theme::panel())
            .border_b_1()
            .border_color(theme::border())
            .child(top)
            .when(writing && expanded, |header| {
                header.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_0p5()
                        .px_1p5()
                        .py_0p5()
                        .border_t_1()
                        .border_color(theme::border())
                        .children(
                            MarkdownFormat::TOOLBAR[visible..]
                                .iter()
                                .map(|&format| self.format_button(format, cx)),
                        ),
                )
            });

        let bold = self.apply_format(MarkdownFormat::Bold);
        let italic = self.apply_format(MarkdownFormat::Italic);
        let link = self.apply_format(MarkdownFormat::Link);
        div()
            .id(("markdown-editor", self.area.entity_id()))
            .flex()
            .flex_col()
            .min_w_0()
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(theme::border_strong())
            .when(writing_with_focus, |editor| {
                editor.border_color(theme::accent())
            })
            .bg(theme::background())
            .when(writing, |editor| {
                editor
                    .key_context("MarkdownWrite")
                    .on_action(move |_: &Bold, window, cx| bold(window, cx))
                    .on_action(move |_: &Italic, window, cx| italic(window, cx))
                    .on_action(move |_: &InsertLink, window, cx| link(window, cx))
            })
            .child(header)
            .children(
                area.markdown
                    .link
                    .as_ref()
                    .map(|link| link.render(&self.area).into_any_element()),
            )
            .child(match document {
                Some(document) => self.preview(document, cx).into_any_element(),
                None => self.area.into_any_element(),
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AssetSource as _, Modifiers, Render, Subscription, TestAppContext};

    use super::*;
    use crate::app_commands::install_app_commands;
    use crate::bundled_assets::BundledIcons;

    /// A composer in a narrow review panel, with the same redraw on layout changes as a comment row.
    struct NarrowComposer {
        width: gpui::Pixels,
        area: Entity<TextArea>,
        _subscription: Subscription,
    }

    impl Render for NarrowComposer {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(self.width)
                .child(MarkdownEditor::new(self.area.clone()))
        }
    }

    /// The compact toolbar wraps, preserves selections across preview and restores editing focus.
    #[gpui::test]
    fn preview_and_format_in_a_narrow_composer(cx: &mut TestAppContext) {
        let area = cx.new(|cx| {
            let mut area = TextArea::comment("Comment", cx);
            area.set_github_markdown(true, cx);
            area
        });
        cx.update(install_app_commands);
        let (_, cx) = cx.add_window_view(|window, cx| {
            area.read(cx).focus(window);
            let subscription = cx.subscribe(&area, |_, _, _: &TextAreaEvent, cx| cx.notify());
            NarrowComposer {
                width: px(194.),
                area: area.clone(),
                _subscription: subscription,
            }
        });
        cx.simulate_input("café");
        cx.simulate_keystrokes("cmd-a cmd-b");
        cx.update(|_, cx| assert_eq!(area.read(cx).text(), "**café**"));

        let click = |cx: &mut gpui::VisualTestContext, label| {
            let bounds = cx.debug_bounds(label).expect("control should be visible");
            assert!(bounds.right() <= px(194.), "{label}: {bounds:?}");
            cx.simulate_click(bounds.center(), Modifiers::default());
            cx.run_until_parked();
        };
        click(cx, "more-formats");
        cx.update(|window, cx| {
            assert!(area.read(cx).markdown.toolbar.expanded);
            assert!(area.read(cx).is_focused(window));
        });

        click(cx, "Preview");
        cx.update(|_, cx| {
            let area = area.read(cx);
            assert_eq!(
                area.markdown.preview.as_deref(),
                Some(&MarkdownDocument::parse("**café**"))
            );
            assert_eq!(&area.text()[area.selected_range.clone()], "café");
        });
        click(cx, "Markdown");
        cx.simulate_keystrokes("cmd-i cmd-k");
        cx.update(|window, cx| {
            let area = area.read(cx);
            assert!(area.is_focused(window));
            assert_eq!(area.text(), "***[café](url)***");
            assert_eq!(&area.text()[area.selected_range.clone()], "url");
        });
    }

    /// Resizing a live editor exposes every fitting button, including after the overflow is open.
    #[gpui::test]
    fn formatting_controls_follow_available_width(cx: &mut TestAppContext) {
        let area = cx.new(|cx| TextArea::comment("Comment", cx));
        let (view, cx) = cx.add_window_view(|_, cx| NarrowComposer {
            width: px(650.),
            _subscription: cx.subscribe(&area, |_, _, _: &TextAreaEvent, cx| cx.notify()),
            area: area.clone(),
        });
        let resize = |width, cx: &mut gpui::VisualTestContext| {
            cx.update(|_, cx| {
                view.update(cx, |view, cx| {
                    view.width = px(width);
                    cx.notify();
                });
            });
            cx.run_until_parked();
        };
        let visible = |cx: &mut gpui::VisualTestContext| {
            cx.update(|_, cx| area.read(cx).markdown.toolbar.visible)
        };

        cx.run_until_parked();
        assert_eq!(visible(cx), MarkdownFormat::TOOLBAR.len());
        let checklist = cx.debug_bounds("Checklist").expect("last format fits");
        assert!(checklist.right() <= px(650.));

        resize(425., cx);
        let count = visible(cx);
        assert!((1..MarkdownFormat::TOOLBAR.len()).contains(&count));
        let more = cx.debug_bounds("more-formats").unwrap();
        cx.simulate_click(more.center(), Modifiers::default());
        cx.run_until_parked();
        assert!(cx.update(|_, cx| area.read(cx).markdown.toolbar.expanded));
        let overflow = cx
            .debug_bounds("Checklist")
            .expect("last format in overflow");
        assert!(overflow.top() >= more.bottom());
        assert!(overflow.right() <= px(425.));

        resize(650., cx);
        assert_eq!(visible(cx), MarkdownFormat::TOOLBAR.len());
        assert!(!cx.update(|_, cx| area.read(cx).markdown.toolbar.expanded));
        resize(425., cx);
        assert_eq!(visible(cx), count);
    }

    /// Every formatting control resolves to an embedded icon in the release binary.
    #[test]
    fn all_formatting_icons_are_bundled() {
        for format in MarkdownFormat::TOOLBAR {
            assert!(BundledIcons.load(format.icon()).unwrap().is_some());
        }
    }
}
