use gpui::{
    AnyElement, AvailableSpace, ContentMask, Context, IntoElement, MouseButton, canvas, div, point,
    prelude::*, px, size,
};

use super::pull_request_detail::PullRequestDetail;

impl PullRequestDetail {
    /// Reuses the top file's header after it starts scrolling out of view.
    /// The next file's header pushes it away, without changing list layout
    /// or scroll anchors. Place after the list so prepaint sees this frame's
    /// measured rows and final scroll position, including bottom clamping.
    pub(super) fn render_sticky_file_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let detail = cx.entity().downgrade();
        let list = self.list().clone();
        div()
            // Keep the overlay's control IDs separate from the in-list header.
            .id("sticky-file-header")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let top = list.logical_scroll_top();
                        let (header, next_row) = detail
                            .update(cx, |detail, cx| {
                                let file = detail.rows().file_at_row(top.item_ix)?;
                                let range = detail.rows().file_range(file)?;
                                if top.item_ix == range.start && top.offset_in_item <= px(0.) {
                                    return None;
                                }
                                Some((detail.render_file_header_row(file, cx), range.end))
                            })
                            .ok()??;

                        let mut header = div()
                            .px_4()
                            // Controls must not select or click code underneath;
                            // wheel gestures still reach the virtualized list.
                            .block_mouse_except_scroll()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(header)
                            .into_any_element();
                        let header_size = header.layout_as_root(
                            size(
                                AvailableSpace::Definite(bounds.size.width),
                                AvailableSpace::MinContent,
                            ),
                            window,
                            cx,
                        );
                        let offset = list.bounds_for_item(next_row).map_or(px(0.), |next| {
                            (next.top() - bounds.top() - header_size.height).min(px(0.))
                        });
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            header.prepaint_at(bounds.origin + point(px(0.), offset), window, cx);
                        });
                        Some(header)
                    },
                    |bounds, header: Option<AnyElement>, window, cx| {
                        if let Some(mut header) = header {
                            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                                header.paint(window, cx);
                            });
                        }
                    },
                )
                .size_full(),
            )
    }
}
