use std::collections::{HashMap, HashSet};

use gpui::UniformListScrollHandle;
use qd_cache::{PanelVisibility, ReviewLayoutState};
use qd_file_tree::{
    FacetCount, FacetFilter, FileFilter, FileKind, FileTree, FilteredFile, NodeId, TreeNodeKind,
    TreeRow,
};
use qd_studio::SmartBucket;

use super::file_review_state::FileReviewState;
use super::folder_summary::FolderSummary;
use super::folder_viewed_state::FolderViewedState;
use super::line_counts::LineCounts;
use super::loaded_file::LoadedFile;
use crate::facet_chips::ChipClick;

/// The file tree panel's state: the loaded files as a tree (file indexes
/// match the loaded files, which are in tree order), the filter shared with
/// the diff with its chips' counts, collapsed folders, the visible rows, and
/// whether the panel shows.
pub struct FileTreeState {
    tree: FileTree,
    filter: FileFilter,
    pub(super) smart_choices: FacetFilter<u32>,
    pub(super) smart_counts: Vec<FacetCount<u32>>,
    smart_buckets: Vec<SmartBucket>,
    smart_membership: HashMap<String, Vec<u32>>,
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
    /// Each folder's changed lines, file count, heat and viewed state, as of the last refresh.
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
            smart_choices: FacetFilter::default(),
            smart_counts: Vec::new(),
            smart_buckets: Vec::new(),
            smart_membership: HashMap::new(),
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

