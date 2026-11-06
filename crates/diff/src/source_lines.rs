/// A file's text after the change, split into lines the way patch lines
/// are shown (tabs as four spaces, no line endings), numbered from 1. The
/// lines around a diff's hunks come from here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceLines {
    lines: Vec<String>,
}

/// What a tab shows as, as in the patch's lines.
const TAB_AS_SPACES: &str = "    ";

impl SourceLines {
    /// Splits `text` into lines; a final line ending adds no empty line.
    pub fn parse(text: &str) -> Self {
        let lines = text
            .lines()
            .map(|line| {
                let line = line.strip_suffix('\r').unwrap_or(line);
                line.replace('\t', TAB_AS_SPACES)
            })
            .collect();
        Self { lines }
    }

    /// How many lines the file has.
    pub fn len(&self) -> u32 {
        u32::try_from(self.lines.len()).unwrap_or(u32::MAX)
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Line `number` (from 1); `None` past the end.
    pub fn line(&self, number: u32) -> Option<&str> {
        let index = usize::try_from(number.checked_sub(1)?).ok()?;
        self.lines.get(index).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines are numbered from 1, without endings, tabs expanded.
    #[test]
    fn numbers_lines_from_one() {
        let source = SourceLines::parse("a\r\n\tb\nc\n");
        assert_eq!(source.len(), 3);
        assert_eq!(source.line(1), Some("a"));
        assert_eq!(source.line(2), Some("    b"));
        assert_eq!(source.line(0), None);
        assert_eq!(source.line(4), None);
    }
}
