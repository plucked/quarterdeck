use std::collections::HashSet;

use gpui::UniformListScrollHandle;
use qc_cache::{PanelVisibility, ReviewLayoutState};
use qc_file_tree::{
    FacetCount, FacetFilter, FileFilter, FileKind, FileTree, FilteredFile, NodeId, TreeRow,
};

use super::chip_click::ChipClick;
use super::file_review_state::FileReviewState;
use super::folder_viewed_state::FolderViewedState;
use super::loaded_file::LoadedFile;

/// The file tree panel's state: the loaded files as a tree (file indexes
/// match the loaded files, which are in tree order), the filter shared with
/// the diff with its chips' counts, collapsed folders, the visible rows, and
/// whether the panel shows.
pub struct FileTreeState {
    tree: FileTree,
    filter: FileFilter,
    /// Folders collapsed in the tree, by path; folders start expanded.
    collapsed_folders: HashSet<String>,
    rows: Vec<TreeRow>,
    /// The kind chips' counts, as of the last row refresh.
    kind_counts: Vec<FacetCount<FileKind>>,
    /// The type chips' counts, most files first, as of the last row refresh.
    type_counts: Vec<FacetCount<String>>,
    /// Files the filter shows, of all loaded, as of the last row refresh.
    shown_count: usize,
    file_count: usize,
    /// Whether the type row shows every type, not only the most common.
    are_types_expanded: bool,
    scroll: UniformListScrollHandle,
    visibility: PanelVisibility,
}

impl FileTreeState {
    /// An empty tree with the saved panel visibility, viewed switch and hidden kinds.
    pub fn new(layout: ReviewLayoutState) -> Self {
        Self {
            tree: FileTree::default(),
            filter: FileFilter {
                hide_viewed: layout.hide_viewed,
                kinds: FacetFilter {
                    hidden: layout.hidden_kinds,
                    ..FacetFilter::default()
                },
                file_types: FacetFilter::default(),
            },
            collapsed_folders: HashSet::new(),
            rows: Vec::new(),
            kind_counts: Vec::new(),
            type_counts: Vec::new(),
            shown_count: 0,
            file_count: 0,
            are_types_expanded: false,
            scroll: UniformListScrollHandle::new(),
            visibility: layout.file_tree,
        }
    }

    /// The panel visibility, viewed switch and hidden kinds, as saved; the
    /// review mode and comment destination are the detail's and left at
    /// their defaults here.
    pub fn layout(&self) -> ReviewLayoutState {
        ReviewLayoutState {
            file_tree: self.visibility,
            hide_viewed: self.filter.hide_viewed,
            hidden_kinds: self.filter.kinds.hidden.clone(),
            ..ReviewLayoutState::default()
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

    pub fn kind_counts(&self) -> &[FacetCount<FileKind>] {
        &self.kind_counts
    }

    pub fn type_counts(&self) -> &[FacetCount<String>] {
        &self.type_counts
    }

    /// Files shown and files loaded.
    pub fn shown_of_total(&self) -> (usize, usize) {
        (self.shown_count, self.file_count)
    }

    pub fn are_types_expanded(&self) -> bool {
        self.are_types_expanded
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
    /// expanded, no chip picked and no type hidden (the viewed switch and
    /// hidden kinds stay).
    pub fn reset_for(&mut self, tree: FileTree) {
        self.tree = tree;
        self.collapsed_folders.clear();
        self.filter.kinds.only.clear();
        self.filter.file_types = FacetFilter::default();
        self.are_types_expanded = false;
    }

    /// Whether each file passes the filter, by index.
    pub fn shown_files(&self, files: &[LoadedFile], review: &FileReviewState) -> Vec<bool> {
        files
            .iter()
            .map(|loaded| self.filter.shows(&filtered_file(loaded, review)))
            .collect()
    }

    /// Recomputes the visible rows and the chips' counts after files,
    /// filters, marks or folders changed.
    pub fn refresh_rows(&mut self, files: &[LoadedFile], review: &FileReviewState) {
        let shown = self.shown_files(files, review);
        self.rows = self.tree.visible_rows(&self.collapsed_folders, |file| {
            shown.get(file).copied().unwrap_or(false)
        });

        // The counts every chip shows, given the other row and the viewed switch.
        let filtered: Vec<FilteredFile<'_>> = files
            .iter()
            .map(|loaded| filtered_file(loaded, review))
            .collect();
        self.kind_counts = self.filter.kind_counts(&filtered);
        self.type_counts = self.filter.file_type_counts(&filtered);
        self.shown_count = shown.iter().filter(|is_shown| **is_shown).count();
        self.file_count = files.len();
    }

    pub fn toggle_visibility(&mut self) {
        self.visibility = self.visibility.toggled();
    }

    /// Hides viewed files, or shows them again.
    pub fn toggle_hide_viewed(&mut self) {
        self.filter.hide_viewed = !self.filter.hide_viewed;
    }

    /// Picks or hides a kind chip, as `click` asks.
    pub fn click_kind(&mut self, kind: FileKind, click: ChipClick) {
        match click {
            ChipClick::Pick => self.filter.kinds.toggle_only(&kind),
            ChipClick::Hide => self.filter.kinds.toggle_hidden(&kind),
        }
    }

    /// Picks or hides a type chip, as `click` asks.
    pub fn click_file_type(&mut self, file_type: &str, click: ChipClick) {
        let file_type = file_type.to_owned();
        match click {
            ChipClick::Pick => self.filter.file_types.toggle_only(&file_type),
            ChipClick::Hide => self.filter.file_types.toggle_hidden(&file_type),
        }
    }

    /// Clears both chip rows (the summary's Reset); the viewed switch stays.
    pub fn clear_chips(&mut self) {
        self.filter.clear_chips();
    }

    /// Shows every type chip, or only the most common again.
    pub fn toggle_types_expanded(&mut self) {
        self.are_types_expanded = !self.are_types_expanded;
    }

    /// Whether `loaded` passes the filter.
    pub fn shows(&self, loaded: &LoadedFile, review: &FileReviewState) -> bool {
        self.filter.shows(&filtered_file(loaded, review))
    }

    pub fn toggle_folder(&mut self, path: &str) {
        if !self.collapsed_folders.remove(path) {
            self.collapsed_folders.insert(path.to_owned());
        }
    }

    /// The files a folder's checkbox covers: those under it that the kind
    /// and type chips let through. Viewed files count even while hidden, so
    /// a folder can read as all viewed.
    pub fn folder_files(&self, folder: NodeId, files: &[LoadedFile]) -> Vec<usize> {
        let unmarked = FileReviewState::default();
        self.tree
            .files_under(folder)
            .into_iter()
            .filter(|file| {
                files.get(*file).is_some_and(|loaded| {
                    self.filter
                        .shows_ignoring_viewed(&filtered_file(loaded, &unmarked))
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
