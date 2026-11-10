use gpui::Context;
use qd_cache::OthersFilter;
use qd_github::RepoRef;

use super::ReviewQueue;
use super::others_filtering::OthersFacet;
use crate::app_settings::AppSettings;
use crate::avatar_images::AvatarImages;
use crate::ui_state::UiStateRecorder;

impl ReviewQueue {
    /// Opens `repo`'s filter under its header (closing another one), or
    /// closes it; a folded repository unfolds so the filter shows.
    pub(super) fn toggle_others_filter_panel(&mut self, repo: &RepoRef, cx: &mut Context<Self>) {
        self.others_tab_mut().toggle_filter_panel(repo);
        // The opened filter's chips are counted now, not while drawing.
        self.update_others_shown();
        if self.is_repository_collapsed(repo) {
            self.toggle_repository_group(repo, cx);
            return;
        }
        self.rebuild_rows();
        cx.notify();
    }

    /// Changes `repo`'s filter with `change`, records it and shows the result.
    pub(super) fn change_others_filter(
        &mut self,
        repo: &RepoRef,
        change: impl FnOnce(&mut OthersFilter),
        cx: &mut Context<Self>,
    ) {
        let filter = self.others_tab_mut().change_filter(repo, change);
        UiStateRecorder::record_others_filter(cx, repo.clone(), filter);
        self.update_others_shown();
        self.rebuild_rows();
        cx.notify();
    }

    /// Shows all of a chip row's chips, or only the most common again.
    pub(super) fn toggle_chip_row(
        &mut self,
        repo: &RepoRef,
        facet: OthersFacet,
        cx: &mut Context<Self>,
    ) {
        self.others_tab_mut().toggle_chip_row(repo, facet);
        cx.notify();
    }

    /// Starts loading the avatars of the listed pull requests' authors (the
    /// rows and the author chips show them); each is downloaded once per session.
    pub(super) fn request_author_avatars(&self, cx: &mut Context<Self>) {
        let urls: Vec<String> = self
            .lists()
            .iter()
            .map(|pull_request| pull_request.author.avatar_url.clone())
            .collect();
        AvatarImages::load(cx, urls);
    }

    /// Follows a change of the settings: changed hidden branch rules show
    /// the lists again; changed watched repositories show the new set at
    /// once (a removed one goes, an added one shows empty) and refresh,
    /// which fetches the added one's pull requests.
    pub(super) fn on_settings_changed(&mut self, cx: &mut Context<Self>) {
        // Changed hidden branch rules show the lists again, without refetching.
        if self.hidden_branch_rules() != AppSettings::hidden_branch_rules(cx) {
            let fetched = self.take_fetched();
            self.replace_lists(fetched, cx);
        }
        let watched = AppSettings::watched_repositories(cx).to_vec();
        if !self.others_tab_mut().set_watched(&watched) {
            return;
        }
        self.update_others_shown();
        self.rebuild_rows();
        self.refresh(cx);
        cx.notify();
    }
}
