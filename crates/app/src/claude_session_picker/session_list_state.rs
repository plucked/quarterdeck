use gpui::{ListAlignment, ListState, px};

const SESSION_LIST_OVERDRAW: f32 = 216.;

/// Measures a few rows beyond the viewport so wheel scrolling can advance into them.
pub(super) fn session_list_state() -> ListState {
    // gpui clamps wheel events to measured height, even when the scrollbar knows more.
    ListState::new(0, ListAlignment::Top, px(SESSION_LIST_OVERDRAW))
}

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext as _, Context, IntoElement, Render, ScrollDelta, ScrollWheelEvent, Styled,
        TestAppContext, Window, div, list, point, size,
    };

    use super::*;
    use crate::claude_session_picker::session_picker_row::SessionPickerRow;
    use crate::scrollbar::ListRowTops;

    const VIEWPORT_HEIGHT: f32 = 378.;
    const WHEEL_STEP: f32 = 12.;

    /// The actual list configuration and row heights, with no discovery, network or receivers.
    struct SessionScrollFixture {
        state: ListState,
        rows: Vec<SessionPickerRow>,
    }

    impl Render for SessionScrollFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            list(
                self.state.clone(),
                cx.processor(|fixture, index: usize, _, _| {
                    div()
                        .h(px(fixture.rows[index].height()))
                        .w_full()
                        .into_any_element()
                }),
            )
            .w_full()
            .h_full()
        }
    }

    /// Repeated wheel steps must traverse beyond the initially painted rows and return to the top.
    #[gpui::test]
    fn wheel_scrolls_through_all_repository_groups(cx: &mut TestAppContext) {
        // Use several mixed-height groups, with enough content to fill multiple viewports.
        let cx = cx.add_empty_window();
        let rows: Vec<_> = (0..5)
            .flat_map(|group| {
                std::iter::once(SessionPickerRow::Repository { group, count: 3 })
                    .chain((group * 3..group * 3 + 3).map(SessionPickerRow::Session))
            })
            .collect();
        let tops = ListRowTops::from_heights(rows.iter().map(|row| row.height()));
        let state = session_list_state();
        state.reset(rows.len());
        let view = cx.new(|_| SessionScrollFixture {
            state: state.clone(),
            rows,
        });

        // Repaint between events, as the native list does while a mouse wheel is turning.
        for _ in 0..100 {
            cx.draw(
                point(px(0.), px(0.)),
                size(px(300.), px(VIEWPORT_HEIGHT)),
                |_, _| view.clone(),
            );
            cx.simulate_event(ScrollWheelEvent {
                position: point(px(20.), px(20.)),
                delta: ScrollDelta::Pixels(point(px(0.), px(-WHEEL_STEP))),
                ..Default::default()
            });
        }
        assert_eq!(
            tops.y_of(state.logical_scroll_top()),
            tops.total() - px(VIEWPORT_HEIGHT)
        );

        // Mouse wheels that report line deltas must also move back through the entire list.
        for _ in 0..100 {
            cx.draw(
                point(px(0.), px(0.)),
                size(px(300.), px(VIEWPORT_HEIGHT)),
                |_, _| view.clone(),
            );
            cx.simulate_event(ScrollWheelEvent {
                position: point(px(20.), px(20.)),
                delta: ScrollDelta::Lines(point(0., 1.)),
                ..Default::default()
            });
        }
        assert_eq!(tops.y_of(state.logical_scroll_top()), px(0.));
    }
}
