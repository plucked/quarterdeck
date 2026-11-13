# text_area

A plain-text editor for the views that take text from the user: multiline, such as review comments, or a field of one line, such as the settings window's token, repository and branch pattern fields, where Return submits and pasted newlines are dropped. It handles typing (with the system's input methods), selection by mouse and keyboard, word and line movement, copy, cut and paste, and submit or cancel keys that it reports to its owner. It does not render Markdown or know what the text is for; the owner reads the text and decides what to do with it.

A containing search picker can capture its Up/Down cursor actions to move through results while keeping typing focused on the field.
