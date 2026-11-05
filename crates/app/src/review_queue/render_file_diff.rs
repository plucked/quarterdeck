use gpui::{Div, div, prelude::*};
use qc_github::PullRequestFile;

use crate::theme;

/// One changed file: a header with name and line counts, then its patch with
/// added and removed lines tinted. Binary or oversized files show no patch.
pub fn render_file_diff(file: &PullRequestFile) -> impl IntoElement {
    let header = div()
        .flex()
        .gap_2()
        .px_2()
        .py_1()
        .bg(theme::panel())
        .border_b_1()
        .border_color(theme::border())
        .child(div().flex_1().truncate().child(file.filename.clone()))
        .child(
            div()
                .text_color(theme::added())
                .child(format!("+{}", file.additions)),
        )
        .child(
            div()
                .text_color(theme::removed())
                .child(format!("−{}", file.deletions)),
        );

    let patch = file.patch.as_deref().map(|patch| {
        div()
            .flex()
            .flex_col()
            .font_family(theme::MONO_FONT)
            .text_xs()
            .children(patch.lines().map(render_patch_line))
    });

    div()
        .flex()
        .flex_col()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .overflow_hidden()
        .child(header)
        .children(patch)
}

/// One line of a unified diff, styled by its leading marker.
fn render_patch_line(line: &str) -> Div {
    let row = div().px_2().whitespace_nowrap().child(line.to_owned());
    match line.as_bytes().first() {
        Some(b'+') => row.bg(theme::added_background()),
        Some(b'-') => row.bg(theme::removed_background()),
        Some(b'@') => row.text_color(theme::muted()),
        _ => row,
    }
}
