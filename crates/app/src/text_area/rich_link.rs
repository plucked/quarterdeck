use std::ops::Range;

use gpui::{App, Context, Entity, IntoElement, Subscription, Window, div, prelude::*};
use qd_markdown::InlineStyle;

use super::{TextArea, TextAreaEvent};
use crate::controls::Button;
use crate::theme;

/// A link destination field temporarily attached to a rich draft, keeping the label's source range.
pub(super) struct RichLink {
    input: Entity<TextArea>,
    target: Range<usize>,
    label: String,
    _subscription: Subscription,
}

impl TextArea {
    /// Edits the destination separately from the styled link label and returns to the same selection.
    pub(super) fn open_rich_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.allows_rich_format(super::markdown_format::MarkdownFormat::Link) {
            return;
        }
        let Some(rich) = self.markdown.rich.as_ref() else {
            return;
        };
        let selected = self.selected_range.clone();
        let existing = rich.marks.iter().find(|mark| {
            mark.style == InlineStyle::LINK
                && mark.outer.start <= selected.start
                && selected.end <= mark.outer.end
        });
        let (target, label, url) = match existing {
            Some(mark) => (
                mark.outer.clone(),
                self.text()[mark.inner.clone()].to_owned(),
                mark.link_destination.clone().unwrap_or_default(),
            ),
            None => (
                selected.clone(),
                self.text()[selected].to_owned(),
                String::new(),
            ),
        };
        let input = cx.new(|cx| {
            let mut input = TextArea::single_line("https://…", cx);
            input.set_text(url, cx);
            input.selected_range = 0..input.text().len();
            input
        });
        let subscription =
            cx.subscribe_in(&input, window, |area, _, event, window, cx| match event {
                TextAreaEvent::Submit => area.finish_rich_link(true, window, cx),
                TextAreaEvent::Cancel => area.finish_rich_link(false, window, cx),
                _ => {}
            });
        input.read(cx).focus(window);
        self.markdown.link = Some(RichLink {
            input,
            target,
            label,
            _subscription: subscription,
        });
        cx.emit(TextAreaEvent::LayoutChanged);
        cx.notify();
    }

    fn finish_rich_link(&mut self, save: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(link) = self.markdown.link.take()
            && save
        {
            let url = link.input.read(cx).text().trim().to_owned();
            if url.is_empty() {
                self.markdown.link = Some(link);
                return;
            }
            let label = if link.label.is_empty() {
                &url
            } else {
                &link.label
            };
            // Angle brackets admit spaces and parentheses without changing the destination.
            let destination = if url.contains([' ', '(', ')', '<', '>']) {
                format!("<{}>", url.replace('<', "%3C").replace('>', "%3E"))
            } else {
                url.clone()
            };
            let selected = link.target.start + 1..link.target.start + 1 + label.len();
            self.replace(link.target, &format!("[{label}]({destination})"), cx);
            self.selected_range = selected;
            self.finish_edit_selection();
        }
        self.focus(window);
        cx.emit(TextAreaEvent::LayoutChanged);
        cx.notify();
    }
}

impl RichLink {
    pub fn render(&self, area: &Entity<TextArea>) -> impl IntoElement {
        let save = area.clone();
        let cancel = area.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .text_xs()
                    .text_color(theme::muted())
                    .child("Link address"),
            )
            .child(self.input.clone())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_1()
                    .child(Button::new("cancel-link", "Cancel").ghost().on_click(
                        move |_, window, cx: &mut App| {
                            cancel.update(cx, |area, cx| area.finish_rich_link(false, window, cx));
                        },
                    ))
                    .child(Button::new("apply-link", "Apply").primary().on_click(
                        move |_, window, cx| {
                            save.update(cx, |area, cx| area.finish_rich_link(true, window, cx));
                        },
                    )),
            )
    }
}
