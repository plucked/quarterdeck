use std::collections::HashMap;

use gpui::Task;

use crate::diff_view::FileTextLoad;

/// The file texts being fetched to reveal lines around hunks, by file
/// index, with the tasks fetching them and coloring the grown diffs.
/// Dropping a task cancels it, so a new pull request starts clean.
#[derive(Default)]
pub struct ContextLoads {
    loads: HashMap<usize, FileTextLoad>,
    fetch_tasks: HashMap<usize, Task<()>>,
    syntax_tasks: HashMap<usize, Task<()>>,
}

impl ContextLoads {
    /// How fetching file `file`'s text stands; `None` when idle or done.
    pub fn load_of(&self, file: usize) -> Option<&FileTextLoad> {
        self.loads.get(&file)
    }

    /// Records that file `file`'s text is being fetched by `task`.
    pub fn start(&mut self, file: usize, task: Task<()>) {
        self.loads.insert(file, FileTextLoad::Loading);
        self.fetch_tasks.insert(file, task);
    }

    /// Records that file `file`'s text arrived and is used.
    pub fn finish(&mut self, file: usize) {
        self.loads.remove(&file);
    }

    /// Records why file `file`'s text cannot be shown.
    pub fn fail(&mut self, file: usize, message: impl Into<gpui::SharedString>) {
        self.loads
            .insert(file, FileTextLoad::Failed(message.into()));
    }

    /// Keeps the task coloring file `file`'s grown diff, replacing an older one.
    pub fn color_with(&mut self, file: usize, task: Task<()>) {
        self.syntax_tasks.insert(file, task);
    }
}
