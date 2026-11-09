use gpui::{AnyElement, FontWeight, IntoElement, SharedString, div, prelude::*, px};
use qd_studio::StepWeight;

use super::brief_handlers::BriefHandlers;
use super::brief_rendering::{caption_line, markdown, render_anchor_link};
use super::prepared_step::PreparedStep;
use crate::theme;

/// Size of a step's number badge.
const STEP_BADGE: f32 = 22.;

/// The reading order: a titled list of numbered steps, each with its
/// weight in words, its title, what it does for the change, and a link to
/// where it starts in the diff. Mechanical steps are drawn quieter, so the
/// eye goes to the core ones.
pub fn render_tour(
    widget: usize,
    title: &str,
    steps: &[PreparedStep],
    handlers: &BriefHandlers,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(caption_line(title).font_weight(FontWeight::SEMIBOLD))
        .child(
            div()
                .flex()
                .flex_col()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .overflow_hidden()
                .children(
                    steps
                        .iter()
                        .enumerate()
                        .map(|(index, step)| render_step(widget, index, step, handlers)),
                ),
        )
        .into_any_element()
}

/// One step: number badge, then weight, title, explanation and link.
fn render_step(
    widget: usize,
    index: usize,
    step: &PreparedStep,
    handlers: &BriefHandlers,
) -> AnyElement {
    let is_mechanical = step.weight == StepWeight::Mechanical;
    let (badge_background, badge_text) = match step.weight {
        StepWeight::Core => (theme::accent(), theme::on_accent()),
        StepWeight::Supporting => (theme::hover(), theme::text()),
        StepWeight::Mechanical => (theme::panel(), theme::subtle()),
    };
    div()
        .id(SharedString::from(format!("brief-{widget}-step-{index}")))
        .flex()
        .gap_3()
        .px_3()
        .py_2p5()
        .when(index > 0, |row| {
            row.border_t_1().border_color(theme::border())
        })
        .child(
            div()
                .flex_none()
                .size(px(STEP_BADGE))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(badge_background)
                .text_color(badge_text)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child((index + 1).to_string()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .when(is_mechanical, |text| text.text_color(theme::muted()))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_baseline()
                        .gap_x_2()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(step.title.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::subtle())
                                .child(step.weight.label().to_uppercase()),
                        ),
                )
                .child(markdown(
                    format!("brief-{widget}-step-{index}-text"),
                    &step.explanation,
                ))
                .children(step.anchor.as_ref().map(|anchor| {
                    render_anchor_link(widget, &format!("step-{index}"), anchor, handlers)
                })),
        )
        .into_any_element()
}
