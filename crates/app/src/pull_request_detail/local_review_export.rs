use std::fmt::Write as _;

use qc_cache::{NoteAnchor, ReviewNote};
use qc_diff::DiffLineKind;
use qc_github::LineSide;

use super::loaded_file::LoadedFile;

/// Lines of code shown above a note's first line.
const CONTEXT_LINES: usize = 2;

/// What the export says the review is of.
pub struct ReviewExportHeader {
    /// `owner/repo`.
    pub repository: String,
    /// `Pull request #12: Title`, or `Stack #9, layers 1–3 (#10 to #12)`.
    pub subject: String,
    /// `feature → main`.
    pub branches: String,
}

/// The local notes as Markdown for an AI agent: what is reviewed, then per
/// file (in diff order) each note with its lines, the code they show and
/// the note itself.
pub fn local_review_markdown(
    header: &ReviewExportHeader,
    notes: &[&ReviewNote],
    files: &[LoadedFile],
) -> String {
    let mut markdown = String::new();
    let _ = writeln!(markdown, "# Review notes: {}", header.subject);
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "Repository: `{}` · {}",
        header.repository, header.branches
    );
    let _ = writeln!(markdown);
    let _ = writeln!(
        markdown,
        "Each note names a file and the lines it is about (line numbers of the new version unless marked old), shows that code from the diff, and says what to change. Please address every note."
    );

    // Notes by file in diff order, then by line.
    let file_index = |path: &str| {
        files
            .iter()
            .position(|loaded| loaded.filename() == path)
            .unwrap_or(usize::MAX)
    };
    let mut sorted: Vec<&ReviewNote> = notes.to_vec();
    sorted.sort_by_key(|note| {
        (
            file_index(&note.path),
            note.path.clone(),
            note_line(note),
            note.created_at,
        )
    });

    let mut current_path: Option<&str> = None;
    for note in sorted {
        if current_path != Some(note.path.as_str()) {
            let _ = writeln!(markdown);
            let _ = writeln!(markdown, "## `{}`", note.path);
            current_path = Some(&note.path);
        }
        let _ = writeln!(markdown);
        let _ = writeln!(markdown, "### {}", lines_title(&note.anchor));
        if let (
            Some(loaded),
            NoteAnchor::Line {
                side,
                line,
                start_line,
            },
        ) = (
            files.iter().find(|loaded| loaded.filename() == note.path),
            &note.anchor,
        ) && let Some(excerpt) = code_excerpt(loaded, *side, start_line.unwrap_or(*line), *line)
        {
            let _ = writeln!(markdown);
            let _ = writeln!(markdown, "```diff");
            markdown.push_str(&excerpt);
            let _ = writeln!(markdown, "```");
        }
        let _ = writeln!(markdown);
        let _ = writeln!(markdown, "{}", note.body.trim());
    }
    markdown
}

/// The line a note sorts by.
fn note_line(note: &ReviewNote) -> u32 {
    match &note.anchor {
        NoteAnchor::Line {
            line, start_line, ..
        } => start_line.unwrap_or(*line),
        NoteAnchor::Reply { .. } => 0,
    }
}

/// `Line 42`, `Lines 40–42`, with ` (old)` for the old side.
fn lines_title(anchor: &NoteAnchor) -> String {
    let NoteAnchor::Line {
        side,
        line,
        start_line,
    } = anchor
    else {
        return "Reply".to_owned();
    };
    let side = match side {
        LineSide::New => "",
        LineSide::Old => " (old)",
    };
    match start_line {
        Some(start) if start != line => format!("Lines {start}–{line}{side}"),
        Some(_) | None => format!("Line {line}{side}"),
    }
}

/// The diff lines from `CONTEXT_LINES` above `first` to `last` on `side`,
/// as `+`/`-`/` ` prefixed lines; `None` when the diff does not show them.
fn code_excerpt(loaded: &LoadedFile, side: LineSide, first: u32, last: u32) -> Option<String> {
    for hunk in loaded.diff.hunks() {
        let number_of = |index: usize| {
            let line = &hunk.lines[index];
            match side {
                LineSide::New => line.new_number,
                LineSide::Old => line.old_number,
            }
        };
        let in_range: Vec<usize> = (0..hunk.lines.len())
            .filter(|index| {
                number_of(*index).is_some_and(|number| (first..=last).contains(&number))
            })
            .collect();
        let (Some(start), Some(end)) = (in_range.first(), in_range.last()) else {
            continue;
        };
        let mut excerpt = String::new();
        for line in &hunk.lines[start.saturating_sub(CONTEXT_LINES)..=*end] {
            let marker = match line.kind {
                DiffLineKind::Added => '+',
                DiffLineKind::Removed => '-',
                DiffLineKind::Context => ' ',
            };
            let _ = writeln!(excerpt, "{marker}{}", line.text);
        }
        return Some(excerpt);
    }
    None
}
