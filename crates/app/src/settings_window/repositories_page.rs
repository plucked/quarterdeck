use gpui::{AnyElement, Context, FontWeight, IntoElement, div, prelude::*, px};
use qd_cache::PullRequestListKey;
use qd_github::{GitHubError, RepoRef};

use super::add_repository_status::AddRepositoryStatus;
use super::setting_row::SettingRow;
use super::settings_window::SettingsWindow;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::button::Button;
use crate::repo_monogram::RepoMonogram;
use crate::theme;

/// Width of the field a repository is typed into.
const REPOSITORY_FIELD_WIDTH: f32 = 280.;
/// GitHub's answer for a repository that does not exist or that the token cannot see.
const NOT_FOUND: u16 = 404;

impl SettingsWindow {
    /// The Repositories page: the repositories whose other open pull
    /// requests the Others tab lists, a field to add one (checked with
    /// GitHub), quick picks from the repositories already in the queue,
    /// and the list with a Remove each.
    pub(super) fn render_repositories_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let watched = AppSettings::watched_repositories(cx).to_vec();
        let suggestions: Vec<RepoRef> = self
            .queue_repositories
            .iter()
            .filter(|repo| !watched.contains(repo))
            .cloned()
            .collect();
        let is_checking = self.add_repository_status == AddRepositoryStatus::Checking;

        div()
            .flex()
            .flex_col()
            .child(SettingRow::new(
                "Add a repository",
                "Open pull requests by others in these repositories show in the Others tab (⌘3), each repository with its own filter there. Type owner/name or paste a GitHub link.",
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .w(px(REPOSITORY_FIELD_WIDTH))
                    .child(self.repository_input.clone())
                    .child(
                        div().flex().child(
                            Button::new("add-repository", if is_checking { "Checking…" } else { "Add" })
                                .primary()
                                .disabled(is_checking)
                                .on_click(cx.listener(|settings, _, _, cx| settings.add_typed_repository(cx))),
                        ),
                    )
                    .children(match &self.add_repository_status {
                        AddRepositoryStatus::Failed(reason) => Some(
                            div()
                                .text_xs()
                                .text_color(theme::danger())
                                .child(reason.clone()),
                        ),
                        AddRepositoryStatus::Idle | AddRepositoryStatus::Checking => None,
                    }),
            ))
            .children((!suggestions.is_empty()).then(|| {
                SettingRow::new(
                    "From your queue",
                    "Repositories of your pull requests, review requests and reviews. Click to add one.",
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_end()
                        .gap_1()
                        .max_w(px(REPOSITORY_FIELD_WIDTH))
                        .children(suggestions.into_iter().enumerate().map(|(index, repo)| {
                            let label = repo.full_name();
                            div()
                                .id(("repository-suggestion", index))
                                .px_2()
                                .py_0p5()
                                .rounded_full()
                                .border_1()
                                .border_color(theme::border())
                                .bg(theme::surface())
                                .text_xs()
                                .text_color(theme::muted())
                                .cursor_pointer()
                                .hover(|style| style.border_color(theme::border_strong()).text_color(theme::text()))
                                .on_click(cx.listener(move |_, _, _, cx| add_watched_repository(repo.clone(), cx)))
                                .child(format!("+ {label}"))
                        })),
                )
            }))
            .child(
                div()
                    .pt_4()
                    .pb_1()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::subtle())
                    .child(format!("WATCHED · {}", watched.len())),
            )
            .child(render_watched_list(&watched, cx))
            .into_any_element()
    }

    /// Adds the repository typed into the field: read, checked with GitHub
    /// (one request, which also gives its current spelling), then saved.
    pub(super) fn add_typed_repository(&mut self, cx: &mut Context<Self>) {
        // Read what was typed; refuse what is not a repository or is listed already.
        let text = self.repository_input.read(cx).text().to_owned();
        let Some(repo) = RepoRef::parse_input(&text) else {
            self.fail_to_add(
                "Type owner/name, like zed-industries/zed, or paste a GitHub link.",
                cx,
            );
            return;
        };
        if AppSettings::watched_repositories(cx).contains(&repo) {
            self.fail_to_add("That repository is in the list already.", cx);
            return;
        }
        let Ok(client) = cx.global::<AppServices>().github.clone() else {
            self.fail_to_add("Sign in to GitHub first (Settings → GitHub).", cx);
            return;
        };

        // Check it off the main thread, then add it as GitHub spells it.
        self.add_repository_status = AddRepositoryStatus::Checking;
        cx.notify();
        self.add_repository_task = Some(cx.spawn(async move |settings, cx| {
            let checked = cx
                .background_executor()
                .spawn(async move { client.repository(&repo) })
                .await;
            settings
                .update(cx, |settings, cx| match checked {
                    Ok(repo) => {
                        settings.add_repository_status = AddRepositoryStatus::Idle;
                        settings
                            .repository_input
                            .update(cx, |input, cx| input.set_text("", cx));
                        add_watched_repository(repo, cx);
                        cx.notify();
                    }
                    Err(error) => settings.fail_to_add(&describe_check_error(&error), cx),
                })
                .ok();
        }));
    }

    /// Reads the repositories of the viewer's own, review-requested and
    /// reviewed pull requests from the cache, off the main thread, as quick picks.
    pub(super) fn read_queue_repositories(&mut self, cx: &mut Context<Self>) {
        let cache = cx.global::<AppServices>().cache.clone();
        self.queue_repositories_task = Some(cx.spawn(async move |settings, cx| {
            let repositories = cx
                .background_executor()
                .spawn(async move {
                    let cache = cache.lock();
                    let mut repositories: Vec<RepoRef> = Vec::new();
                    for key in [
                        PullRequestListKey::REVIEW_REQUESTED,
                        PullRequestListKey::REVIEWED,
                        PullRequestListKey::AUTHORED,
                    ] {
                        for pull_request in cache.list(key)? {
                            if !repositories.contains(&pull_request.repo) {
                                repositories.push(pull_request.repo);
                            }
                        }
                    }
                    anyhow::Ok(repositories)
                })
                .await;
            settings
                .update(cx, |settings, cx| {
                    settings.queue_repositories = repositories
                        .inspect_err(|error| {
                            eprintln!("cannot read the queue's repositories: {error:#}");
                        })
                        .unwrap_or_default();
                    cx.notify();
                })
                .ok();
        }));
    }

    fn fail_to_add(&mut self, reason: &str, cx: &mut Context<Self>) {
        self.add_repository_status = AddRepositoryStatus::Failed(reason.to_owned().into());
        cx.notify();
    }
}

