# controls

Small reusable UI components that look and behave the same wherever a view places them: buttons, checkboxes, option rows, fold chevrons, resize handles, tooltips, section labels, pills, pulsing status lines, animated numeric labels and repository monograms, plus the helper that turns an option's pick into a call on its view. Each one draws only what it is given and reports clicks or drags through the handler it was handed; it knows nothing about pull requests, the queue or the settings. `NumberFlow` and `ProgressBar` retain only transient animation state in GPUI's element lifecycle. Larger pieces with state of their own (the scrollbar, the text area, Markdown, filter chips) are their own modules beside this one.

Numbers in buttons, segments, pills, section labels and status lines automatically use [NumberFlow](number_flow/README.md). Direct numeric labels use the same control, including formatted counts, percentages, sizes, dates, line numbers and progress. Source code, editable text and Markdown content retain their native renderers.

The viewed-files bar uses `ProgressBar` with the same 450 ms easing and live Reduce Motion preference as the numbers. Its fill changes only during paint, continues smoothly when interrupted, and requests no frames once settled.
