use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, KeyDownEvent, ListOffset,
    ListState, Subscription, Task, Window, px,
};
use qd_claude::{ClaudeSession, SessionInventory, SessionSender};

use super::session_group::SessionGroup;
use super::session_list_state::session_list_state;
use super::session_picker_phase::SessionPickerPhase;
use super::session_picker_row::SessionPickerRow;
use super::session_presentation::SessionPresentation;
use super::session_selection::SessionSelection;
use crate::scrollbar::ListRowTops;
use crate::text_area::{TextArea, TextAreaEvent};

/// A review snapshot and its destination selection, isolated from diff rendering.
pub struct ClaudeSessionPicker {
    pub(super) sessions: Vec<ClaudeSession>,
    pub(super) selection: SessionSelection,
    pub(super) presentation: Vec<SessionPresentation>,
    pub(super) search: Entity<TextArea>,
    pub(super) query: String,
    pub(super) phase: SessionPickerPhase,
    pub(super) note_count: usize,
    pub(super) focus: FocusHandle,
    pub(super) scroll: ListState,
    pub(super) groups: Vec<SessionGroup>,
    pub(super) rows: Vec<SessionPickerRow>,
    pub(super) row_tops: ListRowTops,
    markdown: String,
    task: Option<Task<()>>,
    pub(super) checkout_task: Option<Task<()>>,
    _search_subscription: Subscription,
}

impl ClaudeSessionPicker {
    /// Starts discovery for the current review snapshot, with no chosen target.
    pub fn new(markdown: String, note_count: usize, cx: &mut Context<Self>) -> Self {
        // Reuse the app's text field; search changes never select or submit a session.
        let search = cx.new(|cx| TextArea::single_line("Name, repo, folder or branch…", cx));
        let subscription =
            cx.subscribe(
                &search,
                |picker, search, event: &TextAreaEvent, cx| match event {
                    TextAreaEvent::Changed => {
                        let query = search.read(cx).text().to_owned();
                        picker.filter_sessions(query, cx);
                    }
                    TextAreaEvent::Submit => picker.send_review(cx),
                    TextAreaEvent::Cancel => picker.clear_search(cx),
                },
            );
        let mut picker = Self {
            sessions: Vec::new(),
            selection: SessionSelection::default(),
            presentation: Vec::new(),
            search,
            query: String::new(),
            phase: SessionPickerPhase::Loading,
            note_count,
            focus: cx.focus_handle(),
            scroll: session_list_state(),
            groups: Vec::new(),
            rows: Vec::new(),
            row_tops: ListRowTops::from_heights([]),
            markdown,
            task: None,
            checkout_task: None,
            _search_subscription: subscription,
        };
        picker.refresh_sessions(cx);
        picker
    }

    /// Reads a fresh list off the UI thread. Refresh never sends a message.
    pub(super) fn refresh_sessions(&mut self, cx: &mut Context<Self>) {
        if self.is_sending() {
            return;
        }
        // Clear the old selection before another snapshot arrives.
        self.checkout_task = None;
        self.selection.clear_selection();
        self.phase = SessionPickerPhase::Loading;
        self.task = Some(cx.spawn(async move |picker, cx| {
            let result = cx
                .background_executor()
                .spawn(async { SessionInventory::discover()?.refresh() })
                .await;
            picker
                .update(cx, |picker, cx| {
                    match result {
                        Ok(sessions) => {
                            picker.presentation = SessionPresentation::for_sessions(&sessions);
                            picker.selection.replace(
                                picker
                                    .presentation
                                    .iter()
                                    .map(|row| row.searchable.clone())
                                    .collect(),
                                &picker.query,
                            );
                            picker.sessions = sessions;
                            picker.groups.clear();
                            picker.rebuild_session_rows(cx);
                            picker.phase = SessionPickerPhase::Ready;
                            picker.load_session_checkouts(cx);
                        }
                        Err(error) => {
                            picker.phase = SessionPickerPhase::Failed(format!("{error:#}"));
                        }
                    }
                    picker.task = None;
                    cx.notify();
                })
                .ok();
        }));
        cx.notify();
    }

    pub fn is_sending(&self) -> bool {
        matches!(self.phase, SessionPickerPhase::Sending)
    }

    pub(super) fn can_send(&self) -> bool {
        matches!(self.phase, SessionPickerPhase::Ready) && self.selection.selected().is_some()
    }

    /// Filters the existing snapshot without rereading the filesystem or starting work.
    fn filter_sessions(&mut self, query: String, cx: &mut Context<Self>) {
        if matches!(
            self.phase,
            SessionPickerPhase::Sending | SessionPickerPhase::Submitted(_)
        ) {
            return;
        }
        self.query = query;
        self.selection.filter(&self.query);
        self.rebuild_session_rows(cx);
    }

