use crate::change_block::ChangeBlock;
use crate::inline_changes::InlineChanges;
use crate::{DiffHunk, DiffLine, DiffLineKind, DiffOptions, LineOrigin, WhitespaceMode};

/// Applies `options` to freshly parsed hunks: folds whitespace-only changes
/// into context when ignoring whitespace (dropping hunks left without
/// changes), then marks the changed parts inside each removed/added pair.
pub(crate) fn refine_changes(hunks: Vec<DiffHunk>, options: DiffOptions) -> Vec<DiffHunk> {
    hunks
        .into_iter()
        .map(|mut hunk| {
            if options.whitespace == WhitespaceMode::Ignore {
                hunk.lines = fold_whitespace_changes(&hunk.lines);
            }
            mark_inline_changes(&mut hunk.lines, options);
            hunk
        })
        .filter(DiffHunk::has_changes)
        .collect()
}

/// Turns each removed line whose replacement differs only in whitespace
/// into one context line, keeping the other changes of its block in order:
/// removed lines before added lines between folded pairs.
fn fold_whitespace_changes(lines: &[DiffLine]) -> Vec<DiffLine> {
    let blocks = ChangeBlock::find_all(lines);
    let mut folded = Vec::with_capacity(lines.len());
    let mut next_unwritten = 0;

    for block in &blocks {
        // Context before the block stays as it is.
        folded.extend_from_slice(&lines[next_unwritten..block.span().start]);
        next_unwritten = block.span().end;

        // Walk the block pair by pair, holding back real changes until a folded pair.
        let mut pending_removed = Vec::new();
        let mut pending_added = Vec::new();
        let longest = block.removed.len().max(block.added.len());
        for offset in 0..longest {
            let removed =
                (offset < block.removed.len()).then(|| &lines[block.removed.start + offset]);
            let added = (offset < block.added.len()).then(|| &lines[block.added.start + offset]);
            match (removed, added) {
                (Some(removed), Some(added))
                    if equal_ignoring_whitespace(&removed.text, &added.text) =>
                {
                    folded.append(&mut pending_removed);
                    folded.append(&mut pending_added);
                    folded.push(DiffLine {
                        kind: DiffLineKind::Context,
                        old_number: removed.old_number,
                        new_number: added.new_number,
                        text: added.text.clone(),
                        changed_ranges: Vec::new(),
                        origin: LineOrigin::Patch,
                    });
                }
                (removed, added) => {
                    pending_removed.extend(removed.cloned());
                    pending_added.extend(added.cloned());
                }
            }
        }
        folded.append(&mut pending_removed);
        folded.append(&mut pending_added);
    }
    folded.extend_from_slice(&lines[next_unwritten..]);
    folded
}

/// Sets the changed ranges of every removed/added pair.
fn mark_inline_changes(lines: &mut [DiffLine], options: DiffOptions) {
    for block in ChangeBlock::find_all(lines) {
        for (removed, added) in block.pairs() {
            if let Some(changes) = InlineChanges::between(
                &lines[removed].text,
                &lines[added].text,
                options.highlight,
                options.whitespace,
            ) {
                lines[removed].changed_ranges = changes.old;
                lines[added].changed_ranges = changes.new;
            }
        }
    }
}

/// Whether two lines are equal once all whitespace is removed, as `git diff -w` compares.
fn equal_ignoring_whitespace(left: &str, right: &str) -> bool {
    let without_whitespace = |text: &str| {
        text.chars()
            .filter(|character| !character.is_whitespace())
            .collect::<Vec<_>>()
    };
    without_whitespace(left) == without_whitespace(right)
}