    /// Folder `folder`'s changed lines, file count, heat and viewed state.
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
        self.set_smart_buckets(Vec::new());
    }

    /// Whether each file passes the filter, by index.
    pub fn shown_files(&self, files: &[LoadedFile], review: &FileReviewState) -> Vec<bool> {
        files
            .iter()
            .map(|loaded| self.shows(loaded, review))
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
        // Counts retain all values, while matching counts include the smart row.
        let smart_files: Vec<_> = files
            .iter()
            .zip(&filtered)
            .filter(|(loaded, _)| self.smart_admits(loaded.filename()))
            .map(|(_, file)| *file)
            .collect();
        let kinds = self.filter.kind_counts(&smart_files);
        let types = self.filter.file_type_counts(&smart_files);
        for count in &mut self.kind_counts {
            count.matching = kinds
                .iter()
                .find(|other| other.value == count.value)
                .map_or(0, |other| other.matching);
        }
        for count in &mut self.type_counts {
            count.matching = types
                .iter()
                .find(|other| other.value == count.value)
                .map_or(0, |other| other.matching);
        }
        let mut counts: HashMap<u32, (usize, usize)> = HashMap::new();
        for (loaded, file) in files.iter().zip(&filtered) {
            if let Some(orders) = self.smart_membership.get(loaded.filename()) {
                for order in orders {
                    let count = counts.entry(*order).or_default();
                    count.0 += 1;
                    count.1 += usize::from(self.filter.shows(file));
                }
            }
        }
        self.smart_counts = self
            .smart_buckets
            .iter()
            .map(|bucket| {
                let (total, matching) = counts.get(&bucket.order).copied().unwrap_or_default();
                FacetCount {
                    value: bucket.order,
                    total,
                    matching,
                }
            })
            .collect();
        self.shown_count = shown.iter().filter(|is_shown| **is_shown).count();
        self.file_count = files.len();

        // Every folder's totals in one walk, so drawing a row looks them up.
        let mut summaries = HashMap::new();
        let mut widest = LineCounts::default();
        for root in self.tree.roots().to_vec() {
            let totals = self.summarize(root, files, review, &mut summaries);
            widest = widest.widest(totals.lines);
        }
        // Scale heat to matching files, including descendants of collapsed folders.
        // Folders with no matching files contribute zero to the scale.
        let largest = summaries
            .values()
            .map(|summary| summary.file_count)
            .max()
            .unwrap_or_default();
        for summary in summaries.values_mut() {
            summary.heat = folder_heat(summary.file_count, largest);
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
                let filtered = filtered_file(loaded, review);
                let is_covered = self.smart_admits(loaded.filename())
                    && self.filter.shows_ignoring_viewed(&filtered);
                let mut totals = NodeTotals {
                    covered: usize::from(is_covered),
                    viewed: usize::from(is_covered && filtered.is_viewed),
                    ..NodeTotals::default()
                };
                // Counts and heat describe only files that pass the active filters.
                if self.shows(loaded, review) {
                    totals.file_count = 1;
                    totals.lines = LineCounts {
                        additions: loaded.file.additions,
                        deletions: loaded.file.deletions,
                    };
                }
                totals
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
                        file_count: totals.file_count,
                        heat: 0.,
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

    /// Shows the panel; returns whether it was hidden, so the caller saves the change.
    pub fn show_panel(&mut self) -> bool {
        let was_hidden = self.visibility != PanelVisibility::Shown;
        self.visibility = PanelVisibility::Shown;
        was_hidden
    }

    /// Expands every collapsed folder above the file at `path`, so its row shows
    /// once the rows are refreshed.
    pub fn expand_folders_above(&mut self, path: &str) {
        self.collapsed_folders.retain(|folder| {
            !path
                .strip_prefix(folder.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        });
    }

    /// The visible row of file `file`, as of the last row refresh; none while
    /// the filter hides it or a folder above it is collapsed.
    pub fn row_of_file(&self, file: usize) -> Option<usize> {
        self.rows.iter().position(|row| {
            matches!(self.tree.node(row.node).kind, TreeNodeKind::File { file: shown } if shown == file)
        })
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
        self.smart_choices = FacetFilter::default();
    }

    /// Shows every type chip, or only the most common again.
    pub fn toggle_types_expanded(&mut self) {
        self.are_types_expanded = !self.are_types_expanded;
    }

    /// Whether `loaded` passes the filter.
    pub fn shows(&self, loaded: &LoadedFile, review: &FileReviewState) -> bool {
        self.smart_admits(loaded.filename()) && self.filter.shows(&filtered_file(loaded, review))
    }

    /// Replace membership only when a plan changes; ordinary refreshes preserve picks.
    pub fn set_smart_buckets(&mut self, buckets: Vec<SmartBucket>) {
        if self.smart_buckets == buckets {
            return;
        }
        self.smart_membership.clear();
        for bucket in &buckets {
            for path in &bucket.files {
                self.smart_membership
                    .entry(path.clone())
                    .or_default()
                    .push(bucket.order);
            }
        }
        self.smart_buckets = buckets;
        self.smart_choices = FacetFilter::default();
    }

    fn smart_admits(&self, path: &str) -> bool {
        self.smart_choices.admits_any(
            self.smart_membership
                .get(path)
                .map_or(&[][..], Vec::as_slice),
        )
    }

    pub fn click_smart_bucket(&mut self, order: u32, click: ChipClick) {
        match click {
            ChipClick::Pick => self.smart_choices.toggle_only(&order),
            ChipClick::Hide => self.smart_choices.toggle_hidden(&order),
        }
    }

    pub fn has_chips_set(&self) -> bool {
        self.filter.has_chips_set() || self.smart_choices.is_active()
    }

    pub fn describe_filter(&self) -> Vec<String> {
        let mut parts = self.filter.describe();
        for bucket in &self.smart_buckets {
            if self.smart_choices.only.contains(&bucket.order) {
                parts.push(bucket.label.clone());
            }
            if self.smart_choices.hidden.contains(&bucket.order) {
                parts.push(format!("not {}", bucket.label));
            }
        }
        parts
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
                    self.smart_admits(loaded.filename())
                        && self
                            .filter
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
    file_count: usize,
    /// Files the chips let through (a folder checkbox covers these).
    covered: usize,
    /// Of those, the viewed ones.
    viewed: usize,
}

impl NodeTotals {
    fn plus(self, other: Self) -> Self {
        Self {
            lines: self.lines.plus(other.lines),
            file_count: self.file_count + other.file_count,
            covered: self.covered + other.covered,
            viewed: self.viewed + other.viewed,
        }
    }
}

/// One changed file stays cold; the largest folder is warm. A logarithmic
/// scale keeps smaller hotspots visible beside a very large subtree.
fn folder_heat(file_count: usize, largest: usize) -> f64 {
    if file_count <= 1 || largest <= 1 {
        return 0.;
    }
    let logarithm = |count| f64::from(u32::try_from(count).unwrap_or(u32::MAX)).ln();
    (logarithm(file_count) / logarithm(largest)).clamp(0., 1.)
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
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;

    /// Smart picks intersect the other rows, feed counts and folder marks,
    /// and never acquire files added after the plan was generated.
    #[test]
    fn smart_buckets_compose_with_filters_and_folder_review() {
        let paths = ["src/a.rs", "src/b.rs", "src/readme.md", "src/new.rs"];
        let files: Vec<_> = paths
            .iter()
            .map(|path| {
                LoadedFile::build(
                    PullRequestFile {
                        filename: (*path).into(),
                        previous_filename: None,
                        status: "modified".into(),
                        additions: 1,
                        deletions: 0,
                        patch: None,
                        sha: None,
                    },
                    DiffOptions::default(),
                )
            })
            .collect();
        let mut state = FileTreeState::new(ReviewLayoutState::default());
        state.reset_for(FileTree::build(&paths));
        let bucket = |order, paths: &[&str]| SmartBucket {
            order,
            label: format!("Step {order}"),
            description: "Inspect this part".into(),
            suggest_viewed: false,
            files: paths.iter().map(|path| (*path).to_owned()).collect(),
        };
        state.set_smart_buckets(vec![
            bucket(1, &["src/a.rs", "src/readme.md"]),
            bucket(2, &["src/a.rs", "src/b.rs"]),
        ]);
        let mut review = FileReviewState::default();
        review.toggle_viewed(&files[0]);
        state.click_smart_bucket(1, ChipClick::Pick);
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (2, 4));
        let folder = state.tree.roots()[0];
        assert_eq!(state.folder_files(folder, &files), [0, 2]);
        assert_eq!(
            state.folder_viewed_state(folder, &files, &review),
            FolderViewedState::Some
        );
        assert!(!state.shows(&files[3], &review));
        assert_eq!(
            state
                .type_counts
                .iter()
                .find(|c| c.value == ".rs")
                .unwrap()
                .matching,
            1
        );
        state.click_file_type(".rs", ChipClick::Pick);
        state.toggle_hide_viewed();
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (0, 4));
        assert_eq!(
            state
                .smart_counts
                .iter()
                .map(|c| c.matching)
                .collect::<Vec<_>>(),
            [0, 1]
        );
        assert_eq!(state.folder_files(folder, &files), [0]);
        // Hiding an overlapping bucket overrides a picked one.
        state.toggle_hide_viewed();
        state.click_smart_bucket(2, ChipClick::Hide);
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (0, 4));
        state.clear_chips();
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (4, 4));
        assert!(!state.has_chips_set());
        // Loading another scope drops its choices and membership.
        state.click_smart_bucket(1, ChipClick::Pick);
        state.reset_for(FileTree::build(&paths));
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (4, 4));
        assert_eq!(state.smart_counts, Vec::<FacetCount<u32>>::new());
    }

    /// Subtree totals count every status once, including patchless moves and
    /// binaries, and remain stable through folding and visible viewed marks.
    #[test]
    fn folder_counts_and_heat_cover_all_descendants() {
        // Mix direct and nested files, a compacted folder chain, and a root file.
        let files: Vec<_> = [
            ("src/app/main.rs", "modified", 10, 4),
            ("src/app/moved.rs", "renamed", 0, 0),
            ("src/image.png", "modified", 0, 0),
            ("src/deleted.rs", "removed", 0, 7),
            ("src/new.rs", "added", 3, 0),
            ("docs/deep/page.md", "copied", 0, 0),
            ("Cargo.toml", "modified", 1, 1),
        ]
        .into_iter()
        .map(|(filename, status, additions, deletions)| {
            LoadedFile::build(
                PullRequestFile {
                    filename: filename.into(),
                    previous_filename: (status == "renamed").then(|| "old/moved.rs".into()),
                    status: status.into(),
                    additions,
                    deletions,
                    patch: None,
                    sha: None,
                },
                DiffOptions::default(),
            )
        })
        .collect();
        let paths: Vec<_> = files.iter().map(LoadedFile::filename).collect();
        let mut state = FileTreeState::new(ReviewLayoutState::default());
        let mut review = FileReviewState::default();
        state.reset_for(FileTree::build(&paths));
        state.refresh_rows(&files, &review);
        let find_folder = |path| {
            *state
                .folder_summaries
                .keys()
                .find(|node| state.tree.node(**node).path == path)
                .unwrap()
        };
        let source = find_folder("src");
        let nested = find_folder("src/app");
        let docs = find_folder("docs/deep");
        let before = state.folder_summary(source).unwrap();
        assert_eq!(before.file_count, 5);
        assert_eq!(before.lines.additions, 13);
        assert_eq!(before.lines.deletions, 11);
        assert_eq!(state.folder_summary(nested).unwrap().file_count, 2);
        assert_eq!(state.folder_summary(docs).unwrap().file_count, 1);
        assert!(before.heat > state.folder_summary(nested).unwrap().heat);
        assert!(
            state.folder_summary(nested).unwrap().heat > state.folder_summary(docs).unwrap().heat
        );

        // Viewed files still count while shown; folding keeps all descendants counted.
        review.toggle_viewed(&files[0]);
        state.collapse_all_folders();
        state.refresh_rows(&files, &review);
        let after = state.folder_summary(source).unwrap();
        assert_eq!(after.file_count, before.file_count);
        assert_eq!(after.lines, before.lines);
        assert!((after.heat - before.heat).abs() < f64::EPSILON);
        assert!(state.row_of_file(0).is_none());

        // A different, empty review leaves no old counts or scale behind.
        state.reset_for(FileTree::default());
        state.refresh_rows(&[], &FileReviewState::default());
        assert!(state.folder_summaries.is_empty());
    }

    /// Hidden viewed files contribute neither counts nor lines nor heat,
    /// while folder checkboxes still track reviewed descendants.
    #[test]
    fn folder_totals_follow_visible_files() {
        // A fully reviewed folder starts largest; another is partly reviewed.
        let paths = [
            "src/nested/a.rs",
            "src/nested/b.rs",
            "src/c.rs",
            "docs/d.md",
            "reviewed/e.rs",
            "reviewed/f.rs",
            "reviewed/g.rs",
            "reviewed/h.rs",
        ];
        let files: Vec<_> = paths
            .iter()
            .map(|path| {
                LoadedFile::build(
                    PullRequestFile {
                        filename: (*path).into(),
                        previous_filename: None,
                        status: "modified".into(),
                        additions: 2,
                        deletions: 1,
                        patch: None,
                        sha: None,
                    },
                    DiffOptions::default(),
                )
            })
            .collect();
        let mut state = FileTreeState::new(ReviewLayoutState::default());
        let mut review = FileReviewState::default();
        for index in [0, 4, 5, 6, 7] {
            review.toggle_viewed(&files[index]);
        }
        state.reset_for(FileTree::build(&paths));
        state.refresh_rows(&files, &review);
        let find_folder = |path| {
            *state
                .folder_summaries
                .keys()
                .find(|node| state.tree.node(**node).path == path)
                .unwrap()
        };
        let source = find_folder("src");
        let nested = find_folder("src/nested");
        let reviewed = find_folder("reviewed");
        assert_eq!(state.folder_summary(source).unwrap().file_count, 3);

        // Hide viewed: summaries shrink, and the hidden largest folder no longer sets the scale.
        state.toggle_hide_viewed();
        state.refresh_rows(&files, &review);
        let summary = state.folder_summary(source).unwrap();
        assert_eq!(summary.file_count, 2);
        assert_eq!(
            summary.lines,
            LineCounts {
                additions: 4,
                deletions: 2
            }
        );
        assert!((summary.heat - 1.).abs() < f64::EPSILON);
        assert_eq!(summary.viewed, FolderViewedState::Some);
        assert_eq!(state.folder_summary(nested).unwrap().file_count, 1);
        assert_eq!(state.folder_summary(reviewed).unwrap().file_count, 0);
        assert_eq!(
            state.folder_summary(reviewed).unwrap().viewed,
            FolderViewedState::All
        );
        assert!(!state.rows.iter().any(|row| row.node == reviewed));

        // Showing viewed files restores the totals; type filters narrow them too.
        state.toggle_hide_viewed();
        state.refresh_rows(&files, &review);
        assert_eq!(state.folder_summary(source).unwrap().file_count, 3);
        state.toggle_hide_viewed();
        state.click_file_type(".md", ChipClick::Pick);
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (1, files.len()));
        assert_eq!(state.folder_summary(source).unwrap().file_count, 0);
        assert_eq!(
            state.folder_summary(source).unwrap().lines,
            LineCounts::default()
        );

        // Viewing the last matching file leaves an empty, cold tree; unmarking restores it.
        review.toggle_viewed(&files[3]);
        state.refresh_rows(&files, &review);
        assert_eq!(state.rows, Vec::<TreeRow>::new());
        assert_eq!(state.widest_line_counts(), LineCounts::default());
        assert!(
            state
                .folder_summaries
                .values()
                .all(|summary| summary.file_count == 0 && summary.heat.abs() < f64::EPSILON)
        );
        review.toggle_viewed(&files[3]);
        state.refresh_rows(&files, &review);
        assert_eq!(state.shown_of_total(), (1, files.len()));
        assert_eq!(state.rows.len(), 2);
    }

    /// Empty and single-file folders stay cold; larger counts increase heat
    /// logarithmically, including counts beyond the layout's 16-bit range.
    #[test]
    fn heat_handles_small_and_large_file_sets() {
        assert!(folder_heat(0, 0).abs() < f64::EPSILON);
        assert!(folder_heat(1, 1).abs() < f64::EPSILON);
        assert!(folder_heat(1, 100).abs() < f64::EPSILON);
        assert!((folder_heat(10, 100) - 0.5).abs() < f64::EPSILON);
        assert!((folder_heat(100, 100) - 1.).abs() < f64::EPSILON);
        assert!((folder_heat(10_000, 100_000) - 0.8).abs() < f64::EPSILON);
    }

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

    /// Showing a file in the tree expands only the folders above it, and its
    /// row is found once the rows are refreshed.
    #[test]
    fn expands_the_folders_above_a_file() {
        // Set up: every folder collapsed, so only the top-level rows show.
        let mut state = FileTreeState::new(ReviewLayoutState::default());
        let paths = ["src/app/main.rs", "src/lib.rs", "docs/a.md", "srcx/b.rs"];
        state.reset_for(FileTree::build(&paths));
        state.collapse_all_folders();
        state.rows = state.tree.visible_rows(&state.collapsed_folders, |_| true);
        // A file's index is its place in the paths the tree was built from.
        let main_file = 0;
        assert_eq!(state.row_of_file(main_file), None);

        // Act: expand what lies above `src/app/main.rs`.
        state.expand_folders_above("src/app/main.rs");
        state.rows = state.tree.visible_rows(&state.collapsed_folders, |_| true);

        // Expect: `src` and `src/app` open, `docs` and `srcx` still collapsed.
        assert!(!state.is_folder_collapsed("src"));
        assert!(!state.is_folder_collapsed("src/app"));
        assert!(state.is_folder_collapsed("docs"));
        assert!(state.is_folder_collapsed("srcx"));
        assert!(state.row_of_file(main_file).is_some());
    }
}
