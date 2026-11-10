use crate::count_as_f32::count_as_f32;

/// A font size's average character width and the line height the diagram
/// draws it at. Diagrams are laid out before anything is drawn, so text is
/// measured by estimate; the widths are on the generous side, so text fits
/// the boxes made for it.
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    pub character_width: f32,
    pub line_height: f32,
}

/// A box's name: 14 px semibold.
pub const LABEL_TEXT: TextMetrics = TextMetrics {
    character_width: 8.,
    line_height: 18.,
};
/// A box's detail line, an arrow's label, a note: 12 px.
pub const SMALL_TEXT: TextMetrics = TextMetrics {
    character_width: 6.8,
    line_height: 16.,
};

impl TextMetrics {
    /// The width of `text` on one line.
    pub fn width_of(self, text: &str) -> f32 {
        self.width_of_count(text.chars().count())
    }

    /// The lines `text` takes when wrapped at word boundaries to `max_width`
    /// (a word longer than a line takes as many lines as it needs); at least one.
    fn line_count(self, text: &str, max_width: f32) -> usize {
        let per_line = self.characters_within(max_width);
        let mut lines = 1;
        let mut used = 0;
        for word in text.split_whitespace() {
            let length = word.chars().count();
            if used > 0 && used + 1 + length <= per_line {
                used += 1 + length;
                continue;
            }
            // The word starts a line (the first, or a new one), spilling
            // over further lines when it is longer than one.
            if used > 0 {
                lines += 1;
            }
            lines += length.saturating_sub(1) / per_line;
            used = length - length.saturating_sub(1) / per_line * per_line;
        }
        lines
    }

    /// How many whole characters fit in `width`; at least one.
    fn characters_within(self, width: f32) -> usize {
        let mut count = 1_usize;
        while self.width_of_count(count + 1) <= width {
            count += 1;
        }
        count
    }

    /// The width of `count` characters.
    fn width_of_count(self, count: usize) -> f32 {
        self.character_width * count_as_f32(count)
    }

    /// The height of `text` wrapped to `max_width`.
    pub fn height_of(self, text: &str, max_width: f32) -> f32 {
        self.line_height * count_as_f32(self.line_count(text, max_width))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words wrap whole; a word longer than a line takes extra lines.
    #[test]
    fn counts_wrapped_lines() {
        let metrics = TextMetrics {
            character_width: 10.,
            line_height: 10.,
        };
        assert_eq!(metrics.line_count("", 50.), 1);
        assert_eq!(metrics.line_count("abc de", 60.), 1);
        assert_eq!(metrics.line_count("abc def", 60.), 2);
        assert_eq!(metrics.line_count("abcdefghijkl", 50.), 3);
    }
}
