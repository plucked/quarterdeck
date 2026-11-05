use gpui::{App, KeyBinding, actions};

/// The key context of a text area; the queue's single-letter keys stay out of it.
pub const TEXT_AREA_CONTEXT: &str = "TextArea";

actions!(
    text_area,
    [
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteToLineStart,
        Left,
        Right,
        Up,
        Down,
        WordLeft,
        WordRight,
        LineStart,
        LineEnd,
        TextStart,
        TextEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectWordLeft,
        SelectWordRight,
        SelectToLineStart,
        SelectToLineEnd,
        SelectAll,
        Newline,
        Paste,
        Cut,
        Submit,
        Cancel,
        ShowCharacterPalette
    ]
);

/// Binds the editing keys inside a text area (macOS conventions).
pub fn bind_text_area_keys(cx: &mut App) {
    let context = Some(TEXT_AREA_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, context),
        KeyBinding::new("shift-backspace", Backspace, context),
        KeyBinding::new("delete", Delete, context),
        KeyBinding::new("alt-backspace", DeleteWordLeft, context),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, context),
        KeyBinding::new("left", Left, context),
        KeyBinding::new("right", Right, context),
        KeyBinding::new("up", Up, context),
        KeyBinding::new("down", Down, context),
        KeyBinding::new("alt-left", WordLeft, context),
        KeyBinding::new("alt-right", WordRight, context),
        KeyBinding::new("cmd-left", LineStart, context),
        KeyBinding::new("cmd-right", LineEnd, context),
        KeyBinding::new("home", LineStart, context),
        KeyBinding::new("end", LineEnd, context),
        KeyBinding::new("cmd-up", TextStart, context),
        KeyBinding::new("cmd-down", TextEnd, context),
        KeyBinding::new("shift-left", SelectLeft, context),
        KeyBinding::new("shift-right", SelectRight, context),
        KeyBinding::new("shift-up", SelectUp, context),
        KeyBinding::new("shift-down", SelectDown, context),
        KeyBinding::new("alt-shift-left", SelectWordLeft, context),
        KeyBinding::new("alt-shift-right", SelectWordRight, context),
        KeyBinding::new("cmd-shift-left", SelectToLineStart, context),
        KeyBinding::new("cmd-shift-right", SelectToLineEnd, context),
        KeyBinding::new("cmd-a", SelectAll, context),
        KeyBinding::new("enter", Newline, context),
        KeyBinding::new("shift-enter", Newline, context),
        KeyBinding::new("cmd-v", Paste, context),
        KeyBinding::new("cmd-x", Cut, context),
        KeyBinding::new("cmd-enter", Submit, context),
        KeyBinding::new("escape", Cancel, context),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, context),
    ]);
}
