use std::collections::VecDeque;
use std::ops::Range;

/// Bounds both the number of undo steps and the bytes of replaced and inserted text.
const MAX_EDITS: usize = 100;
const MAX_TEXT_BYTES: usize = 1024 * 1024;

/// Consecutive typed letters form a word. Commands and input-method compositions each form one step.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditSource {
    Typing,
    Command,
    BeginComposition,
    Composition,
}

/// Both endpoints of a selection, including which one holds the caret.
#[derive(Clone)]
pub(super) struct EditSelection {
    pub range: Range<usize>,
    pub reversed: bool,
}

/// The text on either side of a single reversible edit and its selections.
struct CommentEdit {
    start: usize,
    removed: String,
    inserted: String,
    before: EditSelection,
    after: EditSelection,
}

/// Session-local edits for one comment, with a cursor between applied and redoable edits.
#[derive(Default)]
pub(super) struct CommentHistory {
    edits: VecDeque<CommentEdit>,
    applied: usize,
    bytes: usize,
    /// The kind of edit that can still absorb more typing or an IME revision.
    extendable: Option<EditSource>,
}

impl CommentHistory {
    pub fn can_undo(&self) -> bool {
        self.applied > 0
    }

    pub fn can_redo(&self) -> bool {
        self.applied < self.edits.len()
    }

    /// Records one replacement, dropping any abandoned redo branch.
    pub fn record(
        &mut self,
        start: usize,
        removed: &str,
        inserted: &str,
        before: EditSelection,
        source: EditSource,
    ) {
        // An IME can begin by marking the existing selection verbatim, then replace it.
        // Keep its starting selection independent of the edit that originally typed those words.
        if removed == inserted && (source != EditSource::BeginComposition || inserted.is_empty()) {
            return;
        }
        // A larger edit cannot be retained. Start fresh so older deltas cannot reach across it.
        if removed.len().saturating_add(inserted.len()) > MAX_TEXT_BYTES {
            *self = Self::default();
            return;
        }
        while self.edits.len() > self.applied {
            let abandoned = self.edits.pop_back().expect("there is a redoable edit");
            self.bytes -= abandoned.removed.len() + abandoned.inserted.len();
        }
        let caret = start + inserted.len();
        let after = EditSelection {
            range: caret..caret,
            reversed: false,
        };
        let typing = source == EditSource::Typing
            && removed.is_empty()
            && !inserted.is_empty()
            && inserted.chars().all(|character| !character.is_whitespace());
        // An IME revises its marked text inside the original edit, possibly between rich style delimiters.
        if let Some(last) = self.edits.back_mut()
            && self.extendable == Some(EditSource::Composition)
            && source == EditSource::Composition
            && let Some(relative) = start.checked_sub(last.start)
            && let Some(end) = relative.checked_add(removed.len())
            && last.inserted.get(relative..end) == Some(removed)
        {
            last.inserted.replace_range(relative..end, inserted);
            // Release a long intermediate composition when its committed replacement is short.
            last.inserted.shrink_to_fit();
            self.bytes = self.bytes - removed.len() + inserted.len();
            last.after = after;
        } else if let Some(last) = self.edits.back_mut()
            && self.extendable == Some(EditSource::Typing)
            && typing
            && last.start + last.inserted.len() == start
        {
            last.inserted.push_str(inserted);
            self.bytes += inserted.len();
            last.after = after;
        } else {
            self.bytes += removed.len() + inserted.len();
            self.edits.push_back(CommentEdit {
                start,
                removed: removed.into(),
                inserted: inserted.into(),
                before,
                after,
            });
        }
        self.extendable = match source {
            EditSource::Typing if typing => Some(EditSource::Typing),
            EditSource::BeginComposition | EditSource::Composition => Some(EditSource::Composition),
            EditSource::Typing | EditSource::Command => None,
        };

        // Evict the oldest complete changes, including a word that grew past the budget.
        while self.edits.len() > MAX_EDITS || self.bytes > MAX_TEXT_BYTES {
            let oldest = self
                .edits
                .pop_front()
                .expect("over-budget history has edits");
            self.bytes -= oldest.removed.len() + oldest.inserted.len();
        }
        self.applied = self.edits.len();
    }

    /// Formatting and input methods may put the caret inside the inserted text.
    pub fn finish_selection(&mut self, selection: EditSelection) {
        if self.applied == self.edits.len()
            && let Some(last) = self.edits.back_mut()
        {
            last.after = selection;
        }
    }

    /// Restores the bytes and selection preceding the last applied change.
    pub fn undo(&mut self, text: &mut String) -> Option<EditSelection> {
        let index = self.applied.checked_sub(1)?;
        let edit = &self.edits[index];
        text.replace_range(edit.start..edit.start + edit.inserted.len(), &edit.removed);
        self.applied = index;
        self.extendable = None;
        Some(edit.before.clone())
    }

