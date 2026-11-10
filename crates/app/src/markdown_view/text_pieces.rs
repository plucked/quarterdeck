use std::ops::Range;

/// Cuts a text of `length` bytes at both edges of every one of `ranges`
/// (clamped to the text): the pieces between two neighbouring edges, in
/// order, never overlapping and together covering the text. Each range
/// then covers a piece wholly or not at all, so a piece has one look, as
/// gpui's text highlights need.
pub fn edge_pieces<'a>(
    length: usize,
    ranges: impl IntoIterator<Item = &'a Range<usize>>,
) -> Vec<Range<usize>> {
    // Every place where the look may change, once each, in order.
    let mut edges = vec![0, length];
    for range in ranges {
        edges.extend([range.start.min(length), range.end.min(length)]);
    }
    edges.sort_unstable();
    edges.dedup();

    // The pieces between neighbouring edges.
    edges.windows(2).map(|pair| pair[0]..pair[1]).collect()
}

/// Whether `range` covers all of `piece`.
pub fn covers(range: &Range<usize>, piece: &Range<usize>) -> bool {
    range.start <= piece.start && piece.end <= range.end
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Overlapping ranges cut the text at each of their edges, clamped to it.
    #[test]
    fn cuts_at_every_edge() {
        let ranges = [2..5, 4..99];
        assert_eq!(edge_pieces(8, &ranges), [0..2, 2..4, 4..5, 5..8]);
    }

    /// Without ranges the whole text is one piece.
    #[test]
    fn a_text_without_ranges_is_one_piece() {
        assert_eq!(edge_pieces(3, &[]), [Range { start: 0, end: 3 }]);
    }
}
