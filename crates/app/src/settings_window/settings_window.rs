use gpui::{
    Context, FocusHandle, FontWeight, IntoElement, Render, Subscription, Window, div, prelude::*,
    px,
};

use super::file_diff_page::render_file_diff_page;
use super::settings_category::SettingsCategory;
use crate::app_commands::CloseWindow;
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
    /// Keeps the observer of [`AppSettings`] registered for the view's lifetime.
    _settings_subscription: Subscription,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so cmd-w reaches it.
        let focus = cx.focus_handle();
        window.focus(&focus);
        Self {
            focus,
            category: SettingsCategory::default(),
            _settings_subscription: cx.observe_global::<AppSettings>(|_, cx| cx.notify()),
        }
    }

    fn select_category(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
        self.category = category;
        cx.notify();
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
            SettingsCategory::FileDiff => render_file_diff_page(AppSettings::diff_view(cx)),
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
