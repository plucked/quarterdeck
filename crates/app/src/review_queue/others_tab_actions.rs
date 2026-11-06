use gpui::Context;
use qd_cache::OthersFilter;
use qd_github::RepoRef;

use super::ReviewQueue;
use super::others_filtering::OthersFacet;
use crate::app_settings::AppSettings;
use crate::avatar_images::AvatarImages;
use crate::display_date::unix_now;
use crate::ui_state::UiStateRecorder;

impl ReviewQueue {
    /// Opens `repo`'s filter under its header (closing another one), or
    /// closes it; a folded repository unfolds so the filter shows.
    pub(super) fn toggle_others_filter_panel(&mut self, repo: &RepoRef, cx: &mut Context<Self>) {
        self.others_tab_mut().toggle_filter_panel(repo);
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

    /// Works out which of others' pull requests the Others tab shows.
    pub(super) fn update_others_shown(&mut self) {
        let lists = std::mem::take(self.lists_mut());
        self.others_tab_mut().update_shown(&lists, unix_now());
        *self.lists_mut() = lists;
    }

    /// Follows a change of the watched repositories: shows the new set at
    /// once (a removed one goes, an added one shows empty) and refreshes,
    /// which fetches the added one's pull requests.
    pub(super) fn on_settings_changed(&mut self, cx: &mut Context<Self>) {
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
