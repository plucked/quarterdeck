use qd_cache::NoteAnchor;
use qd_github::LineSide;

/// `Line 42`, `Lines 40–42`, with ` (old)` on the old side.
pub fn lines_label(side: LineSide, line: u32, start_line: Option<u32>) -> String {
    let side = match side {
        LineSide::New => "",
        LineSide::Old => " (old)",
    };
    match start_line {
        Some(start) if start != line => format!("Lines {start}–{line}{side}"),
        Some(_) | None => format!("Line {line}{side}"),
    }
}

/// The lines a note is about ([`lines_label`]), or `Reply` for a reply.
pub fn note_lines_label(anchor: &NoteAnchor) -> String {
    match anchor {
        NoteAnchor::Line {
            side,
            line,
            start_line,
        } => lines_label(*side, *line, *start_line),
        NoteAnchor::Reply { .. } => "Reply".to_owned(),
    }
}
