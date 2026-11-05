use std::collections::{BTreeSet, HashSet};

use gpui::UniformListScrollHandle;
use qc_cache::{PanelVisibility, ReviewLayoutState, StackReviewMode};
use qc_file_tree::{ExtensionCount, FileFilter, FileTree, NodeId, TreeRow};

use super::file_review_state::FileReviewState;
use super::filter_toggle::FilterToggle;
use super::folder_viewed_state::FolderViewedState;
use super::loaded_file::LoadedFile;

/// The file tree panel's state: the loaded files as a tree (file indexes
/// match the loaded files, which are in tree order), the filter shared with
/// the diff, collapsed folders, the visible rows, and whether the panel shows.
pub struct FileTreeState {
    tree: FileTree,
    filter: FileFilter,
    /// Folders collapsed in the tree, by path; folders start expanded.
    collapsed_folders: HashSet<String>,
    rows: Vec<TreeRow>,
    extensions: Vec<ExtensionCount>,
    scroll: UniformListScrollHandle,
    visibility: PanelVisibility,
}

impl FileTreeState {
    /// An empty tree with the saved panel visibility and hide toggles.
    pub fn new(layout: ReviewLayoutState) -> Self {
        Self {
            tree: FileTree::default(),
            filter: FileFilter {
                hide_viewed: layout.hide_viewed,
                hide_tests: layout.hide_tests,
                hide_generated: layout.hide_generated,
                hidden_extensions: BTreeSet::new(),
            },
            collapsed_folders: HashSet::new(),
            rows: Vec::new(),
            extensions: Vec::new(),
            scroll: UniformListScrollHandle::new(),
            visibility: layout.file_tree,
        }
    }

    /// The panel visibility and hide toggles, as saved; the review mode is
    /// the detail's and left at its default here.
    pub fn layout(&self) -> ReviewLayoutState {
        ReviewLayoutState {
            file_tree: self.visibility,
            hide_viewed: self.filter.hide_viewed,
            hide_tests: self.filter.hide_tests,
            hide_generated: self.filter.hide_generated,
            stack_review: StackReviewMode::default(),
        }
    }

    pub fn tree(&self) -> &FileTree {
        &self.tree
    }

    pub fn filter(&self) -> &FileFilter {
        &self.filter
    }

    pub fn rows(&self) -> &[TreeRow] {
        &self.rows
    }

    pub fn extensions(&self) -> &[ExtensionCount] {
        &self.extensions
    }

    pub fn scroll(&self) -> &UniformListScrollHandle {
        &self.scroll
    }

    pub fn visibility(&self) -> PanelVisibility {
        self.visibility
    }

    pub fn is_folder_collapsed(&self, path: &str) -> bool {
        self.collapsed_folders.contains(path)
    }

    /// Starts over for another pull request's files: its tree, all folders
    /// expanded, no extension hidden (the hide toggles stay).
    pub fn reset_for(&mut self, tree: FileTree, files: &[LoadedFile]) {
        self.tree = tree;
        self.collapsed_folders.clear();
        self.filter.hidden_extensions.clear();
        let paths: Vec<&str> = files.iter().map(LoadedFile::filename).collect();
        self.extensions = ExtensionCount::of(&paths);
    }

    /// Whether each file passes the filter, by index.
    pub fn shown_files(&self, files: &[LoadedFile], review: &FileReviewState) -> Vec<bool> {
        files
            .iter()
            .map(|loaded| {
                self.filter
                    .shows(loaded.filename(), review.is_viewed(loaded.filename()))
            })
            .collect()
    }

    /// Recomputes the visible rows after files, filters, marks or folders changed.
    pub fn refresh_rows(&mut self, files: &[LoadedFile], review: &FileReviewState) {
        let shown = self.shown_files(files, review);
        self.rows = self.tree.visible_rows(&self.collapsed_folders, |file| {
            shown.get(file).copied().unwrap_or(false)
        });
    }

    pub fn toggle_visibility(&mut self) {
        self.visibility = self.visibility.toggled();
    }

    pub fn toggle_filter(&mut self, toggle: FilterToggle) {
        let flag = match toggle {
            FilterToggle::Viewed => &mut self.filter.hide_viewed,
            FilterToggle::Tests => &mut self.filter.hide_tests,
            FilterToggle::Generated => &mut self.filter.hide_generated,
        };
        *flag = !*flag;
    }

    /// Hides the files with `extension`, or shows them again.
    pub fn toggle_extension(&mut self, extension: &str) {
        if !self.filter.hidden_extensions.remove(extension) {
            self.filter.hidden_extensions.insert(extension.to_owned());
        }
    }

    pub fn toggle_folder(&mut self, path: &str) {
        if !self.collapsed_folders.remove(path) {
            self.collapsed_folders.insert(path.to_owned());
        }
    }

    /// The files a folder's checkbox covers: those under it that the tests,
    /// generated and extension filters let through. Viewed files count even
    /// while hidden, so a folder can read as all viewed.
    pub fn folder_files(&self, folder: NodeId, files: &[LoadedFile]) -> Vec<usize> {
        let without_viewed = FileFilter {
            hide_viewed: false,
            ..self.filter.clone()
        };
        self.tree
            .files_under(folder)
            .into_iter()
            .filter(|file| {
                files
                    .get(*file)
                    .is_some_and(|loaded| without_viewed.shows(loaded.filename(), false))
            })
            .collect()
    }

    /// How much of a folder is viewed.
    pub fn folder_viewed_state(
        &self,
        folder: NodeId,
        files: &[LoadedFile],
        review: &FileReviewState,
    ) -> FolderViewedState {
        let covered = self.folder_files(folder, files);
        let viewed = covered
            .iter()
            .filter(|file| {
                files
                    .get(**file)
                    .is_some_and(|loaded| review.is_viewed(loaded.filename()))
            })
            .count();
        FolderViewedState::of(viewed, covered.len())
    }
}
