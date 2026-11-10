use std::collections::HashMap;
use std::ops::Range;

use gpui::Task;
use qd_diff::RevealedLines;

use crate::diff_view::FileTextLoad;

/// The file texts being fetched to reveal lines around hunks, by file
/// index, with the lines asked for meanwhile and the tasks fetching them
/// and coloring the grown diffs. Dropping a task cancels it, so a new pull
/// request starts clean.
#[derive(Default)]
pub struct ContextLoads {
    loads: HashMap<usize, FileTextLoad>,
    /// Lines clicked open while the file's text is on its way.
    waiting: HashMap<usize, RevealedLines>,
    fetch_tasks: HashMap<usize, Task<()>>,
    syntax_tasks: HashMap<usize, Task<()>>,
}

impl ContextLoads {
    /// How fetching file `file`'s text stands; `None` when idle or done.
    pub fn load_of(&self, file: usize) -> Option<&FileTextLoad> {
        self.loads.get(&file)
    }

    /// Records that file `file`'s text is being fetched by `task`, to reveal `lines`.
    pub fn start(&mut self, file: usize, lines: Range<u32>, task: Task<()>) {
        self.loads.insert(file, FileTextLoad::Loading);
        self.waiting.insert(file, RevealedLines::default());
        self.reveal_when_loaded(file, lines);
        self.fetch_tasks.insert(file, task);
    }

    /// Adds `lines` to what file `file` reveals once its text arrives.
    pub fn reveal_when_loaded(&mut self, file: usize, lines: Range<u32>) {
        self.waiting.entry(file).or_default().reveal(lines);
    }

    /// Records that file `file`'s text arrived; the lines asked for meanwhile.
    pub fn finish(&mut self, file: usize) -> RevealedLines {
        self.loads.remove(&file);
        self.waiting.remove(&file).unwrap_or_default()
    }

    /// Records why file `file`'s text cannot be shown; the lines asked for are dropped.
    pub fn fail(&mut self, file: usize, message: impl Into<gpui::SharedString>) {
        self.waiting.remove(&file);
        self.loads
            .insert(file, FileTextLoad::Failed(message.into()));
    }

    /// Keeps the task coloring file `file`'s grown diff, replacing an older one.
    pub fn color_with(&mut self, file: usize, task: Task<()>) {
        self.syntax_tasks.insert(file, task);
    }
}
