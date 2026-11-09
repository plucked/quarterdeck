use gpui::{
    App, Context, Entity, FocusHandle, FontWeight, IntoElement, Render, SharedString, Subscription,
    Task, Window, div, prelude::*, px,
};
use qd_cache::{BranchSide, CacheUsage, ColorTheme};

use super::add_repository_status::AddRepositoryStatus;
use super::file_diff_page::render_file_diff_page;
use super::github_page::render_github_page;
use super::settings_category::SettingsCategory;
use super::sign_in_status::SignInStatus;
use super::updates_page::render_updates_page;
use crate::app_commands::CloseWindow;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::app_update::AppUpdater;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::theme;

/// Width of the category list.
const CATEGORY_LIST_WIDTH: f32 = 180.;

/// The settings window's view: the categories on the left, the chosen
/// category's settings on the right. Redraws when [`AppSettings`] or the
/// [`AppUpdater`]'s status changes, wherever the change came from.
pub struct SettingsWindow {
    focus: FocusHandle,
    category: SettingsCategory,
    /// Whether the cache was cleared since the window opened, so the button says so.
    has_cleared_cache: bool,
    /// What the cache holds, read when the window opens and after clearing;
    /// `None` until read (or when it cannot be).
    cache_usage: Option<CacheUsage>,
    /// The read of `cache_usage` in flight.
    usage_task: Option<Task<()>>,
    /// The theme card under the pointer, previewed instead of the active theme.
    previewed_theme: Option<ColorTheme>,
    /// Where a GitHub token is pasted to sign in.
    pub(super) token_input: Entity<TextArea>,
    pub(super) sign_in_status: SignInStatus,
    /// The token check or sign-out in flight.
    pub(super) sign_in_task: Option<Task<()>>,
    /// Where a repository to watch is typed.
    pub(super) repository_input: Entity<TextArea>,
    pub(super) add_repository_status: AddRepositoryStatus,
    /// The check of a typed repository in flight.
    pub(super) add_repository_task: Option<Task<()>>,
    /// The repositories of the queue's own lists, offered as quick picks.
    pub(super) queue_repositories: Vec<qd_github::RepoRef>,
    /// The read of `queue_repositories` in flight.
    pub(super) queue_repositories_task: Option<Task<()>>,
    /// Where a branch pattern to hide is typed.
    pub(super) branch_pattern_input: Entity<TextArea>,
    /// Which branch the next added pattern looks at.
    pub(super) branch_rule_side: BranchSide,
    /// Why the typed pattern could not be added, when it could not.
    pub(super) branch_rule_error: Option<SharedString>,
    /// Keep the observers of [`AppSettings`], the updater and the text
    /// fields registered for the view's lifetime.
    _subscriptions: [Subscription; 5],
}

impl SettingsWindow {
    pub fn new(category: SettingsCategory, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so cmd-w reaches it.
        let focus = cx.focus_handle();
        window.focus(&focus);

        // cmd-enter in the token field signs in, like the button.
        let token_input = cx.new(|cx| TextArea::new("ghp_…", cx));
        let repository_input = cx.new(|cx| TextArea::new("owner/name or a GitHub link", cx));
        let branch_pattern_input = cx.new(|cx| TextArea::new("feat/something*", cx));
        let subscriptions = [
            cx.observe_global::<AppSettings>(|_, cx| cx.notify()),
            cx.observe(&AppUpdater::global(cx), |_, _, cx| cx.notify()),
            cx.subscribe(&token_input, |settings, _, event: &TextAreaEvent, cx| {
                if *event == TextAreaEvent::Submit {
                    settings.sign_in(cx);
                }
            }),
            // The field is one line: return (or cmd-enter) adds the repository.
            cx.subscribe(
                &repository_input,
                |settings, input, event: &TextAreaEvent, cx| {
                    if is_one_line_submit(&input, *event, cx) {
                        settings.add_typed_repository(cx);
                    }
                },
            ),
            // The same for the branch pattern field: return adds the pattern.
            cx.subscribe(
                &branch_pattern_input,
                |settings, input, event: &TextAreaEvent, cx| {
                    if is_one_line_submit(&input, *event, cx) {
                        settings.add_typed_branch_rule(cx);
                    }
                },
            ),
        ];
        let mut settings = Self {
            focus,
            category,
            has_cleared_cache: false,
            cache_usage: None,
            usage_task: None,
            previewed_theme: None,
            token_input,
            sign_in_status: SignInStatus::default(),
            sign_in_task: None,
            repository_input,
            add_repository_status: AddRepositoryStatus::default(),
            add_repository_task: None,
            queue_repositories: Vec::new(),
            queue_repositories_task: None,
            branch_pattern_input,
            branch_rule_side: BranchSide::default(),
            branch_rule_error: None,
            _subscriptions: subscriptions,
        };
        settings.read_cache_usage(cx);
        settings.read_queue_repositories(cx);
        settings
    }

