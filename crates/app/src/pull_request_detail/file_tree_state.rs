use std::collections::{BTreeSet, HashSet};

use gpui::UniformListScrollHandle;
use qc_cache::{PanelVisibility, ReviewLayoutState, StackReviewMode};
use qc_file_tree::{FileFilter, FileGroup, FileTree, FileTypeCount, FilteredFile, NodeId, TreeRow};

use super::file_review_state::FileReviewState;
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
    file_types: Vec<FileTypeCount>,
    /// How many files were moved or copied without changes.
    unchanged_moves: usize,
    scroll: UniformListScrollHandle,
    visibility: PanelVisibility,
}

impl FileTreeState {
    /// An empty tree with the saved panel visibility and hide toggles.
    pub fn new(layout: ReviewLayoutState) -> Self {
        Self {
            tree: FileTree::default(),
            filter: FileFilter {
                hidden_groups: layout.hidden_groups,
                hidden_file_types: BTreeSet::new(),
            },
            collapsed_folders: HashSet::new(),
            rows: Vec::new(),
            file_types: Vec::new(),
            unchanged_moves: 0,
            scroll: UniformListScrollHandle::new(),
            visibility: layout.file_tree,
        }
    }

    /// The panel visibility and hide toggles, as saved; the review mode is
    /// the detail's and left at its default here.
    pub fn layout(&self) -> ReviewLayoutState {
        ReviewLayoutState {
            file_tree: self.visibility,
            hidden_groups: self.filter.hidden_groups.clone(),
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

    pub fn file_types(&self) -> &[FileTypeCount] {
        &self.file_types
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
    /// expanded, no file type hidden (the hide toggles stay).
    pub fn reset_for(&mut self, tree: FileTree, files: &[LoadedFile]) {
        self.tree = tree;
        self.collapsed_folders.clear();
        self.filter.hidden_file_types.clear();
        let paths: Vec<&str> = files.iter().map(LoadedFile::filename).collect();
        self.file_types = FileTypeCount::of(&paths);
        self.unchanged_moves = files
            .iter()
            .filter(|loaded| loaded.is_unchanged_move())
            .count();
    }

    pub fn unchanged_moves(&self) -> usize {
        self.unchanged_moves
    }

    /// Whether each file passes the filter, by index.
    pub fn shown_files(&self, files: &[LoadedFile], review: &FileReviewState) -> Vec<bool> {
        files
            .iter()
            .map(|loaded| self.filter.shows(&filtered_file(loaded, review)))
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

    /// Hides `group`, or shows it again.
    pub fn toggle_group(&mut self, group: FileGroup) {
        self.filter.toggle_group(group);
    }

    /// Shows every file when anything is hidden, else hides every group
    /// and file type (to show them again one by one).
    pub fn toggle_all_filters(&mut self) {
        if self.filter.is_active() {
            self.filter.show_all();
        } else {
            let file_types = self.file_types.iter().map(|count| count.file_type.clone());
            self.filter.hide_all(file_types);
        }
    }

    /// Whether `loaded` passes the filter.
    pub fn shows(&self, loaded: &LoadedFile, review: &FileReviewState) -> bool {
        self.filter.shows(&filtered_file(loaded, review))
    }

    /// Hides the files of `file_type`, or shows them again.
    pub fn toggle_file_type(&mut self, file_type: &str) {
        if !self.filter.hidden_file_types.remove(file_type) {
            self.filter.hidden_file_types.insert(file_type.to_owned());
        }
    }

    pub fn toggle_folder(&mut self, path: &str) {
        if !self.collapsed_folders.remove(path) {
            self.collapsed_folders.insert(path.to_owned());
        }
    }

    /// The files a folder's checkbox covers: those under it that the tests,
    /// generated and file type filters let through. Viewed files count even
    /// while hidden, so a folder can read as all viewed.
    pub fn folder_files(&self, folder: NodeId, files: &[LoadedFile]) -> Vec<usize> {
        let mut without_viewed = self.filter.clone();
        without_viewed.hidden_groups.remove(&FileGroup::Viewed);
        self.tree
            .files_under(folder)
            .into_iter()
            .filter(|file| {
                files.get(*file).is_some_and(|loaded| {
                    without_viewed.shows(&FilteredFile {
                        is_viewed: false,
                        ..filtered_file(loaded, &FileReviewState::default())
                    })
                })
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

/// What the filter looks at in `loaded`, with its viewed mark from `review`.
fn filtered_file<'file>(
    loaded: &'file LoadedFile,
    review: &FileReviewState,
) -> FilteredFile<'file> {
    FilteredFile {
        path: loaded.filename(),
        is_viewed: review.is_viewed(loaded.filename()),
        is_unchanged_move: loaded.is_unchanged_move(),
    }
}
