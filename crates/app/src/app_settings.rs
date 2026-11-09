//! `AppSettings`: the user's preferences, shared by every window.

use gpui::{App, BorrowAppContext, Global};
use qd_cache::{CacheTtl, ColorTheme, DiffViewSettings, HiddenBranchRule};
use qd_github::RepoRef;

use crate::theme;
use crate::ui_state::UiStateRecorder;

/// The user's preferences as a gpui global: the one place views read them
/// from and the settings window changes them. Views that depend on a
/// preference observe this global (`cx.observe_global::<AppSettings>`) and
/// apply changes as they happen. Every change is saved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppSettings {
    diff_view: DiffViewSettings,
    cache_ttl: CacheTtl,
    color_theme: ColorTheme,
    /// The repositories whose other open pull requests the Others tab lists,
    /// in the order they were added.
    watched_repositories: Vec<RepoRef>,
    /// The rules hiding pull requests from every queue tab by branch, in
    /// the order they were added.
    hidden_branch_rules: Vec<HiddenBranchRule>,
}

impl Global for AppSettings {}

impl AppSettings {
    /// The preferences saved before this launch; defaults for anything not
    /// saved or no longer readable. Makes the saved color theme the active
    /// one, so call it before opening a window.
    pub fn load(recorder: &UiStateRecorder) -> Self {
        let settings = Self {
            diff_view: recorder.saved_diff_view_settings().unwrap_or_default(),
            cache_ttl: recorder.saved_cache_ttl().unwrap_or_default(),
            color_theme: recorder.saved_color_theme().unwrap_or_default(),
            watched_repositories: recorder.saved_watched_repositories(),
            hidden_branch_rules: recorder.saved_hidden_branch_rules(),
        };
        theme::set_active_theme(settings.color_theme);
        settings
    }

    /// The color theme every window is drawn in.
    pub fn color_theme(cx: &App) -> ColorTheme {
        cx.global::<Self>().color_theme
    }

    /// Switches every window to `color_theme` at once and saves it.
    pub fn set_color_theme(cx: &mut App, color_theme: ColorTheme) {
        if Self::color_theme(cx) == color_theme {
            return;
        }
        cx.update_global::<Self, _>(|app_settings, _| app_settings.color_theme = color_theme);
        theme::set_active_theme(color_theme);
        UiStateRecorder::record_color_theme(cx, color_theme);
        // Colors are read while drawing, so a redraw shows the new theme.
        cx.refresh_windows();
    }

    /// How file diffs are shown.
    pub fn diff_view(cx: &App) -> DiffViewSettings {
        cx.global::<Self>().diff_view
    }

    /// How long unused cached GitHub data is kept.
    pub fn cache_ttl(cx: &App) -> CacheTtl {
        cx.global::<Self>().cache_ttl
    }

    /// Changes how long unused cached GitHub data is kept and saves it; the
    /// next cleanup (after the next queue refresh) uses it.
    pub fn set_cache_ttl(cx: &mut App, ttl: CacheTtl) {
        if Self::cache_ttl(cx) == ttl {
            return;
        }
        cx.update_global::<Self, _>(|app_settings, _| app_settings.cache_ttl = ttl);
        UiStateRecorder::record_cache_ttl(cx, ttl);
    }

    /// Changes how file diffs are shown and saves it; observers are notified
    /// only when something changed.
    pub fn set_diff_view(cx: &mut App, settings: DiffViewSettings) {
        if Self::diff_view(cx) == settings {
            return;
        }
        cx.update_global::<Self, _>(|app_settings, _| app_settings.diff_view = settings);
        UiStateRecorder::record_diff_view_settings(cx, settings);
    }

    /// The repositories the Others tab lists.
    pub fn watched_repositories(cx: &App) -> &[RepoRef] {
        &cx.global::<Self>().watched_repositories
    }

    /// Changes the repositories the Others tab lists and saves them;
    /// observers (the queue refreshes) are notified only on a change.
    pub fn set_watched_repositories(cx: &mut App, repositories: Vec<RepoRef>) {
        if Self::watched_repositories(cx) == repositories.as_slice() {
            return;
        }
        UiStateRecorder::record_watched_repositories(cx, repositories.clone());
        cx.update_global::<Self, _>(|app_settings, _| {
            app_settings.watched_repositories = repositories;
        });
    }

    /// The rules hiding pull requests from every queue tab by branch.
    pub fn hidden_branch_rules(cx: &App) -> &[HiddenBranchRule] {
        &cx.global::<Self>().hidden_branch_rules
    }

    /// Changes the rules hiding pull requests by branch and saves them;
    /// observers (the queue shows its lists again) are notified only on a change.
    pub fn set_hidden_branch_rules(cx: &mut App, rules: Vec<HiddenBranchRule>) {
        if Self::hidden_branch_rules(cx) == rules.as_slice() {
            return;
        }
        UiStateRecorder::record_hidden_branch_rules(cx, rules.clone());
        cx.update_global::<Self, _>(|app_settings, _| app_settings.hidden_branch_rules = rules);
    }
}
