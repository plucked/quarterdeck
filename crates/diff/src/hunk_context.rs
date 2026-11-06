use crate::{
    ContextGap, DiffHunk, DiffLine, DiffLineKind, FileDiff, GapPlace, LineOrigin, RevealedLines,
    SourceLines,
};

impl FileDiff {
    /// The unchanged lines the hunks leave out: above each hunk that does
    /// not start where the previous one (or the file) does, and below the
    /// last one unless it reaches `line_count` (the file's length, `None`
    /// while unknown). A diff without hunks has no gaps.
    pub fn gaps(&self, line_count: Option<u32>) -> Vec<ContextGap> {
        let mut gaps = Vec::new();
        let mut covered_end = 1;
        for (index, hunk) in self.hunks().iter().enumerate() {
            // Lines between the previous hunk (or the top) and this one.
            let new_lines = hunk.new_lines();
            if new_lines.start > covered_end {
                gaps.push(ContextGap {
                    place: GapPlace::AboveHunk(index),
                    new_start: covered_end,
                    new_end: Some(new_lines.start),
                    old_offset: offset_of(hunk.old_lines().start, new_lines.start),
                });
            }
            covered_end = new_lines.end;
        }

        // Lines below the last hunk, unless it reaches the end of the file.
        let Some(last) = self.hunks().last() else {
            return gaps;
        };
        let new_end = line_count.map(|count| count.saturating_add(1));
        if new_end.is_none_or(|end| end > covered_end) {
            gaps.push(ContextGap {
                place: GapPlace::End,
                new_start: covered_end,
                new_end,
                old_offset: offset_of(last.old_lines().end, last.new_lines().end),
            });
        }
        gaps
    }

    /// This diff with the `revealed` lines of `source` (the file's text
    /// after the change) shown as unchanged lines around its hunks. Hunks
    /// that the revealed lines join become one, under a header worked out
    /// again. `None` when `source` is not the file the patch changed into:
    /// a line of the patch's new side differs from it.
    pub fn with_context(&self, source: &SourceLines, revealed: &RevealedLines) -> Option<Self> {
        // The text must be what the patch leads to, or its lines would sit beside the wrong code.
        let matches_patch = self
            .hunks()
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .filter_map(|line| Some((line.new_number?, line.text.as_str())))
            .all(|(number, text)| source.line(number) == Some(text));
        if !matches_patch {
            return None;
        }

        // Walk the gaps and hunks in order; a hidden line closes the hunk being built.
        let mut builder = HunkBuilder::default();
        let gaps = self.gaps(Some(source.len()));
        for (index, hunk) in self.hunks().iter().enumerate() {
            if let Some(gap) = gaps
                .iter()
                .find(|gap| gap.place == GapPlace::AboveHunk(index))
            {
                builder.add_gap(gap, source, revealed);
            }
            builder.add_hunk(hunk);
        }
        if let Some(gap) = gaps.iter().find(|gap| gap.place == GapPlace::End) {
            builder.add_gap(gap, source, revealed);
        }
        Some(Self::from_hunks(builder.finish()))
    }
}

/// Collects lines into hunks: a run of shown lines is one hunk, and a
/// hidden line ends it.
#[derive(Default)]
struct HunkBuilder {
    finished: Vec<OpenHunk>,
    open: Option<OpenHunk>,
}

/// A hunk being built: its lines, and the section of the first patch hunk in it.
struct OpenHunk {
    section: String,
    lines: Vec<DiffLine>,
}

impl HunkBuilder {
    /// Adds the gap's revealed lines, and ends the open hunk at each hidden one.
    fn add_gap(&mut self, gap: &ContextGap, source: &SourceLines, revealed: &RevealedLines) {
        let end = gap
            .new_end
            .unwrap_or(u32::MAX)
            .min(source.len().saturating_add(1));
        for number in gap.new_start..end {
            let old_number = u32::try_from(i64::from(number) + gap.old_offset).ok();
            let shown = revealed
                .contains(number)
                .then(|| source.line(number))
                .flatten()
                .zip(old_number);
            let Some((text, old_number)) = shown else {
                self.close();
                continue;
            };
            self.open_hunk().lines.push(DiffLine {
                kind: DiffLineKind::Context,
                old_number: Some(old_number),
                new_number: Some(number),
                text: text.to_owned(),
                changed_ranges: Vec::new(),
                origin: LineOrigin::FileText,
            });
        }
    }

    /// Adds a patch hunk's lines to the open hunk (or a new one), taking its
    /// section when the open hunk has none.
    fn add_hunk(&mut self, hunk: &DiffHunk) {
        let open = self.open_hunk();
        if open.section.is_empty() {
            hunk.section().clone_into(&mut open.section);
        }
        open.lines.extend(hunk.lines.iter().cloned());
    }

    /// The open hunk, opening one when there is none.
    fn open_hunk(&mut self) -> &mut OpenHunk {
        self.open.get_or_insert_with(|| OpenHunk {
            section: String::new(),
            lines: Vec::new(),
        })
    }

    fn close(&mut self) {
        self.finished.extend(self.open.take());
    }

    /// Every hunk built, each under a header worked out from its lines.
    fn finish(mut self) -> Vec<DiffHunk> {
        self.close();
        self.finished
            .into_iter()
            .map(|open| {
                let mut hunk = DiffHunk {
                    header: String::new(),
                    old_start: 0,
                    new_start: 0,
                    lines: open.lines,
                };
                let (old, new) = (hunk.old_lines(), hunk.new_lines());
                // An empty side's start names the line before it, as git writes it.
                hunk.old_start = header_start(&old);
                hunk.new_start = header_start(&new);
                hunk.header = format!(
                    "@@ -{},{} +{},{} @@{}",
                    hunk.old_start,
                    old.len(),
                    hunk.new_start,
                    new.len(),
                    open.section
                );
                hunk
            })
            .collect()
    }
}

/// The start a header writes for a side covering `lines`.
fn header_start(lines: &std::ops::Range<u32>) -> u32 {
    if lines.is_empty() {
        lines.start.saturating_sub(1)
    } else {
        lines.start
    }
}

/// Old minus new line number.
fn offset_of(old: u32, new: u32) -> i64 {
    i64::from(old) - i64::from(new)
}
