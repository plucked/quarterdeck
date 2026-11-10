use std::fmt::Write as _;

use qd_cache::{NoteAnchor, ReviewNote};
use qd_diff::DiffLineKind;
use qd_github::{LineSide, PullRequest};

use super::lines_label::note_lines_label;
use super::loaded_file::LoadedFile;
use super::review_target::ReviewTarget;
use crate::diff_line_sides::line_number_on;
use crate::display_text::short_sha;

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

impl ReviewExportHeader {
    /// What the export of `pull_request`'s notes at `target` is of: a
    /// combined range's stack, layers and commits, else the pull request
    /// with its link.
    pub fn for_target(pull_request: &PullRequest, target: &ReviewTarget) -> Self {
        match target {
            ReviewTarget::Combined(range) => {
                let stack_number = pull_request
                    .stack
                    .as_ref()
                    .map_or(0, |membership| membership.number);
                Self {
                    repository: range.repo.full_name(),
                    subject: format!(
                        "stack #{stack_number}, layers {}–{} combined (#{} to #{})",
                        range.first_index + 1,
                        range.last_index + 1,
                        range.first_number,
                        range.last_number
                    ),
                    branches: format!(
                        "base {} → head {}",
                        short_sha(&range.base_sha),
                        short_sha(&range.head_sha)
                    ),
                }
            }
            ReviewTarget::Layer | ReviewTarget::AwaitingStack => Self {
                repository: pull_request.repo.full_name(),
                subject: format!(
                    "pull request #{}: {}",
                    pull_request.number, pull_request.title
                ),
                branches: pull_request.html_url.clone(),
            },
        }
    }
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
        let _ = writeln!(markdown, "### {}", note_lines_label(&note.anchor));
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

/// The diff lines from `CONTEXT_LINES` above `first` to `last` on `side`,
/// as `+`/`-`/` ` prefixed lines; `None` when the diff does not show them.
fn code_excerpt(loaded: &LoadedFile, side: LineSide, first: u32, last: u32) -> Option<String> {
    for hunk in loaded.diff.hunks() {
        let number_of = |index: usize| line_number_on(&hunk.lines[index], side);
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
