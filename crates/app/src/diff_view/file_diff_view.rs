use std::rc::Rc;
use std::sync::Arc;

use gpui::{AnyElement, App, ClickEvent, IntoElement, RenderOnce, Window, div, prelude::*, px};
use qc_cache::{DiffLayout, DiffViewSettings};
use qc_diff::{FileDiff, WhitespaceMode};
use qc_github::PullRequestFile;

use super::FileDiffState;
use super::diff_cell_rendering::DIFF_ROW_HEIGHT;
use super::file_header_rendering::{FileClickHandler, render_file_header};
use super::split_rendering::render_split;
use super::unified_rendering::render_unified;
use crate::theme;

/// One changed file: its header and, unless collapsed, its diff in the
/// layout and wrapping of `settings`.
///
/// Elements must own what they draw, so the file and its built diff come in
/// `Arc`s the owning view keeps. `index` is the file's position in the
/// list; it keeps the ids of each file's elements apart.
#[derive(IntoElement)]
pub struct FileDiffView {
    index: usize,
    file: Arc<PullRequestFile>,
    diff: Arc<FileDiff>,
    settings: DiffViewSettings,
    state: FileDiffState,
    on_toggle_collapsed: FileClickHandler,
    on_toggle_viewed: FileClickHandler,
}

impl FileDiffView {
    pub fn new(
        index: usize,
        file: Arc<PullRequestFile>,
        diff: Arc<FileDiff>,
        settings: DiffViewSettings,
        state: FileDiffState,
    ) -> Self {
        let ignore_click: FileClickHandler = Rc::new(|_, _, _| {});
        Self {
            index,
            file,
            diff,
            settings,
            state,
            on_toggle_collapsed: ignore_click.clone(),
            on_toggle_viewed: ignore_click,
        }
    }

    /// Called when the header (or a large file's expand note) is clicked.
    pub fn on_toggle_collapsed(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_collapsed = Rc::new(handler);
        self
    }

    /// Called when the viewed checkbox is clicked.
    pub fn on_toggle_viewed(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_viewed = Rc::new(handler);
        self
    }
}

impl RenderOnce for FileDiffView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let header = render_file_header(
            self.index,
            &self.file,
            self.state,
            self.on_toggle_collapsed.clone(),
            self.on_toggle_viewed.clone(),
        );
        let body = match FileBody::for_state(self.state) {
            FileBody::Diff => Some(self.render_body()),
            FileBody::ExpandNote => Some(render_large_note(
                self.index,
                self.diff.changed_line_count(),
                self.on_toggle_collapsed,
            )),
            FileBody::Hidden => None,
        };

        div()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(theme::border())
            .overflow_hidden()
            .child(header.border_b_1().border_color(theme::border()))
            .children(body)
    }
}

impl FileDiffView {
    /// The diff, or a note when there is nothing to draw.
    fn render_body(&self) -> AnyElement {
        if self.file.patch.is_none() {
            return render_note("Binary file, or a diff too large for GitHub to send.");
        }
        if self.diff.is_empty() {
            return render_note(empty_diff_message(&self.file, self.settings));
        }
        let body = match self.settings.layout {
            DiffLayout::Unified => render_unified(&self.diff, self.settings.wrapping, self.index),
            DiffLayout::Split => render_split(&self.diff, self.settings.wrapping, self.index),
        };
        div()
            .font_family(theme::MONO_FONT)
            .text_xs()
            .line_height(px(DIFF_ROW_HEIGHT))
            .child(body)
            .into_any_element()
    }
}

/// What shows below a file's header.
enum FileBody {
    Diff,
    /// A large file that started collapsed offers to expand.
    ExpandNote,
    Hidden,
}

impl FileBody {
    fn for_state(state: FileDiffState) -> Self {
        if !state.is_collapsed {
            Self::Diff
        } else if state.is_large && !state.is_viewed {
            Self::ExpandNote
        } else {
            Self::Hidden
        }
    }
}

/// Why a file with a patch shows no lines.
fn empty_diff_message(file: &PullRequestFile, settings: DiffViewSettings) -> &'static str {
    match (settings.whitespace, file.status.as_str()) {
        (WhitespaceMode::Ignore, _) => "Only whitespace changes.",
        (WhitespaceMode::Show, "renamed") => "File renamed without changes.",
        (WhitespaceMode::Show, _) => "No changes to show.",
    }
}

/// A muted one-line note in place of a body.
fn render_note(message: &'static str) -> AnyElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .text_color(theme::muted())
        .child(message)
        .into_any_element()
}

/// The note under a large file that starts collapsed; clicking it expands the file.
fn render_large_note(index: usize, line_count: usize, on_expand: FileClickHandler) -> AnyElement {
    div()
        .id(("expand-large", index))
        .px_3()
        .py_2()
        .text_xs()
        .text_color(theme::accent())
        .cursor_pointer()
        .on_click(move |event, window, cx| on_expand(event, window, cx))
        .child(format!(
            "Large diff ({line_count} changed lines) — click to show"
        ))
        .into_any_element()
}
