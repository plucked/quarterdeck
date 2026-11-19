use std::ops::Range;

use gpui::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point};

use super::comment_history::EditSource;
use super::text_area::TextArea;
use super::text_boundaries::offset_within;

impl TextArea {
    /// An edit that replaces the active marked range continues the same input-method composition.
    fn input_source(&self, range: &Range<usize>, default: EditSource) -> EditSource {
        if self.marked_range.as_ref() == Some(range) {
            EditSource::Composition
        } else {
            default
        }
    }

    /// The UTF-16 offset of UTF-8 offset `offset`.
    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.editing_text()[..self.to_visible(offset.min(self.text().len()))]
            .chars()
            .map(char::len_utf16)
            .sum()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        if *range == self.range_to_utf16(&self.selected_range) {
            return self.selected_range.clone();
        }
        self.source_range(
            offset_from_utf16(self.editing_text(), range.start)
                ..offset_from_utf16(self.editing_text(), range.end),
        )
    }
}

/// Converts a UTF-16 offset in the supplied text to a UTF-8 character boundary.
fn offset_from_utf16(text: &str, offset: usize) -> usize {
    let mut utf16 = 0;
    for (byte, character) in text.char_indices() {
        if utf16 >= offset {
            return byte;
        }
        utf16 += character.len_utf16();
    }
    text.len()
}

/// The system's text input: typed characters, input method composition and
/// the caret position the input method's window follows. The system speaks
/// UTF-16 offsets.
impl EntityInputHandler for TextArea {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(if self.secret {
            "*".repeat(self.text()[range].encode_utf16().count())
        } else {
            self.editing_text()[self.visible_range(&range)].to_owned()
        })
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        let source = self.input_source(&range, EditSource::Typing);
        self.edit_input(range, new_text, source, cx);
        if source == EditSource::Composition {
            self.finish_edit_selection();
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());
        // A single-line area drops newlines, so what went in can be shorter than `new_text`.
        let source = self.input_source(&range, EditSource::BeginComposition);
        let inserted = self.edit_input(range, new_text, source, cx);

        // The composing text stays marked, with the input method's own selection inside it.
        self.marked_range = (!inserted.is_empty()).then(|| inserted.clone());
        if let Some(selected) = new_selected_range_utf16 {
            let marked = &self.text()[inserted.clone()];
            self.selected_range = inserted.start + offset_from_utf16(marked, selected.start)
                ..inserted.start + offset_from_utf16(marked, selected.end);
        }
        self.finish_edit_selection();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let start = layout.position_for_offset(self.to_visible(range.start));
        let end = layout.position_for_offset(self.to_visible(range.end));
        Some(Bounds::from_corners(
            bounds.origin + start,
            bounds.origin + point(end.x, end.y + layout.line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let layout = self.last_layout.as_ref()?;
        // The layout may be the placeholder's or predate an edit.
        let offset = offset_within(
            self.editing_text(),
            layout.offset_for_position(position - bounds.origin),
        );
        Some(self.offset_to_utf16(self.to_source(offset, true)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_area::bind_text_area_keys;

    /// The input method sees displayed UTF-16 positions even around hidden markers and emoji.
    #[gpui::test]
    fn composition_uses_the_rich_selection(cx: &mut gpui::TestAppContext) {
        cx.update(bind_text_area_keys);
        let (editor, visual) = cx.add_window_view(|window, cx| {
            let mut area = TextArea::comment("", cx);
            area.set_github_markdown(true, cx);
            area.set_text("**😀ab**", cx);
            area.focus(window);
            area
        });
        visual.update(|window, app| {
            editor.update(app, |area, cx| {
                area.replace_and_mark_text_in_range(Some(2..4), "かな", Some(0..2), window, cx);
                assert_eq!(area.marked_text_range(window, cx), Some(2..4));
                let mut actual = None;
                assert_eq!(
                    area.text_for_range(2..4, &mut actual, window, cx)
                        .as_deref(),
                    Some("かな")
                );
                assert_eq!(actual, Some(2..4));
                area.replace_text_in_range(None, "仮名", window, cx);
                assert_eq!(area.text(), "**😀仮名**");
                assert_eq!(
                    area.selected_text_range(false, window, cx).unwrap().range,
                    4..4
                );
            });
        });
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), "**😀ab**"));
    }
    /// Marked input can be nested inside delimiters added by an active typing style.
    #[gpui::test]
    fn formatted_composition_undoes_its_delimiters(cx: &mut gpui::TestAppContext) {
        cx.update(bind_text_area_keys);
        let (editor, visual) = cx.add_window_view(|window, cx| {
            let mut area = TextArea::comment("", cx);
            area.set_github_markdown(true, cx);
            super::super::markdown_format::MarkdownFormat::Bold.apply(&mut area, cx);
            area.focus(window);
            area
        });
        visual.update(|window, app| {
            editor.update(app, |area, cx| {
                area.replace_and_mark_text_in_range(None, "o", Some(1..1), window, cx);
                area.replace_and_mark_text_in_range(None, "お", Some(1..1), window, cx);
                area.replace_text_in_range(None, "音", window, cx);
                assert_eq!(area.text(), "**音**");
            });
        });
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), ""));
        visual.simulate_keystrokes("cmd-shift-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), "**音**"));
    }
}