    /// Reads what the cache holds off the main thread (it sums every cached
    /// patch), then shows it.
    fn read_cache_usage(&mut self, cx: &mut Context<Self>) {
        let cache = cx.global::<AppServices>().cache.clone();
        self.usage_task = Some(cx.spawn(async move |settings, cx| {
            let usage = cx
                .background_executor()
                .spawn(async move { cache.lock().usage() })
                .await;
            settings
                .update(cx, |settings, cx| {
                    settings.cache_usage = usage
                        .inspect_err(|error| eprintln!("cannot read the cache's size: {error:#}"))
                        .ok();
                    cx.notify();
                })
                .ok();
        }));
    }

    /// The theme to preview in place of the active one, while its card is hovered.
    pub(super) fn previewed_theme(&self) -> Option<ColorTheme> {
        self.previewed_theme
    }

    /// Previews `color_theme` while its card is hovered. Leaving a card
    /// clears the preview only if no other card took it over meanwhile.
    pub(super) fn preview_theme(
        &mut self,
        color_theme: ColorTheme,
        is_hovered: bool,
        cx: &mut Context<Self>,
    ) {
        if is_hovered {
            self.previewed_theme = Some(color_theme);
        } else if self.previewed_theme == Some(color_theme) {
            self.previewed_theme = None;
        }
        cx.notify();
    }

    pub(super) fn select_category(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
        self.category = category;
        cx.notify();
    }

    /// Clears the cache off the main thread, then shows that it did.
    fn clear_cache(cx: &mut Context<Self>) {
        let cache = cx.global::<AppServices>().cache.clone();
        cx.spawn(async move |settings, cx| {
            let cleared = cx
                .background_executor()
                .spawn(async move { cache.lock().clear() })
                .await;
            if let Err(error) = &cleared {
                eprintln!("cannot clear the cache: {error:#}");
            }
            settings
                .update(cx, |settings, cx| {
                    settings.has_cleared_cache = cleared.is_ok();
                    settings.read_cache_usage(cx);
                    cx.notify();
                })
                .ok();
        })
        // Finishes on its own; the window closing meanwhile only skips the label.
        .detach();
    }

    /// The list of categories, the current one highlighted.
    fn render_category_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .flex_none()
            .w(px(CATEGORY_LIST_WIDTH))
            .h_full()
            .p_2()
            .bg(theme::panel())
            .border_r_1()
            .border_color(theme::border())
            .children(
                SettingsCategory::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, category)| {
                        let is_selected = category == self.category;
                        div()
                            .id(index)
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_selected, |item| item.bg(theme::selected()))
                            .when(!is_selected, |item| {
                                item.hover(|style| style.bg(theme::hover()))
                            })
                            .on_click(cx.listener(move |settings, _, _, cx| {
                                settings.select_category(category, cx);
                            }))
                            .child(category.title())
                    }),
            )
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = match self.category {
            SettingsCategory::Appearance => self.render_appearance_page(cx).into_any_element(),
            SettingsCategory::FileDiff => {
                render_file_diff_page(AppSettings::diff_view(cx)).into_any_element()
            }
            SettingsCategory::GitHub => {
                let settings = cx.entity().downgrade();
                let clear_cache = move |_: &mut Window, cx: &mut App| {
                    settings.update(cx, |_, cx| Self::clear_cache(cx)).ok();
                };
                let account_rows = self.render_account_rows(cx);
                render_github_page(
                    account_rows,
                    self.has_cleared_cache,
                    self.cache_usage,
                    clear_cache,
                    cx,
                )
                .into_any_element()
            }
            SettingsCategory::Repositories => self.render_repositories_page(cx),
            SettingsCategory::Filters => self.render_filters_page(cx),
            SettingsCategory::Updates => render_updates_page(cx).into_any_element(),
        };
        div()
            .track_focus(&self.focus)
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .flex()
            .size_full()
            .bg(theme::background())
            .text_color(theme::text())
            .text_sm()
            .child(self.render_category_list(cx))
            .child(
                // The page scrolls when it is taller than the window.
                div()
                    .id("settings-page")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .p_6()
                    .child(
                        div()
                            .pb_2()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.category.title()),
                    )
                    .child(page),
            )
    }
}

/// Whether `event` of the one-line field `input` submits it: return or
/// cmd-enter. A newline typed into it is taken out and counts as the submit.
fn is_one_line_submit(
    input: &Entity<TextArea>,
    event: TextAreaEvent,
    cx: &mut Context<SettingsWindow>,
) -> bool {
    let text = input.read(cx).text().to_owned();
    let has_newline = text.contains('\n');
    if has_newline {
        input.update(cx, |input, cx| input.set_text(text.replace('\n', ""), cx));
    }
    event == TextAreaEvent::Submit || has_newline
}
