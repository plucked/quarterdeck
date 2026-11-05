use std::ops::Range;

/// A selection over a rendered document's text, in document offsets: from
/// where the drag started (`anchor`) to where the pointer is (`head`). The
/// head may lie before the anchor when dragging backwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: usize,
    pub head: usize,
}

impl TextSelection {
    /// An empty selection at `index`, as a press without a drag leaves it.
    pub fn caret(index: usize) -> Self {
        Self {
            anchor: index,
            head: index,
        }
    }

    /// A selection covering `range`, as a double or triple click makes it.
    pub fn covering(range: Range<usize>) -> Self {
        Self {
            anchor: range.start,
            head: range.end,
        }
    }

    /// The selected offsets, start before end.
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A backwards drag still yields a forward range.
    #[test]
    fn backwards_drag_has_forward_range() {
        let selection = TextSelection { anchor: 9, head: 3 };
        assert_eq!(selection.range(), 3..9);
    }

    /// A press without a drag selects nothing.
    #[test]
    fn caret_is_empty() {
        assert!(TextSelection::caret(4).is_empty());
    }
}
