# text_area

A plain-text editor for the views that take text from the user: multiline, such as review comments, or a field of one line, such as the settings window's token, repository and branch pattern fields, where Return submits and pasted newlines are dropped. It handles typing (with the system's input methods), selection by mouse and keyboard, word and line movement, copy, cut and paste, and submit or cancel keys that it reports to its owner. Its owner reads the source text on submission.

A containing search picker can capture its Up/Down cursor actions to move through results while keeping typing focused on the field.

GitHub review composers open in Rich text. The formatted surface projects styles from the underlying Markdown and maps displayed text positions back to the draft. Markdown exposes the editable source and Preview uses the renderer for posted comments. All writing surfaces share the draft and its edit history. The toolbar measures its available width and moves the remaining controls into More. In Rich text, heading, quote, list and code block controls reflect the selection. Clicking an active control removes that format. Switching list types preserves the selected text. Return continues the current list, increments ordered markers and opens new tasks unchecked. Return on an empty item exits the list. Shift-Return inserts a line break inside the item. Clicking a format returns focus to the draft. Layout changes let virtualized comment rows resize.

Single-line fields keep one visual row. Text is shaped without wrapping, clipped to the field and shifted horizontally to keep the caret visible; mouse selection and input-method coordinates use that same painted origin.

Comment editors keep a bounded undo history for typing, deletion, pasting and Markdown formatting. ⌘Z and ⇧⌘Z restore the text and its selection. New edits discard the redo branch, and input-method composition commits as one change.
