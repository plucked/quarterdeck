//! `Checkbox`: a small drawn checkbox (checked, unchecked or mixed) for
//! viewed marks and options.

use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::theme;

/// Edge length of the box.
const BOX_SIZE: f32 = 14.;
/// Corner radius of the box.
const BOX_CORNER_RADIUS: f32 = 3.;
/// Text size of the tick or dash in the box.
const MARK_TEXT_SIZE: f32 = 10.;

/// Whether a checkbox is ticked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Unchecked,
    Checked,
    /// Some of what it stands for is ticked (a folder).
    Mixed,
}

impl CheckState {
    /// Checked when `is_checked`, else unchecked.
    pub fn from_bool(is_checked: bool) -> Self {
        if is_checked {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }
}

/// A drawn checkbox; its owner wraps it in whatever takes the click.
#[derive(IntoElement)]
pub struct Checkbox {
    state: CheckState,
}

impl Checkbox {
    pub fn new(state: CheckState) -> Self {
        Self { state }
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mark = match self.state {
            CheckState::Unchecked => None,
            CheckState::Checked => Some("✓"),
            CheckState::Mixed => Some("−"),
        };
        div()
            .flex_none()
            .size(px(BOX_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(BOX_CORNER_RADIUS))
            .border_1()
            .text_size(px(MARK_TEXT_SIZE))
            .line_height(px(BOX_SIZE))
            .when(mark.is_some(), |checkbox| {
                checkbox
                    .bg(theme::accent())
                    .border_color(theme::accent())
                    .text_color(theme::on_accent())
            })
            .when(mark.is_none(), |checkbox| {
                checkbox.border_color(theme::subtle())
            })
            .children(mark)
    }
}
