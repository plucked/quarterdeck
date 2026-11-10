use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, EventEmitter, FocusHandle, Focusable,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, SharedString, Window,
    div, prelude::*,
};

use super::line_mode::LineMode;
use super::text_area_actions::{
    Backspace, Cancel, Cut, Delete, DeleteToLineStart, DeleteWordLeft, Down, Left, LineEnd,
    LineStart, Newline, Paste, Right, SelectAll, SelectDown, SelectLeft, SelectRight,
    SelectToLineEnd, SelectToLineStart, SelectUp, SelectWordLeft, SelectWordRight,
    ShowCharacterPalette, Submit, TEXT_AREA_CONTEXT, TextEnd, TextStart, Up, WordLeft, WordRight,
};
use super::text_area_element::TextAreaElement;
use super::text_area_event::TextAreaEvent;
use super::text_area_layout::TextAreaLayout;
use super::text_boundaries::{
    line_end, line_start, next_boundary, next_word_end, offset_within, previous_boundary,
    previous_word_start, word_range_at,
};
use crate::app_commands::Copy;
use crate::theme;

/// Clicks that select a word.
const DOUBLE_CLICK: usize = 2;

/// A plain-text editor, multiline or a field of one line. The owner creates
/// it with `cx.new(|cx| TextArea::new(placeholder, cx))` (or
/// [`TextArea::single_line`]), renders the entity, focuses it, subscribes to
/// [`TextAreaEvent`]s and reads [`TextArea::text`].
///
/// Offsets are UTF-8 byte offsets into the text, always on character
/// boundaries; the system input methods speak UTF-16 (`text_area_input.rs`).
pub struct TextArea {
    focus: FocusHandle,
    text: String,
    placeholder: SharedString,
    /// Whether Return inserts a newline or submits, and whether the text
    /// may hold newlines at all.
    line_mode: LineMode,
    /// The selection; empty for a caret.
    pub(super) selected_range: Range<usize>,
    /// Whether the caret is at the selection's start rather than its end.
    pub(super) selection_reversed: bool,
    /// Text an input method is still composing.
    pub(super) marked_range: Option<Range<usize>>,
    /// The layout and bounds of the last paint, for hit tests and the
    /// input method's caret position.
    pub(super) last_layout: Option<TextAreaLayout>,
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
}

impl EventEmitter<TextAreaEvent> for TextArea {}

impl Focusable for TextArea {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextArea {
    /// An empty multiline text area showing `placeholder` until something
    /// is typed. Return inserts a newline; cmd-enter submits.
    pub fn new(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self::with_line_mode(placeholder, LineMode::Multiline, cx)
    }

    /// An empty field of one line showing `placeholder` until something is
    /// typed: Return (or Enter on the keypad) submits like cmd-enter, and
    /// newlines in pasted or input-method text are dropped, so the text is
    /// always one line.
    pub fn single_line(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self::with_line_mode(placeholder, LineMode::SingleLine, cx)
    }

    /// An empty text area of `line_mode` showing `placeholder`.
    fn with_line_mode(
        placeholder: impl Into<SharedString>,
        line_mode: LineMode,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus: cx.focus_handle(),
            text: String::new(),
            placeholder: placeholder.into(),
            line_mode,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn placeholder(&self) -> &SharedString {
        &self.placeholder
    }

    /// Replaces the whole text, with the caret at its end (in a
    /// single-line area, without its newlines).
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = text.into();
        self.text = self.line_mode.inserted_text(&text).into_owned();
        self.selected_range = self.text.len()..self.text.len();
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(TextAreaEvent::Changed);
        cx.notify();
    }