    /// Rebuilds headings and matches together, keeping the explicit selection in view.
    pub(super) fn rebuild_session_rows(&mut self, cx: &mut Context<Self>) {
        self.rows = SessionPickerRow::for_groups(&self.groups, self.selection.visible());
        self.row_tops = ListRowTops::from_heights(self.rows.iter().map(|row| row.height()));
        self.scroll.reset(self.rows.len());
        self.reveal_selection(cx);
    }

    /// Clearing a search leaves destination choice explicit, including after no matches.
    pub(super) fn clear_search(&mut self, cx: &mut Context<Self>) {
        self.search.update(cx, |search, cx| search.set_text("", cx));
        cx.notify();
    }

    /// Starts a new explicit selection after a successful submission.
    pub(super) fn choose_another_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_sessions(cx);
        self.clear_search(cx);
        self.search.read(cx).focus(window);
    }

    /// A selection is explicit; discovery never silently picks the first row.
    pub(super) fn select_session(&mut self, index: usize, cx: &mut Context<Self>) {
        if !matches!(self.phase, SessionPickerPhase::Ready) || index >= self.sessions.len() {
            return;
        }
        self.selection.select(index);
        cx.notify();
    }

    /// Takes one snapshot into a background write; repeated clicks cannot queue duplicates.
    pub(super) fn send_review(&mut self, cx: &mut Context<Self>) {
        if !self.can_send() {
            return;
        }
        let Some(session) = self
            .selection
            .selected()
            .and_then(|index| self.sessions.get(index))
            .cloned()
        else {
            return;
        };
        let markdown = self.markdown.clone();
        // Disable submission before dispatching work, including keyboard repeats.
        self.phase = SessionPickerPhase::Sending;
        self.task = Some(cx.spawn(async move |picker, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    SessionSender::new(SessionInventory::discover()?).send(&session, &markdown)
                })
                .await;
            // Keep the notes and report only what the socket transport confirms.
            picker
                .update(cx, |picker, cx| {
                    picker.phase = match result {
                        Ok(()) => SessionPickerPhase::Submitted(
                            "Submitted. Check Claude for receipt or approval; replies stay there."
                                .into(),
                        ),
                        Err(error) => SessionPickerPhase::Failed(format!("{error:#}")),
                    };
                    picker.task = None;
                    cx.notify();
                })
                .ok();
        }));
        cx.notify();
    }

    /// Capture plain arrows above the text field, keeping typing focused on search.
    pub(super) fn on_navigation_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(self.phase, SessionPickerPhase::Ready)
            || event.keystroke.modifiers != gpui::Modifiers::default()
        {
            return;
        }
        match event.keystroke.key.as_str() {
            "up" => self.select_previous_session(cx),
            "down" => self.select_next_session(cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// The search field's mapped cursor action moves through results instead.
    pub(super) fn select_next_session(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.phase, SessionPickerPhase::Ready) {
            return;
        }
        self.selection.select_next();
        self.reveal_selection(cx);
    }

    /// Up is bounded by the first matching session, including when search has focus.
    pub(super) fn select_previous_session(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.phase, SessionPickerPhase::Ready) {
            return;
        }
        self.selection.select_previous();
        self.reveal_selection(cx);
    }

    /// Keyboard navigation scrolls only when the chosen row is outside the viewport.
    fn reveal_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(selected) = self.selection.selected()
            && let Some(row) = self
                .rows
                .iter()
                .position(|row| *row == SessionPickerRow::Session(selected))
        {
            // Exact row heights also reveal destinations that have not been measured yet.
            let target = ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            };
            let top = self.row_tops.y_of(target);
            let bottom = top + px(self.rows[row].height());
            let current = self.row_tops.y_of(self.scroll.logical_scroll_top());
            let viewport = self.scroll.viewport_bounds().size.height;
            if top < current || viewport <= px(0.) {
                self.scroll.scroll_to(target);
            } else if bottom > current + viewport {
                self.scroll
                    .scroll_to(self.row_tops.offset_at(bottom - viewport));
            }
        }
        cx.notify();
    }

    /// Enter submits from the list; the search field reports its own submit event.
    pub(super) fn on_list_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.focus.is_focused(window) && event.keystroke.key == "enter" {
            self.send_review(cx);
            cx.stop_propagation();
        }
    }
}

impl Focusable for ClaudeSessionPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.search.focus_handle(cx)
    }
}
