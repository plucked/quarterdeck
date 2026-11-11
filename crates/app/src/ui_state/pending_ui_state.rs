use std::collections::HashMap;

use qd_cache::{
    CacheTtl, ColorTheme, DiffViewSettings, FoldedGroups, HiddenBranchRule, OthersFilter,
    ReviewLayoutState, ReviewQueueViewState, ReviewScope, ViewedFile, WindowPlacement,
};
use qd_github::RepoRef;

use super::SharedUiStateStore;
use super::review_change::ReviewChange;

/// Changes recorded since the last write; only the latest value of each part
/// is kept.
#[derive(Default)]
pub struct PendingUiState {
    pub window_placement: Option<WindowPlacement>,
    pub review_queue: Option<ReviewQueueViewState>,
    pub diff_view_settings: Option<DiffViewSettings>,
    pub cache_ttl: Option<CacheTtl>,
    pub color_theme: Option<ColorTheme>,
    pub folder_heatmap: Option<bool>,
    pub review_layout: Option<ReviewLayoutState>,
    pub sidebar_width: Option<f32>,
    pub side_panel_width: Option<f32>,
    pub folded_groups: Option<FoldedGroups>,
    pub watched_repositories: Option<Vec<RepoRef>>,
    pub hidden_branch_rules: Option<Vec<HiddenBranchRule>>,
    /// The latest Others filter of each repository changed.
    pub others_filters: HashMap<RepoRef, OthersFilter>,
    /// Every viewed mark change, in the order made: unlike settings, each counts.
    pub viewed_changes: Vec<ViewedChange>,
    /// Every review note and summary change, in the order made.
    pub review_changes: Vec<ReviewChange>,
}

/// A change to a file's viewed mark, waiting to be written.
pub enum ViewedChange {
    Mark {
        scope: ReviewScope,
        file: ViewedFile,
    },
    Unmark {
        scope: ReviewScope,
        filename: String,
    },
}

impl PendingUiState {
    /// Whether nothing was recorded since the last write.
    pub fn is_empty(&self) -> bool {
        self.window_placement.is_none()
            && self.review_queue.is_none()
            && self.diff_view_settings.is_none()
            && self.cache_ttl.is_none()
            && self.color_theme.is_none()
            && self.folder_heatmap.is_none()
            && self.review_layout.is_none()
            && self.sidebar_width.is_none()
            && self.side_panel_width.is_none()
            && self.folded_groups.is_none()
            && self.watched_repositories.is_none()
            && self.hidden_branch_rules.is_none()
            && self.others_filters.is_empty()
            && self.viewed_changes.is_empty()
            && self.review_changes.is_empty()
    }

    /// Writes each recorded part to `store`, reporting failures on stderr;
    /// a part that fails does not stop the rest.
    ///
    /// Blocking: runs on the background executor.
    pub fn write_to(&self, store: &SharedUiStateStore) {
        let store = store.lock();

        // The one-value parts, each written when recorded.
        if let Some(placement) = &self.window_placement {
            report_failed_write("window placement", store.put_window_placement(placement));
        }
        if let Some(state) = &self.review_queue {
            report_failed_write(
                "review queue state",
                store.put_review_queue_view_state(state),
            );
        }
        if let Some(settings) = &self.diff_view_settings {
            report_failed_write("diff view settings", store.put_diff_view_settings(settings));
        }
        if let Some(layout) = &self.review_layout {
            report_failed_write("the review layout", store.put_review_layout(layout));
        }
        if let Some(width) = self.sidebar_width {
            report_failed_write("the sidebar width", store.put_sidebar_width(width));
        }
        if let Some(width) = self.side_panel_width {
            report_failed_write("the side panel width", store.put_side_panel_width(width));
        }
        if let Some(groups) = &self.folded_groups {
            report_failed_write("the folded groups", store.put_folded_groups(groups));
        }
        if let Some(repositories) = &self.watched_repositories {
            report_failed_write(
                "the watched repositories",
                store.put_watched_repositories(repositories),
            );
        }
        if let Some(rules) = &self.hidden_branch_rules {
            report_failed_write(
                "the hidden branch rules",
                store.put_hidden_branch_rules(rules),
            );
        }
        for (repo, filter) in &self.others_filters {
            report_failed_write("an Others filter", store.put_others_filter(repo, filter));
        }
        if let Some(ttl) = self.cache_ttl {
            report_failed_write("the cache TTL", store.put_cache_ttl(ttl));
        }
        if let Some(theme) = self.color_theme {
            report_failed_write("the color theme", store.put_color_theme(theme));
        }
        if let Some(enabled) = self.folder_heatmap {
            report_failed_write("the folder heatmap", store.put_folder_heatmap(enabled));
        }

        // Viewed marks and review notes, every change in the order made.
        for change in &self.viewed_changes {
            let written = match change {
                ViewedChange::Mark { scope, file } => store.mark_file_viewed(scope, file),
                ViewedChange::Unmark { scope, filename } => {
                    store.unmark_file_viewed(scope, filename)
                }
            };
            report_failed_write("a viewed mark", written);
        }
        for change in &self.review_changes {
            let written = match change {
                ReviewChange::PutNote(note) => store.put_review_note(note),
                ReviewChange::DeleteNote(id) => store.delete_review_note(*id),
                ReviewChange::PutSummary(pull_request_id, summary) => {
                    store.put_review_summary(*pull_request_id, summary)
                }
                ReviewChange::DeleteSummary(pull_request_id) => {
                    store.delete_review_summary(*pull_request_id)
                }
            };
            report_failed_write("a review note", written);
        }
    }
}

/// Reports on stderr that `part` could not be saved, when `result` failed;
/// losing it only costs what the next launch restores.
fn report_failed_write(part: &str, result: anyhow::Result<()>) {
    if let Err(error) = result {
        eprintln!("ui state: cannot save {part}: {error:#}");
    }
}