/// The watched repositories, one row each with its monogram and a Remove.
fn render_watched_list(watched: &[RepoRef], cx: &mut Context<SettingsWindow>) -> AnyElement {
    if watched.is_empty() {
        return div()
            .py_3()
            .text_xs()
            .text_color(theme::muted())
            .child("None yet. The Others tab stays empty until you add a repository.")
            .into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .children(watched.iter().enumerate().map(|(index, repo)| {
            let removed = repo.clone();
            div()
                .flex()
                .items_center()
                .gap_2()
                .py_1p5()
                .border_b_1()
                .border_color(theme::border())
                .child(RepoMonogram::new(repo.name.clone()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_baseline()
                        .gap_1p5()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(repo.name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::subtle())
                                .child(repo.owner.clone()),
                        ),
                )
                .child(
                    Button::new(("remove-repository", index), "Remove")
                        .ghost()
                        .on_click(
                            cx.listener(move |_, _, _, cx| remove_watched_repository(&removed, cx)),
                        ),
                )
        }))
        .into_any_element()
}

/// Appends `repo` to the watched repositories (once).
fn add_watched_repository(repo: RepoRef, cx: &mut gpui::App) {
    let mut watched = AppSettings::watched_repositories(cx).to_vec();
    if !watched.contains(&repo) {
        watched.push(repo);
        AppSettings::set_watched_repositories(cx, watched);
    }
}

/// Removes `repo` from the watched repositories.
fn remove_watched_repository(repo: &RepoRef, cx: &mut gpui::App) {
    let watched: Vec<RepoRef> = AppSettings::watched_repositories(cx)
        .iter()
        .filter(|watched| *watched != repo)
        .cloned()
        .collect();
    AppSettings::set_watched_repositories(cx, watched);
}

/// Why a typed repository could not be added, in words for the page.
fn describe_check_error(error: &GitHubError) -> String {
    match error {
        GitHubError::Api {
            status: NOT_FOUND, ..
        } => "No such repository, or your token cannot see it.".to_owned(),
        _ => error.to_string(),
    }
}