    /// Focuses the text area so typing goes to it.
    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus);
    }

    pub(super) fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    /// Where the caret is: the selection's end, or its start when reversed.
    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Replaces `range` with `new_text` (in a single-line area, without its
    /// newlines), putting the caret after it. Returns where the inserted
    /// text now is.
    pub(super) fn replace(
        &mut self,
        range: Range<usize>,
        new_text: &str,
        cx: &mut Context<Self>,
    ) -> Range<usize> {
        let new_text = self.line_mode.inserted_text(new_text);
        self.text.replace_range(range.clone(), &new_text);
        let caret = range.start + new_text.len();
        self.selected_range = caret..caret;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.emit(TextAreaEvent::Changed);
        cx.notify();
        range.start..caret
    }

    /// Replaces the selection (or the composing text) with `new_text`.
    fn replace_selection(&mut self, new_text: &str, cx: &mut Context<Self>) {
        let range = self
            .marked_range
            .clone()
            .unwrap_or_else(|| self.selected_range.clone());
        self.replace(range, new_text, cx);
    }

    /// Where Up moves the caret: one visual row up, or in a single-line
    /// area the text's start, as in a macOS text field.
    fn offset_above(&self) -> usize {
        match self.line_mode {
            LineMode::Multiline => self.vertical_offset(-1.),
            LineMode::SingleLine => 0,
        }
    }

    /// Where Down moves the caret: one visual row down, or in a single-line
    /// area the text's end.
    fn offset_below(&self) -> usize {
        match self.line_mode {
            LineMode::Multiline => self.vertical_offset(1.),
            LineMode::SingleLine => self.text.len(),
        }
    }

    /// Puts the caret at `offset`, dropping the selection.
    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    /// Moves the caret to `offset`, keeping the selection's other end.
    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    /// The offset one visual row above (`rows` = -1) or below (1) the caret.
    fn vertical_offset(&self, rows: f32) -> usize {
        let Some(layout) = &self.last_layout else {
            return self.cursor_offset();
        };
        let caret = layout.position_for_offset(self.cursor_offset());
        let target_y = caret.y + layout.line_height * (rows + 0.5);
        if target_y < Pixels::ZERO {
            return 0;
        }
        if target_y > layout.height {
            return self.text.len();
        }
        // The layout may be the placeholder's or predate an edit.
        offset_within(
            &self.text,
            layout.offset_for_position(gpui::point(caret.x, target_y)),
        )
    }

    /// The offset under a window position, from the last paint.
    /// The text offset under `position`. The layout is the last drawn one,
    /// which shows the placeholder while the text is empty and can predate
    /// an edit, so its offset is clamped into the text (slicing past it, or
    /// inside a character, panics).
    fn offset_for_window_position(&self, position: Point<Pixels>) -> usize {
        let (Some(layout), Some(bounds)) = (&self.last_layout, self.last_bounds) else {
            return self.text.len();
        };
        offset_within(
            &self.text,
            layout.offset_for_position(position - bounds.origin),
        )
    }

    fn on_backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let start = previous_boundary(&self.text, self.cursor_offset());
            self.replace(start..self.cursor_offset(), "", cx);
        } else {
            self.replace_selection("", cx);
        }
    }

    fn on_delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let end = next_boundary(&self.text, self.cursor_offset());
            self.replace(self.cursor_offset()..end, "", cx);
        } else {
            self.replace_selection("", cx);
        }
    }

    fn on_delete_word_left(&mut self, _: &DeleteWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let end = self.cursor_offset();
        self.replace(previous_word_start(&self.text, end)..end, "", cx);
    }

    fn on_delete_to_line_start(
        &mut self,
        _: &DeleteToLineStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let end = self.cursor_offset();
        self.replace(line_start(&self.text, end)..end, "", cx);
    }

    fn on_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let target = if self.selected_range.is_empty() {
            previous_boundary(&self.text, self.cursor_offset())
        } else {
            self.selected_range.start
        };
        self.move_to(target, cx);
    }

    fn on_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let target = if self.selected_range.is_empty() {
            next_boundary(&self.text, self.cursor_offset())
        } else {
            self.selected_range.end
        };
        self.move_to(target, cx);
    }

    fn on_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.offset_above(), cx);
    }

    fn on_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.offset_below(), cx);
    }

    fn on_word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(previous_word_start(&self.text, self.cursor_offset()), cx);
    }

    fn on_word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(next_word_end(&self.text, self.cursor_offset()), cx);
    }

    fn on_line_start(&mut self, _: &LineStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(line_start(&self.text, self.cursor_offset()), cx);
    }

    fn on_line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(line_end(&self.text, self.cursor_offset()), cx);
    }

    fn on_text_start(&mut self, _: &TextStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn on_text_end(&mut self, _: &TextEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.text.len(), cx);
    }

    fn on_select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(previous_boundary(&self.text, self.cursor_offset()), cx);
    }

    fn on_select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(next_boundary(&self.text, self.cursor_offset()), cx);
    }

    fn on_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.offset_above(), cx);
    }

    fn on_select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.offset_below(), cx);
    }

    fn on_select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(previous_word_start(&self.text, self.cursor_offset()), cx);
    }

    fn on_select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(next_word_end(&self.text, self.cursor_offset()), cx);
    }

    fn on_select_to_line_start(
        &mut self,
        _: &SelectToLineStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(line_start(&self.text, self.cursor_offset()), cx);
    }

    fn on_select_to_line_end(
        &mut self,
        _: &SelectToLineEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(line_end(&self.text, self.cursor_offset()), cx);
    }

    fn on_select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.text.len();
        self.selection_reversed = false;
        cx.notify();
    }

    /// Return: a newline in a multiline area; in a single-line one, the
    /// owner takes the text, as with cmd-enter.
    fn on_newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        match self.line_mode {
            LineMode::Multiline => self.replace_selection("\n", cx),
            LineMode::SingleLine => cx.emit(TextAreaEvent::Submit),
        }
    }

    fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_selection(&text.replace("\r\n", "\n"), cx);
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            let selected = self.text[self.selected_range.clone()].to_owned();
            cx.write_to_clipboard(ClipboardItem::new_string(selected));
        }
    }

    fn on_cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            let selected = self.text[self.selected_range.clone()].to_owned();
            cx.write_to_clipboard(ClipboardItem::new_string(selected));
            self.replace_selection("", cx);
        }
    }

    /// A press places the caret (shift: extends the selection; two clicks: the word).
    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus);
        let offset = self.offset_for_window_position(event.position);
        match (event.click_count >= DOUBLE_CLICK, event.modifiers.shift) {
            (true, _) => {
                self.selected_range = word_range_at(&self.text, offset);
                self.selection_reversed = false;
                cx.notify();
            }
            (false, true) => self.select_to(offset, cx),
            (false, false) => {
                self.is_selecting = true;
                self.move_to(offset, cx);
            }
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.offset_for_window_position(event.position), cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }
}

