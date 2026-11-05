use gpui::{
    App, Context, FocusHandle, FontWeight, IntoElement, Render, Subscription, Task, Window, div,
    prelude::*, px,
};
use qd_cache::CacheUsage;

use super::file_diff_page::render_file_diff_page;
use super::github_page::render_github_page;
use super::settings_category::SettingsCategory;
use crate::app_commands::CloseWindow;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::theme;

/// Width of the category list.
const CATEGORY_LIST_WIDTH: f32 = 180.;

/// The settings window's view: the categories on the left, the chosen
/// category's settings on the right. Redraws when [`AppSettings`] changes,
/// wherever the change came from.
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
    /// Keeps the observer of [`AppSettings`] registered for the view's lifetime.
    _settings_subscription: Subscription,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so cmd-w reaches it.
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut settings = Self {
            focus,
            category: SettingsCategory::default(),
            has_cleared_cache: false,
            cache_usage: None,
            usage_task: None,
            _settings_subscription: cx.observe_global::<AppSettings>(|_, cx| cx.notify()),
        };
        settings.read_cache_usage(cx);
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

    fn select_category(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
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
            SettingsCategory::FileDiff => {
                render_file_diff_page(AppSettings::diff_view(cx)).into_any_element()
            }
            SettingsCategory::GitHub => {
                let settings = cx.entity().downgrade();
                let clear_cache = move |_: &mut Window, cx: &mut App| {
                    settings.update(cx, |_, cx| Self::clear_cache(cx)).ok();
                };
                render_github_page(self.has_cleared_cache, self.cache_usage, clear_cache, cx)
                    .into_any_element()
            }
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
                div()
                    .flex_1()
                    .min_w_0()
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
