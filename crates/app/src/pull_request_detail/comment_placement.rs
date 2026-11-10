use std::collections::HashMap;

use super::diff_line_ref::DiffLineRef;
use super::review_comments::PlacedCard;

/// Where the comment cards and the open composer go in the diff: by file
/// index and line.
#[derive(Debug, Default)]
pub struct CommentPlacement {
    /// Cards under a line: the line → card indexes.
    on_lines: HashMap<DiffLineRef, Vec<usize>>,
    /// Every card of a file, in card order.
    of_files: HashMap<usize, Vec<usize>>,
    /// The line a new comment is being written under.
    composer: Option<DiffLineRef>,
}

impl CommentPlacement {
    /// Places `cards` in the files `file_of_path` finds them in; cards of
    /// files not loaded are left out.
    pub fn build(
        cards: &[PlacedCard],
        file_of_path: impl Fn(&str) -> Option<usize>,
        composer: Option<DiffLineRef>,
    ) -> Self {
        let mut placement = Self {
            composer,
            ..Self::default()
        };
        for (index, card) in cards.iter().enumerate() {
            let Some(file) = file_of_path(&card.path) else {
                continue;
            };
            placement.of_files.entry(file).or_default().push(index);
            if let Some((side, line)) = card.anchor {
                placement
                    .on_lines
                    .entry(DiffLineRef { file, side, line })
                    .or_default()
                    .push(index);
            }
        }
        placement
    }

    /// The cards under `line`.
    pub fn cards_on(&self, line: DiffLineRef) -> &[usize] {
        self.on_lines.get(&line).map_or(&[], Vec::as_slice)
    }

    /// Every card of file `file`.
    pub fn cards_of(&self, file: usize) -> &[usize] {
        self.of_files.get(&file).map_or(&[], Vec::as_slice)
    }

    /// Whether the composer goes under `line`.
    pub fn has_composer_on(&self, line: DiffLineRef) -> bool {
        self.composer == Some(line)
    }
}
