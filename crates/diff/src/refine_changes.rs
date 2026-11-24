use std::time::Duration;

use similar::{Algorithm, DiffOp, TextDiffConfig};

use crate::change_block::ChangeBlock;
use crate::inline_changes::InlineChanges;
use crate::{DiffHunk, DiffLine, DiffLineKind, DiffOptions, LineOrigin, WhitespaceMode};

/// The longest a hunk re-diff may take before falling back to an approximate match.
const HUNK_DIFF_TIMEOUT: Duration = Duration::from_millis(50);

/// One side's line from the parsed hunk, paired with its position there so
/// unchanged original context lines keep [`LineOrigin::Patch`].
struct HunkSideLine<'a> {
    original_index: usize,
    line: &'a DiffLine,
}

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

/// Re-diffs one hunk's old and new sides ignoring all whitespace (as
/// `git diff -w` compares), turning matched lines into context while keeping
/// genuine original context as [`LineOrigin::Patch`] and re-paired lines as
/// [`LineOrigin::FoldedReplacement`].
fn fold_whitespace_changes(lines: &[DiffLine]) -> Vec<DiffLine> {
    // Only hunks with both removals and additions can fold whitespace-only changes.
    let has_removed = lines.iter().any(|line| line.kind == DiffLineKind::Removed);
    let has_added = lines.iter().any(|line| line.kind == DiffLineKind::Added);
    if !has_removed || !has_added {
        return lines.to_vec();
    }

    // Separate the hunk into its full old and new line sequences and strip whitespace for matching.
    let old_lines: Vec<HunkSideLine<'_>> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.kind != DiffLineKind::Added)
        .map(|(original_index, line)| HunkSideLine {
            original_index,
            line,
        })
        .collect();
    let new_lines: Vec<HunkSideLine<'_>> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.kind != DiffLineKind::Removed)
        .map(|(original_index, line)| HunkSideLine {
            original_index,
            line,
        })
        .collect();
    let old_keys: Vec<String> = old_lines
        .iter()
        .map(|entry| strip_whitespace(&entry.line.text))
        .collect();
    let new_keys: Vec<String> = new_lines
        .iter()
        .map(|entry| strip_whitespace(&entry.line.text))
        .collect();
    let old_key_slices: Vec<&str> = old_keys.iter().map(String::as_str).collect();
    let new_key_slices: Vec<&str> = new_keys.iter().map(String::as_str).collect();

    // Re-diff with Patience so unique code lines anchor re-indented blocks instead of braces or blanks.
    let mut config = TextDiffConfig::default();
    config
        .algorithm(Algorithm::Patience)
        .timeout(HUNK_DIFF_TIMEOUT);
    let diff = config.diff_slices(&old_key_slices, &new_key_slices);
    let operations = slide_changes_down(diff.ops(), &old_key_slices, &new_key_slices);

    // Emit matched lines as context and flush removals before additions between them.
    let mut folded = Vec::with_capacity(lines.len());
    let mut pending_removed = Vec::new();
    let mut pending_added = Vec::new();
    for operation in operations {
        match operation {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                if len == 0 {
                    continue;
                }
                folded.append(&mut pending_removed);
                folded.append(&mut pending_added);
                for offset in 0..len {
                    folded.push(folded_context_line(
                        &old_lines[old_index + offset],
                        &new_lines[new_index + offset],
                    ));
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for entry in &old_lines[old_index..old_index + old_len] {
                    pending_removed.push(removed_line(entry.line));
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for entry in &new_lines[new_index..new_index + new_len] {
                    pending_added.push(added_line(entry.line));
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                for entry in &old_lines[old_index..old_index + old_len] {
                    pending_removed.push(removed_line(entry.line));
                }
                for entry in &new_lines[new_index..new_index + new_len] {
                    pending_added.push(added_line(entry.line));
                }
            }
        }
    }
    folded.append(&mut pending_removed);
    folded.append(&mut pending_added);
    folded
}

/// Shifts pure insertions and deletions downward across identical following lines
/// (ignoring whitespace), so wrapping or unwrapping a block marks its outer closing
/// delimiter instead of an inner one.
fn slide_changes_down(operations: &[DiffOp], old_keys: &[&str], new_keys: &[&str]) -> Vec<DiffOp> {
    let mut remaining = operations.to_vec();
    let mut shifted = Vec::with_capacity(remaining.len());

    for index in 0..remaining.len() {
        let current = remaining[index];
        let Some(DiffOp::Equal {
            old_index: equal_old,
            new_index: equal_new,
            len: equal_len,
        }) = remaining.get(index + 1).copied()
        else {
            shifted.push(current);
            continue;
        };

        // Slide an insertion or deletion across as many matching equal lines as follow it.
        match current {
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => {
                let mut shift = 0;
                while shift < equal_len
                    && new_keys[new_index + shift] == new_keys[equal_new + shift]
                {
                    shift += 1;
                }
                if shift > 0 {
                    shifted.push(DiffOp::Equal {
                        old_index: equal_old,
                        new_index,
                        len: shift,
                    });
                    shifted.push(DiffOp::Insert {
                        old_index: old_index + shift,
                        new_index: new_index + shift,
                        new_len,
                    });
                    remaining[index + 1] = DiffOp::Equal {
                        old_index: equal_old + shift,
                        new_index: equal_new + shift,
                        len: equal_len - shift,
                    };
                } else {
                    shifted.push(current);
                }
            }
            DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => {
                let mut shift = 0;
                while shift < equal_len
                    && old_keys[old_index + shift] == old_keys[equal_old + shift]
                {
                    shift += 1;
                }
                if shift > 0 {
                    shifted.push(DiffOp::Equal {
                        old_index,
                        new_index: equal_new,
                        len: shift,
                    });
                    shifted.push(DiffOp::Delete {
                        old_index: old_index + shift,
                        old_len,
                        new_index: new_index + shift,
                    });
                    remaining[index + 1] = DiffOp::Equal {
                        old_index: equal_old + shift,
                        new_index: equal_new + shift,
                        len: equal_len - shift,
                    };
                } else {
                    shifted.push(current);
                }
            }
            DiffOp::Equal { .. } | DiffOp::Replace { .. } => shifted.push(current),
        }
    }
    shifted
}

/// A matched old/new pair shown as context: original context lines stay
/// [`LineOrigin::Patch`], while re-paired lines record [`LineOrigin::FoldedReplacement`].
fn folded_context_line(old_entry: &HunkSideLine<'_>, new_entry: &HunkSideLine<'_>) -> DiffLine {
    let origin = if old_entry.original_index == new_entry.original_index {
        old_entry.line.origin
    } else {
        LineOrigin::FoldedReplacement
    };
    DiffLine {
        kind: DiffLineKind::Context,
        old_number: old_entry.line.old_number,
        new_number: new_entry.line.new_number,
        text: new_entry.line.text.clone(),
        changed_ranges: Vec::new(),
        origin,
    }
}

/// An unmatched old-side line shown as removed.
fn removed_line(line: &DiffLine) -> DiffLine {
    DiffLine {
        kind: DiffLineKind::Removed,
        old_number: line.old_number,
        new_number: None,
        text: line.text.clone(),
        changed_ranges: Vec::new(),
        origin: LineOrigin::Patch,
    }
}

/// An unmatched new-side line shown as added.
fn added_line(line: &DiffLine) -> DiffLine {
    DiffLine {
        kind: DiffLineKind::Added,
        old_number: None,
        new_number: line.new_number,
        text: line.text.clone(),
        changed_ranges: Vec::new(),
        origin: LineOrigin::Patch,
    }
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

/// Removes all whitespace so lines can be compared with `git diff -w` semantics.
fn strip_whitespace(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
