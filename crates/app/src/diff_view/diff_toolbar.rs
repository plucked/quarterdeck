use std::rc::Rc;

use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*};
use qc_cache::{DiffLayout, DiffViewSettings, LineWrapping};
use qc_diff::{ChangeHighlight, WhitespaceMode};

use super::DiffSummary;
use crate::segmented_control::SegmentedControl;
use crate::theme;

/// A handler for a new choice of settings.
type SettingsHandler = Rc<dyn Fn(&DiffViewSettings, &mut Window, &mut App)>;
/// A handler for a toolbar button.
type ButtonHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// The bar above the changed files: totals, the view options (layout,
/// highlight, whitespace, wrapping) and collapse/expand all. Each option
/// calls `on_change` with the settings it would select.
#[derive(IntoElement)]
pub struct DiffToolbar {
    settings: DiffViewSettings,
    summary: DiffSummary,
    on_change: SettingsHandler,
    on_collapse_all: ButtonHandler,
    on_expand_all: ButtonHandler,
}

impl DiffToolbar {
    pub fn new(
        settings: DiffViewSettings,
        summary: DiffSummary,
        on_change: impl Fn(&DiffViewSettings, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            settings,
            summary,
            on_change: Rc::new(on_change),
            on_collapse_all: Rc::new(|_, _| {}),
            on_expand_all: Rc::new(|_, _| {}),
        }
    }

    pub fn on_collapse_all(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_collapse_all = Rc::new(handler);
        self
    }

    pub fn on_expand_all(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_expand_all = Rc::new(handler);
        self
    }

    /// One group of exclusive options for one setting: each `(label, value)`
    /// is shown selected when it is `current`, and selects `apply(settings, value)`.
    fn choice<T: Copy + PartialEq + 'static>(
        &self,
        id: &'static str,
        current: T,
        options: &[(&'static str, T)],
        apply: fn(DiffViewSettings, T) -> DiffViewSettings,
    ) -> SegmentedControl {
        options
            .iter()
            .fold(SegmentedControl::new(id), |control, &(label, value)| {
                let on_change = self.on_change.clone();
                let selected = apply(self.settings, value);
                control.option(label, value == current, move |window, cx| {
                    on_change(&selected, window, cx);
                })
            })
    }
}

impl RenderOnce for DiffToolbar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let settings = self.settings;
        let summary = self.summary;

        // One group per setting, in the order a reviewer reaches for them.
        let layout = self.choice(
            "diff-layout",
            settings.layout,
            &[
                ("Unified", DiffLayout::Unified),
                ("Split", DiffLayout::Split),
            ],
            |settings, layout| DiffViewSettings { layout, ..settings },
        );
        let highlight = self.choice(
            "diff-highlight",
            settings.highlight,
            &[
                ("Lines", ChangeHighlight::Lines),
                ("Words", ChangeHighlight::Words),
                ("Chars", ChangeHighlight::Characters),
            ],
            |settings, highlight| DiffViewSettings {
                highlight,
                ..settings
            },
        );
        let whitespace = self.choice(
            "diff-whitespace",
            settings.whitespace,
            &[
                ("Show whitespace", WhitespaceMode::Show),
                ("Hide", WhitespaceMode::Ignore),
            ],
            |settings, whitespace| DiffViewSettings {
                whitespace,
                ..settings
            },
        );
        let wrapping = self.choice(
            "diff-wrapping",
            settings.wrapping,
            &[
                ("Wrap", LineWrapping::Wrap),
                ("Scroll", LineWrapping::Scroll),
            ],
            |settings, wrapping| DiffViewSettings {
                wrapping,
                ..settings
            },
        );

        let collapse_all = self.on_collapse_all;
        let expand_all = self.on_expand_all;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .text_color(theme::text())
            .child(div().flex_1().min_w_0().text_sm().child(format!(
                "{} files · +{} −{} · {}/{} viewed",
                summary.file_count,
                summary.additions,
                summary.deletions,
                summary.viewed_count,
                summary.file_count
            )))
            .child(layout)
            .child(highlight)
            .child(whitespace)
            .child(wrapping)
            .child(
                SegmentedControl::new("diff-folding")
                    .option("Collapse all", false, move |window, cx| {
                        collapse_all(window, cx);
                    })
                    .option("Expand all", false, move |window, cx| {
                        expand_all(window, cx);
                    }),
            )
    }
}
