use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Entity, Subscription, Task, UniformListScrollHandle};
use qd_cache::FilesMode;

use super::run_cancel::RunCancel;
use crate::text_area::TextArea;

/// Interactive Files controls and background content indexing, owned by one detail pane.
/// Content is shared with independent search tasks; changing scopes drops every task.
pub(super) struct FocusDrawer {
    pub mode: FilesMode,
    pub filters_open: bool,
    pub input: Entity<TextArea>,
    pub subscription: Option<Subscription>,
    pub preview: Option<super::search_preview::SearchPreview>,
    pub preview_task: Option<Task<()>>,
    pub query_task: Option<Task<()>>,
    pub retry_task: Option<Task<()>>,
    pub content_task: Option<(Task<()>, RunCancel)>,
    pub contents: HashMap<String, Arc<str>>,
    pub indexed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub content_started: bool,
    pub content_error: Option<String>,
    pub search_scroll: UniformListScrollHandle,
    pub search_files: Vec<usize>,
}

impl FocusDrawer {
    /// Starts with the persisted navigation mode and an empty, lazily loaded index.
    pub fn new(mode: FilesMode, input: Entity<TextArea>) -> Self {
        Self {
            mode,
            input,
            filters_open: false,
            subscription: None,
            preview: None,
            preview_task: None,
            query_task: None,
            retry_task: None,
            content_task: None,
            contents: HashMap::new(),
            indexed: 0,
            skipped: 0,
            failed: 0,
            content_started: false,
            content_error: None,
            search_scroll: UniformListScrollHandle::new(),
            search_files: Vec::new(),
        }
    }

    /// Drops obsolete work and text before a different revision's files are installed.
    pub fn reset_contents(&mut self) {
        self.preview_task = None;
        self.preview = None;
        self.query_task = None;
        self.content_task = None;
        self.retry_task = None;
        self.contents.clear();
        self.indexed = 0;
        self.skipped = 0;
        self.failed = 0;
        self.content_started = false;
        self.content_error = None;
        self.search_files.clear();
    }
}
