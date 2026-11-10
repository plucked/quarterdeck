//! `view_option`: the handler of one option of a choice (a
//! `SegmentedControl`'s), passing its value to a method of the view.

use gpui::{App, Context, WeakEntity, Window};

/// A select handler that calls `pick(view, value, cx)` on the view behind
/// `view` when the option for `value` is chosen; ignored once the view is
/// closed. `pick` is usually the view's setter
/// (`PullRequestDetail::show_side_panel_tab`).
pub fn view_option<V: 'static, Value: Copy + 'static>(
    view: &WeakEntity<V>,
    value: Value,
    pick: fn(&mut V, Value, &mut Context<V>),
) -> impl Fn(&mut Window, &mut App) + 'static {
    let view = view.clone();
    move |_, cx| {
        view.update(cx, |view, cx| pick(view, value, cx)).ok();
    }
}
