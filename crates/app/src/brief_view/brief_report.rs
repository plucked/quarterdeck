use gpui::{AppContext as _, ClipboardItem, Context, Task, Window};
use qd_studio::{BriefRecord, ShownPage, report_root, write_brief_report};

use super::report_state::ReportState;
use crate::text_area::TextArea;

/// Reporting the shown brief: where it stands and the write in flight.
/// Owned by the view showing the brief (the detail's `BriefPane`, the
/// gallery); the steps are [`ReportHost`]'s, so both views report alike.
#[derive(Default)]
pub struct BriefReport {
    state: ReportState,
    /// The report being written to disk; kept when the panel closes, so a
    /// save in flight finishes.
    task: Option<Task<()>>,
}

impl BriefReport {
    pub fn state(&self) -> &ReportState {
        &self.state
    }

    /// Closes the panel (another brief or page shows).
    pub fn close(&mut self) {
        self.state = ReportState::Closed;
    }
}

/// A view that shows a brief and reports it: it hands out its
/// [`BriefReport`] and says what happens when the panel changes; opening,
/// saving, closing and copying the path are the same for every view.
pub trait ReportHost: Sized + 'static {
    fn report_mut(&mut self) -> &mut BriefReport;

    /// After the report panel changed; the gallery lays its row out again.
    fn report_changed(&mut self) {}

    /// Sets where the report stands and redraws.
    fn set_report_state(&mut self, state: ReportState, cx: &mut Context<Self>) {
        self.report_mut().state = state;
        self.report_changed();
        cx.notify();
    }

    /// Opens the panel with an empty text field showing `placeholder`, focused.
    fn open_report(
        &mut self,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = cx.new(|cx| TextArea::new(placeholder, cx));
        editor.read(cx).focus(window);
        self.set_report_state(ReportState::Writing(editor), cx);
    }

    /// Closes the panel.
    fn close_report(&mut self, cx: &mut Context<Self>) {
        self.set_report_state(ReportState::Closed, cx);
    }

    /// Writes a report on the brief `record` describes with the words in
    /// the panel and the page the reader has open (`shown_page`), on the
    /// background executor (`app_version` names the build); the panel then
    /// shows the saved report's path, or why it failed with the words kept.
    fn save_report(
        &mut self,
        record: BriefRecord,
        shown_page: ShownPage,
        app_version: String,
        cx: &mut Context<Self>,
    ) {
        // The words, and where they go.
        let editor = match self.report_mut().state() {
            ReportState::Writing(editor) | ReportState::Failed { editor, .. } => editor.clone(),
            ReportState::Closed | ReportState::Saving | ReportState::Saved(_) => return,
        };
        let feedback = editor.read(cx).text().trim().to_owned();
        let root = match (feedback.is_empty(), report_root()) {
            (true, _) => Err("Write what is wrong or weird first."),
            (false, None) => Err("There is no Application Support folder to save into."),
            (false, Some(root)) => Ok(root),
        };
        let root = match root {
            Ok(root) => root,
            Err(message) => {
                let message = message.into();
                self.set_report_state(ReportState::Failed { message, editor }, cx);
                return;
            }
        };
        self.set_report_state(ReportState::Saving, cx);

        // Write it off the main thread, then show where it went.
        let task = cx.spawn(async move |view, cx| {
            let written = cx
                .background_executor()
                .spawn(async move {
                    write_brief_report(&root, &record, &feedback, &shown_page, &app_version)
                })
                .await;
            view.update(cx, |view, cx| {
                let state = match written {
                    Ok(path) => ReportState::Saved(path.display().to_string().into()),
                    Err(error) => ReportState::Failed {
                        message: format!("The report could not be saved: {error}").into(),
                        editor,
                    },
                };
                view.set_report_state(state, cx);
            })
            .ok();
        });
        self.report_mut().task = Some(task);
    }

    /// Puts the saved report's path on the clipboard.
    fn copy_report_path(&mut self, cx: &mut Context<Self>) {
        if let ReportState::Saved(path) = self.report_mut().state() {
            cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
        }
    }
}
