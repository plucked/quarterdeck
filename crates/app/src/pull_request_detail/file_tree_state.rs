use std::collections::{HashMap, HashSet};

use gpui::UniformListScrollHandle;
use qd_cache::{PanelVisibility, ReviewLayoutState};
use qd_file_tree::{
    FacetCount, FacetFilter, FileFilter, FileKind, FileTree, FilteredFile, NodeId, TreeNodeKind,
    TreeRow,
};

use super::chip_click::ChipClick;
use super::file_review_state::FileReviewState;
use super::folder_summary::FolderSummary;
use super::folder_viewed_state::FolderViewedState;
use super::line_counts::LineCounts;
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
    /// Every folder's path in the tree, for collapsing them all.
    folder_paths: Vec<String>,
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
    /// Each folder's changed lines and viewed state, as of the last row refresh.
    folder_summaries: HashMap<NodeId, FolderSummary>,
    /// The largest added and removed counts a top-level row shows, which
    /// no deeper row exceeds; the tree's count columns are sized for it.
    widest_line_counts: LineCounts,
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
            folder_paths: Vec::new(),
            rows: Vec::new(),
            kind_counts: Vec::new(),
            type_counts: Vec::new(),
            shown_count: 0,
            file_count: 0,
            are_types_expanded: false,
            folder_summaries: HashMap::new(),
            widest_line_counts: LineCounts::default(),
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

    /// Folder `folder`'s changed lines and viewed state.
    pub fn folder_summary(&self, folder: NodeId) -> Option<FolderSummary> {
        self.folder_summaries.get(&folder).copied()
    }

    pub fn widest_line_counts(&self) -> LineCounts {
        self.widest_line_counts
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
        self.folder_paths = tree.folder_paths();
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

        // Every folder's totals in one walk, so drawing a row looks them up.
        let mut summaries = HashMap::new();
        let mut widest = LineCounts::default();
        for root in self.tree.roots().to_vec() {
            let totals = self.summarize(root, files, review, &mut summaries);
            widest = widest.widest(totals.lines);
        }
        self.folder_summaries = summaries;
        self.widest_line_counts = widest;
    }

    /// The totals of node `node` (a file's own, or a folder's files
    /// together), recording every folder's summary in `summaries` on the way.
    fn summarize(
        &self,
        node: NodeId,
        files: &[LoadedFile],
        review: &FileReviewState,
        summaries: &mut HashMap<NodeId, FolderSummary>,
    ) -> NodeTotals {
        match &self.tree.node(node).kind {
            TreeNodeKind::File { file } => {
                let Some(loaded) = files.get(*file) else {
                    return NodeTotals::default();
                };
                // The folder checkbox covers files the chips let through, viewed or not.
                let is_covered = self
                    .filter
                    .shows_ignoring_viewed(&filtered_file(loaded, &FileReviewState::default()));
                let is_viewed = review.is_viewed(loaded.filename());
                NodeTotals {
                    lines: LineCounts {
                        additions: loaded.file.additions,
                        deletions: loaded.file.deletions,
                    },
                    covered: usize::from(is_covered),
                    viewed: usize::from(is_covered && is_viewed),
                }
            }
            TreeNodeKind::Folder { children } => {
                let totals = children
                    .iter()
                    .map(|child| self.summarize(*child, files, review, summaries))
                    .fold(NodeTotals::default(), NodeTotals::plus);
                summaries.insert(
                    node,
                    FolderSummary {
                        lines: totals.lines,
                        viewed: FolderViewedState::of(totals.viewed, totals.covered),
                    },
                );
                totals
            }
        }
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

    /// Whether the tree has folders at all (a flat list has nothing to fold).
    pub fn has_folders(&self) -> bool {
        !self.folder_paths.is_empty()
    }

    /// Whether some folder is expanded, so collapsing all would change something.
    pub fn has_expanded_folder(&self) -> bool {
        self.collapsed_folders.len() < self.folder_paths.len()
    }

    /// Collapses every folder, nested ones too, leaving the top level.
    pub fn collapse_all_folders(&mut self) {
        self.collapsed_folders = self.folder_paths.iter().cloned().collect();
    }

    /// Expands every folder.
    pub fn expand_all_folders(&mut self) {
        self.collapsed_folders.clear();
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

/// Totals of the files under a tree node, gathered for its folders' summaries.
#[derive(Clone, Copy, Default)]
struct NodeTotals {
    lines: LineCounts,
    /// Files the chips let through (a folder checkbox covers these).
    covered: usize,
    /// Of those, the viewed ones.
    viewed: usize,
}

impl NodeTotals {
    fn plus(self, other: Self) -> Self {
        Self {
            lines: self.lines.plus(other.lines),
            covered: self.covered + other.covered,
            viewed: self.viewed + other.viewed,
        }
    }
}

/// What the filter looks at in `loaded`, with its viewed mark from `review`.
fn filtered_file<'file>(
    loaded: &'file LoadedFile,
    review: &FileReviewState,
) -> FilteredFile<'file> {
    FilteredFile {
        facets: &loaded.facets,
        is_viewed: review.is_viewed(loaded.filename()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collapsing all folds every folder, nested ones too; expanding all
    /// unfolds them, and a flat list has nothing to fold.
    #[test]
    fn folds_every_folder_at_once() {
        let mut state = FileTreeState::new(ReviewLayoutState::default());
        state.reset_for(FileTree::build(&[
            "src/app/main.rs",
            "src/lib.rs",
            "docs/a.md",
        ]));
        assert!(state.has_folders());
        assert!(state.has_expanded_folder());

        state.collapse_all_folders();
        assert!(!state.has_expanded_folder());
        assert!(state.is_folder_collapsed("src/app"));

        state.expand_all_folders();
        assert!(state.has_expanded_folder());
        assert!(!state.is_folder_collapsed("src"));

        state.reset_for(FileTree::build(&["a.rs", "b.rs"]));
        assert!(!state.has_folders());
    }
}
