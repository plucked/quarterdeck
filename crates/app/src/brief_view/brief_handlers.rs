use std::rc::Rc;

use gpui::{App, ClickEvent, Context, WeakEntity, Window};
use qd_studio::{CodeAnchor, ReaderFamiliarity};

/// Runs when a quoted anchor is clicked.
pub type AnchorHandler = Rc<dyn Fn(&CodeAnchor, &mut Window, &mut App)>;
/// Runs when something about page `index` is clicked.
pub type WidgetHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;
/// Runs when a reader familiarity is picked.
pub type ReaderHandler = Rc<dyn Fn(ReaderFamiliarity, &mut Window, &mut App)>;
/// Runs when a button is clicked.
pub type ActionHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// What the brief's clickable parts do, given by the view that shows it:
/// the detail opens the diff and runs the agent, the gallery only switches
/// pages. Built once per view; the handlers hold a weak view handle.
#[derive(Clone)]
pub struct BriefHandlers {
    /// An anchor link: show those lines in the diff.
    pub show_anchor: AnchorHandler,
    /// Write brief, Try again and Write again.
    pub write: ActionHandler,
    /// Cancel, while the brief is written.
    pub cancel: ActionHandler,
    /// A page tab or a previous / next button: show page `index`.
    pub show_page: WidgetHandler,
    /// How well do you know this area: remember the answer for the next brief.
    pub choose_reader: ReaderHandler,
    /// Report: open the text field for what is wrong with the brief.
    pub open_report: ActionHandler,
    /// Save report: write the report folder.
    pub save_report: ActionHandler,
    /// Cancel or Done: close the report panel.
    pub close_report: ActionHandler,
    /// Copy path: put the saved report's path on the clipboard.
    pub copy_report_path: ActionHandler,
}

/// An action handler running `run` on the view behind `view`; ignored once
/// the view is closed.
pub fn view_action<V: 'static>(
    view: &WeakEntity<V>,
    run: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> ActionHandler {
    let view = view.clone();
    Rc::new(move |window, cx| {
        view.update(cx, |view, cx| run(view, window, cx)).ok();
    })
}

/// A button's click handler running `handler`.
pub fn on_click(handler: &ActionHandler) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let handler = handler.clone();
    move |_, window, cx| handler(window, cx)
}
