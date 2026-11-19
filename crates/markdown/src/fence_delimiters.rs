use crate::MarkdownFence;

/// Source enclosing a replacement payload, including its language, indentation and line endings.
pub struct FenceDelimiters {
    pub opening: String,
    pub closing: String,
}

impl FenceDelimiters {
    /// Retains the existing delimiters, widening them together when new code contains their marker.
    pub fn for_code(fence: &MarkdownFence, source: &str, code: &str) -> Self {
        let marker = char::from(source.as_bytes()[fence.opening.start]);
        let longest = code
            .split(|character| character != marker)
            .map(str::len)
            .max()
            .unwrap_or(0);
        let width = fence.opening.len().max(longest + 1);

        // Keep compact or unfinished blocks editable, including their first line ending.
        let header = &source[fence.opening.end..fence.code.start];
        let line_ending = if header.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut opening = source[fence.outer.start..fence.opening.end].to_owned();
        opening.extend(std::iter::repeat_n(marker, width - fence.opening.len()));
        opening.push_str(header);
        if !code.is_empty() && !header.contains('\n') {
            opening.push_str(line_ending);
        }

        // An unfinished fence stays open. A closing marker always starts on its own line.
        let mut closing = String::new();
        if let Some(delimiter) = &fence.closing {
            let separator = &source[fence.code.end..delimiter.start];
            if !code.is_empty() && !separator.contains('\n') {
                closing.push_str(line_ending);
            }
            closing.push_str(&source[fence.code.end..delimiter.end]);
            closing.extend(std::iter::repeat_n(
                marker,
                width.saturating_sub(delimiter.len()),
            ));
            closing.push_str(&source[delimiter.end..fence.outer.end]);
        } else {
            closing.push_str(&source[fence.code.end..fence.outer.end]);
        }
        Self { opening, closing }
    }
}
