use gpui::{AnyElement, FontWeight, IntoElement, SharedString, div, prelude::*, px};

use super::brief_handlers::BriefHandlers;
use super::prepared_page::PreparedPage;
use crate::controls::{NumberFlow, SectionLabel};
use crate::theme;

/// Height of the underline marking the shown page's tab.
const TAB_UNDERLINE: f32 = 2.;
/// Room under a tab's label, its underline included.
const TAB_BOTTOM_SPACE: f32 = 6.;

/// The brief's pages as a row of numbered tabs, the shown one underlined;
/// a click shows that page. Wraps onto a second line when narrow.
pub fn render_page_tabs(
    pages: &[PreparedPage],
    shown: usize,
    handlers: &BriefHandlers,
) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .gap_x_1()
        .border_b_1()
        .border_color(theme::border())
        .children(pages.iter().enumerate().map(|(index, page)| {
            let is_shown = index == shown;
            let show_page = handlers.show_page.clone();
            div()
                .id(SharedString::from(format!("brief-page-tab-{index}")))
                .flex()
                .items_baseline()
                .gap_1p5()
                .px_2()
                .pt_1p5()
                .pb(px(TAB_BOTTOM_SPACE - TAB_UNDERLINE))
                .border_b(px(TAB_UNDERLINE))
                .border_color(if is_shown {
                    theme::accent()
                } else {
                    gpui::transparent_black().into()
                })
                .cursor_pointer()
                .hover(|style| style.text_color(theme::text()))
                .text_sm()
                .text_color(if is_shown {
                    theme::text()
                } else {
                    theme::muted()
                })
                .when(is_shown, |tab| tab.font_weight(FontWeight::SEMIBOLD))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(NumberFlow::label((index + 1).to_string())),
                )
                .child(page.title.clone())
                .on_click(move |_, window, cx| show_page(index, window, cx))
        }))
        .into_any_element()
}

/// The top of a page: where it sits in the walk, its title, and the
/// sentence saying what it answers.
pub fn render_page_intro(page: &PreparedPage, index: usize, count: usize) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(SectionLabel::new(format!("Page {} of {count}", index + 1)))
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(page.title.clone()),
        )
        .when(!page.summary.is_empty(), |intro| {
            intro.child(div().text_color(theme::muted()).child(page.summary.clone()))
        })
        .into_any_element()
}

/// The end of a page: buttons to the previous page (left) and the next one
/// (right, with what it answers), so the brief reads as a sequence. Each
/// fills its half of the row: a card whose width came from its text would
/// be laid out for the summary on one line, and the wrapped summary would
/// spill out of its bottom.
pub fn render_page_navigation(
    pages: &[PreparedPage],
    shown: usize,
    handlers: &BriefHandlers,
) -> AnyElement {
    let previous = shown
        .checked_sub(1)
        .and_then(|index| Some((index, pages.get(index)?)));
    let next = pages.get(shown + 1).map(|page| (shown + 1, page));
    let half = || div().flex_1().min_w_0();
    div()
        .flex()
        .gap_3()
        .pt_4()
        .border_t_1()
        .border_color(theme::border())
        .child(half().children(
            previous.map(|(index, page)| {
                navigation_card(index, page, NavigationStep::Previous, handlers)
            }),
        ))
        .child(half().children(
            next.map(|(index, page)| navigation_card(index, page, NavigationStep::Next, handlers)),
        ))
        .into_any_element()
}

/// Which neighbour a navigation button leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NavigationStep {
    Previous,
    Next,
}

/// A button leading to page `index`: the direction, the page's title and,
/// for the next page, what it answers.
fn navigation_card(
    index: usize,
    page: &PreparedPage,
    step: NavigationStep,
    handlers: &BriefHandlers,
) -> AnyElement {
    let show_page = handlers.show_page.clone();
    let (direction, is_next) = match step {
        NavigationStep::Previous => ("← Previous", false),
        NavigationStep::Next => ("Next →", true),
    };
    div()
        .id(SharedString::from(format!("brief-page-step-{index}")))
        .flex()
        .flex_col()
        .gap_0p5()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme::border_strong())
        .cursor_pointer()
        .hover(|style| style.border_color(theme::accent()).bg(theme::hover()))
        // Its lines keep the card's width (stretched, not sized to their text),
        // so a long summary wraps inside the border; the next card's are right-aligned.
        .when(is_next, Styled::text_right)
        .child(div().text_xs().text_color(theme::subtle()).child(direction))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(NumberFlow::label(format!("{}  {}", index + 1, page.title))),
        )
        .when(is_next && !page.summary.is_empty(), |card| {
            card.child(
                div()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(page.summary.clone()),
            )
        })
        .on_click(move |_, window, cx| show_page(index, window, cx))
        .into_any_element()
}
