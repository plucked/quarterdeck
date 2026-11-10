use gpui::{
    Context, FocusHandle, FontWeight, IntoElement, Render, Subscription, Task, Window, div,
    prelude::*, px,
};
use qd_cache::{CacheUsage, ColorTheme};

use super::file_diff_page::render_file_diff_page;
use super::filters_page::BranchRuleForm;
use super::github_sign_in::TokenSignIn;
use super::repositories_page::RepositoryAdder;
use super::settings_category::SettingsCategory;
use super::updates_page::render_updates_page;
use crate::app_commands::CloseWindow;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::app_update::AppUpdater;
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
    /// The GitHub page's token field and sign-in.
    token_sign_in: TokenSignIn,
    /// The Repositories page's field, its check and the quick picks.
    repository_adder: RepositoryAdder,
    /// The Filters page's pattern field, branch side and error.
    branch_rule_form: BranchRuleForm,
    /// Keep the observers of [`AppSettings`] and the updater registered
    /// for the view's lifetime.
    _subscriptions: [Subscription; 2],
}

impl SettingsWindow {
    pub fn new(category: SettingsCategory, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so cmd-w reaches it.
        let focus = cx.focus_handle();
        window.focus(&focus);

        // Every page redraws when a preference or the update status changes.
        let subscriptions = [
            cx.observe_global::<AppSettings>(|_, cx| cx.notify()),
            cx.observe(&AppUpdater::global(cx), |_, _, cx| cx.notify()),
        ];
        let mut settings = Self {
            focus,
            category,
            has_cleared_cache: false,
            cache_usage: None,
            usage_task: None,
            previewed_theme: None,
            token_sign_in: TokenSignIn::new(cx),
            repository_adder: RepositoryAdder::new(cx),
            branch_rule_form: BranchRuleForm::new(cx),
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

    pub(super) fn token_sign_in(&self) -> &TokenSignIn {
        &self.token_sign_in
    }

    pub(super) fn token_sign_in_mut(&mut self) -> &mut TokenSignIn {
        &mut self.token_sign_in
    }

    pub(super) fn repository_adder(&self) -> &RepositoryAdder {
        &self.repository_adder
    }

    pub(super) fn repository_adder_mut(&mut self) -> &mut RepositoryAdder {
        &mut self.repository_adder
    }

    pub(super) fn branch_rule_form(&self) -> &BranchRuleForm {
        &self.branch_rule_form
    }

    pub(super) fn branch_rule_form_mut(&mut self) -> &mut BranchRuleForm {
        &mut self.branch_rule_form
    }

    /// Whether the cache was cleared since the window opened.
    pub(super) fn has_cleared_cache(&self) -> bool {
        self.has_cleared_cache
    }

    /// What the cache holds, once read.
    pub(super) fn cache_usage(&self) -> Option<CacheUsage> {
        self.cache_usage
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
    pub(super) fn clear_cache(cx: &mut Context<Self>) {
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
            SettingsCategory::GitHub => self.render_github_page(cx).into_any_element(),
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