    /// Reapplies the next undone change and restores its resulting selection.
    pub fn redo(&mut self, text: &mut String) -> Option<EditSelection> {
        let edit = self.edits.get(self.applied)?;
        text.replace_range(edit.start..edit.start + edit.removed.len(), &edit.inserted);
        self.applied += 1;
        self.extendable = None;
        Some(edit.after.clone())
    }
}

#[cfg(test)]
mod tests {
    use gpui::{EntityInputHandler as _, TestAppContext};

    use super::*;
    use crate::text_area::markdown_format::MarkdownFormat;
    use crate::text_area::{TextArea, bind_text_area_keys};

    /// The comment editor's real keymap undoes toolbar commands atomically and restores UTF-8 selections.
    #[gpui::test]
    fn formatting_and_typing_undo_redo_in_the_comment_field(cx: &mut TestAppContext) {
        cx.update(bind_text_area_keys);
        let (editor, visual) = cx.add_window_view(|window, cx| {
            let area = TextArea::comment("", cx);
            area.focus(window);
            area
        });
        // Programmatic text starts a fresh history. Formatting trims spaces while Undo restores them.
        editor.update(visual, |area, cx| {
            area.set_text("  café  ", cx);
            area.selected_range = 0..area.text().len();
            area.selection_reversed = true;
            MarkdownFormat::Bold.apply(area, cx);
        });
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| {
            assert_eq!(area.text(), "  café  ");
            assert_eq!(area.selected_range, 0..area.text().len());
            assert!(area.selection_reversed);
        });
        visual.simulate_keystrokes("cmd-shift-z");
        editor.update(visual, |area, _| {
            assert_eq!(area.text(), "  **café**  ");
            assert_eq!(&area.text()[area.selected_range.clone()], "café");
        });

        // Native typing coalesces adjacent letters. A new edit after Undo starts another branch.
        editor.update(visual, |area, cx| area.set_text("", cx));
        visual.simulate_input("a");
        visual.simulate_input("b");
        visual.simulate_input("c");
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), ""));
        visual.simulate_keystrokes("cmd-shift-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), "abc"));
        visual.simulate_keystrokes("cmd-z");
        visual.simulate_input("new");
        visual.simulate_keystrokes("cmd-shift-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), "new"));
    }

    /// Revised and committed IME text is one undo step even with multibyte characters after ASCII.
    #[gpui::test]
    fn comment_undo_restores_the_precomposition_text(cx: &mut TestAppContext) {
        cx.update(bind_text_area_keys);
        let (editor, visual) = cx.add_window_view(|window, cx| {
            let area = TextArea::comment("", cx);
            area.focus(window);
            area
        });
        visual.update(|window, app| {
            editor.update(app, |area, cx| {
                area.set_text("prefix ", cx);
                area.replace_and_mark_text_in_range(None, "か", Some(1..1), window, cx);
                assert_eq!(area.selected_range, 10..10);
                area.replace_and_mark_text_in_range(None, "かな", Some(0..2), window, cx);
                area.replace_text_in_range(None, "仮名", window, cx);
            });
        });
        visual.simulate_keystrokes("cmd-z");
        editor.update(visual, |area, _| assert_eq!(area.text(), "prefix "));
        visual.simulate_keystrokes("cmd-shift-z");
        editor.update(visual, |area, _| {
            assert_eq!(area.text(), "prefix 仮名");
            assert_eq!(area.selected_range, area.text().len()..area.text().len());
        });
    }

    /// Marking unchanged text begins a separate composition even when that text was just typed.
    #[test]
    fn composing_over_typed_text_keeps_the_original_edit() {
        let mut history = CommentHistory::default();
        let selection = |range| EditSelection {
            range,
            reversed: false,
        };
        history.record(0, "", "a", selection(0..0), EditSource::Typing);
        history.record(0, "a", "a", selection(0..1), EditSource::BeginComposition);
        history.record(0, "a", "á", selection(0..1), EditSource::Composition);

        let mut text = String::from("á");
        assert_eq!(history.undo(&mut text).unwrap().range, 0..1);
        assert_eq!(text, "a");
        assert_eq!(history.undo(&mut text).unwrap().range, 0..0);
        assert_eq!(text, "");
    }

    /// Old deltas are evicted and a single oversized replacement clears history safely.
    #[test]
    fn history_has_step_and_byte_limits() {
        let mut history = CommentHistory::default();
        let caret = |offset| EditSelection {
            range: offset..offset,
            reversed: false,
        };
        for offset in 0..=MAX_EDITS {
            history.record(offset, "", "a", caret(offset), EditSource::Command);
        }
        let mut text = "a".repeat(MAX_EDITS + 1);
        while history.undo(&mut text).is_some() {}
        assert_eq!(text, "a");

        // Older edits cannot undo across an unrecorded large edit.
        history.record(
            1,
            "",
            &"x".repeat(MAX_TEXT_BYTES + 1),
            caret(1),
            EditSource::Command,
        );
        assert!(history.undo(&mut text).is_none());
        assert!(history.redo(&mut text).is_none());
    }
}