impl Render for TextArea {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context(TEXT_AREA_CONTEXT)
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::on_backspace))
            .on_action(cx.listener(Self::on_delete))
            .on_action(cx.listener(Self::on_delete_word_left))
            .on_action(cx.listener(Self::on_delete_to_line_start))
            .on_action(cx.listener(Self::on_left))
            .on_action(cx.listener(Self::on_right))
            .on_action(cx.listener(Self::on_up))
            .on_action(cx.listener(Self::on_down))
            .on_action(cx.listener(Self::on_word_left))
            .on_action(cx.listener(Self::on_word_right))
            .on_action(cx.listener(Self::on_line_start))
            .on_action(cx.listener(Self::on_line_end))
            .on_action(cx.listener(Self::on_text_start))
            .on_action(cx.listener(Self::on_text_end))
            .on_action(cx.listener(Self::on_select_left))
            .on_action(cx.listener(Self::on_select_right))
            .on_action(cx.listener(Self::on_select_up))
            .on_action(cx.listener(Self::on_select_down))
            .on_action(cx.listener(Self::on_select_word_left))
            .on_action(cx.listener(Self::on_select_word_right))
            .on_action(cx.listener(Self::on_select_to_line_start))
            .on_action(cx.listener(Self::on_select_to_line_end))
            .on_action(cx.listener(Self::on_select_all))
            .on_action(cx.listener(Self::on_newline))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(|_, _: &Submit, _, cx| cx.emit(TextAreaEvent::Submit)))
            .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(TextAreaEvent::Cancel)))
            .on_action(|_: &ShowCharacterPalette, window, _| window.show_character_palette())
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .w_full()
            .px_2()
            .py_1p5()
            .rounded_md()
            .border_1()
            .border_color(theme::border())
            .bg(theme::background())
            .text_sm()
            .child(TextAreaElement::new(cx.entity()))
    }
}
