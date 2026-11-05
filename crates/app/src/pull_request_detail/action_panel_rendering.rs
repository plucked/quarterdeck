use gpui::{AnyElement, Context, FontWeight, IntoElement, Rgba, SharedString, div, prelude::*};
use qc_github::{MergeOptions, MergeQueueState, PullRequestState};

use super::action_panel::{ActionPanel, ActionProgress, ArmedAction, PanelAction};
use super::pull_request_detail::PullRequestDetail;
use crate::button::Button;
use crate::segmented_control::SegmentedControl;
use crate::theme;

impl PullRequestDetail {
    /// `Merge…` and `Close…` in the header, opening their panel; `None`
    /// without a GitHub connection.
    pub(super) fn render_action_buttons(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.services().github.as_ref().ok()?;
        let open = self.action_panel().map(|panel| panel.action);
        Some(
            div()
                .flex_none()
                .flex()
                .gap_1()
                .child(
                    with_open_mark(
                        Button::new("open-merge", "Merge…"),
                        open == Some(PanelAction::Merge),
                    )
                    .on_click(cx.listener(|detail, _, _, cx| {
                        detail.toggle_action_panel(PanelAction::Merge, cx);
                    })),
                )
                .child(
                    with_open_mark(
                        Button::new("open-close", "Close…"),
                        open == Some(PanelAction::Close),
                    )
                    .on_click(cx.listener(|detail, _, _, cx| {
                        detail.toggle_action_panel(PanelAction::Close, cx);
                    })),
                )
                .into_any_element(),
        )
    }

    /// The open merge or close panel under the header.
    pub(super) fn render_action_panel(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = self.action_panel()?;
        let body = match (&panel.options, &panel.options_error) {
            (_, Some(error)) => render_line(
                format!("GitHub's merge options could not be read: {error}"),
                theme::error(),
            ),
            (None, None) => render_line("Reading the merge options from GitHub…", theme::muted()),
            (Some(options), None) => match options.state {
                PullRequestState::Merged => render_line("Already merged.", theme::added()),
                PullRequestState::Closed => render_line("Already closed.", theme::muted()),
                PullRequestState::Open => match panel.action {
                    PanelAction::Merge => self.render_merge_body(panel, options, cx),
                    PanelAction::Close => self.render_close_body(panel, options, cx),
                },
            },
        };
        Some(
            div()
                .mt_1()
                .ml_5()
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .bg(theme::background())
                .text_sm()
                .child(body)
                .children(render_progress(&panel.progress))
                .into_any_element(),
        )
    }

    /// The merge panel: readiness, queue and stack notes, the method, the
    /// branch, and merge (or admin merge) with its confirmation.
    fn render_merge_body(
        &self,
        panel: &ActionPanel,
        options: &MergeOptions,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let readiness = options.readiness;
        let readiness_color = if readiness.allows_merge() {
            theme::added()
        } else {
            theme::removed()
        };
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(render_line(readiness.description(), readiness_color));

        // What a merge does here: the queue, and the layers below in a stack.
        match options.merge_queue {
            MergeQueueState::Available => {
                body = body.child(render_line(
                    "The base branch uses a merge queue: merging adds it to the queue.",
                    theme::muted(),
                ));
            }
            MergeQueueState::Queued => {
                body = body.child(render_line("Already in the merge queue.", theme::muted()));
            }
            MergeQueueState::NoQueue => {}
        }
        if let Some(note) = self.stack_merge_note(&options.base_ref) {
            body = body.child(render_line(note, theme::accent()));
        }

        // The method: a choice when the repository allows several.
        let method = panel.method;
        body = match options.allowed_methods.as_slice() {
            [] => body.child(render_line(
                "The repository allows no merge method.",
                theme::error(),
            )),
            [only] => body.child(render_line(
                format!("Method: {}", only.label()),
                theme::muted(),
            )),
            methods => {
                let mut control = SegmentedControl::new("merge-method");
                for choice in methods {
                    let choice = *choice;
                    let detail = cx.entity().downgrade();
                    control =
                        control.option(choice.label(), method == Some(choice), move |_, cx| {
                            detail
                                .update(cx, |detail, cx| detail.set_merge_method(choice, cx))
                                .ok();
                        });
                }
                body.child(control)
            }
        };
        body = body.child(self.render_branch_choice(panel, options, cx));

        body.child(self.render_merge_buttons(panel, options, cx))
            .into_any_element()
    }

    /// Merge (and admin merge when allowed) and cancel, or the confirmation
    /// of the one clicked.
    fn render_merge_buttons(
        &self,
        panel: &ActionPanel,
        options: &MergeOptions,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let readiness = options.readiness;
        let method = panel.method;
        let can_merge = readiness.allows_merge() && method.is_some() && !panel.is_working();
        let can_admin_merge = options.permissions.can_merge_as_admin
            && !readiness.allows_merge()
            && method.is_some()
            && !panel.is_working();
        let what = format!(
            "{} #{} into {}",
            method.map_or("Merge", |method| method.label()),
            self.pull_request()
                .map_or(0, |pull_request| pull_request.number),
            options.base_ref
        );
        match panel.armed {
            Some(armed @ (ArmedAction::Merge | ArmedAction::AdminMerge)) => {
                let question = match armed {
                    ArmedAction::AdminMerge => {
                        format!("{what}, bypassing the unmet requirements? This cannot be undone.")
                    }
                    ArmedAction::Merge | ArmedAction::Close => {
                        format!("{what}? This cannot be undone.")
                    }
                };
                render_confirmation(question, "Confirm merge", cx)
            }
            Some(ArmedAction::Close) | None => div()
                .flex()
                .gap_2()
                .justify_end()
                .child(Button::new("merge-cancel", "Cancel").on_click(cx.listener(
                    |detail, _, _, cx| {
                        detail.toggle_action_panel(PanelAction::Merge, cx);
                    },
                )))
                .when(
                    options.permissions.can_merge_as_admin && !readiness.allows_merge(),
                    |row| {
                        row.child(
                            Button::new("admin-merge", "Merge as admin")
                                .danger()
                                .disabled(!can_admin_merge)
                                .on_click(cx.listener(|detail, _, _, cx| {
                                    detail.arm_action(Some(ArmedAction::AdminMerge), cx);
                                })),
                        )
                    },
                )
                .child(
                    Button::new("merge", method.map_or("Merge", |method| method.label()))
                        .primary()
                        .disabled(!can_merge)
                        .on_click(cx.listener(|detail, _, _, cx| {
                            detail.arm_action(Some(ArmedAction::Merge), cx);
                        })),
                )
                .into_any_element(),
        }
    }

    /// The close panel: the branch, and close with its confirmation.
    fn render_close_body(
        &self,
        panel: &ActionPanel,
        options: &MergeOptions,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let number = self
            .pull_request()
            .map_or(0, |pull_request| pull_request.number);
        let row = match panel.armed {
            Some(ArmedAction::Close) => render_confirmation(
                format!("Close #{number} without merging?"),
                "Confirm close",
                cx,
            ),
            Some(ArmedAction::Merge | ArmedAction::AdminMerge) | None => div()
                .flex()
                .gap_2()
                .justify_end()
                .child(Button::new("close-cancel", "Cancel").on_click(cx.listener(
                    |detail, _, _, cx| {
                        detail.toggle_action_panel(PanelAction::Close, cx);
                    },
                )))
                .child(
                    Button::new("close", "Close pull request")
                        .danger()
                        .disabled(!options.permissions.can_close || panel.is_working())
                        .on_click(cx.listener(|detail, _, _, cx| {
                            detail.arm_action(Some(ArmedAction::Close), cx);
                        })),
                )
                .into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(render_line(
                format!("Close #{number} without merging it; it can be reopened on GitHub."),
                theme::muted(),
            ))
            .when(!options.permissions.can_close, |body| {
                body.child(render_line("You may not close it.", theme::error()))
            })
            .child(self.render_branch_choice(panel, options, cx))
            .child(row)
            .into_any_element()
    }

    /// Deleting the head branch with the action: a checkbox when allowed,
    /// else why not.
    fn render_branch_choice(
        &self,
        panel: &ActionPanel,
        options: &MergeOptions,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let branch = &options.head_ref;
        if panel.action == PanelAction::Merge && options.deletes_branch_on_merge {
            return render_line(
                format!("The repository deletes {branch} after merging."),
                theme::muted(),
            );
        }
        if let Some(above) = self.layer_above_head() {
            return render_line(
                format!("{branch} stays: #{above}, the layer above, is based on it."),
                theme::muted(),
            );
        }
        if !self.can_delete_head_branch(Some(options)) {
            return render_line(
                format!("{branch} stays: you may not delete it."),
                theme::muted(),
            );
        }
        let is_on = panel.delete_branch;
        div()
            .id("delete-branch")
            .flex()
            .items_center()
            .gap_1p5()
            .text_xs()
            .cursor_pointer()
            .on_click(cx.listener(|detail, _, _, cx| detail.toggle_delete_branch(cx)))
            .child(if is_on { "☑" } else { "☐" })
            .child(format!("Delete branch {branch}"))
            .into_any_element()
    }

    /// For a stacked pull request with unmerged layers below: they merge too.
    fn stack_merge_note(&self, base_ref: &str) -> Option<String> {
        let current = self.current_layer_index()?;
        let stack = self.stack()?;
        let below = stack.layers[..current]
            .iter()
            .filter(|layer| !layer.is_merged)
            .count();
        (below > 0).then(|| {
            let plural = if below == 1 { "" } else { "s" };
            format!(
                "Stacked: the {below} unmerged layer{plural} below merge too, together into {}.",
                if stack.base_ref.is_empty() {
                    base_ref
                } else {
                    &stack.base_ref
                }
            )
        })
    }
}

/// A button drawn as pressed while its panel is open.
fn with_open_mark(button: Button, is_open: bool) -> Button {
    if is_open { button.primary() } else { button }
}

/// One line of the panel in `color`.
fn render_line(text: impl Into<SharedString>, color: Rgba) -> AnyElement {
    let text: SharedString = text.into();
    div()
        .text_xs()
        .text_color(color)
        .child(text)
        .into_any_element()
}

/// The question of an armed action with its confirm and back buttons.
fn render_confirmation(
    question: String,
    confirm_label: &'static str,
    cx: &mut Context<PullRequestDetail>,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .p_2()
        .rounded_md()
        .bg(theme::panel())
        .child(
            div()
                .flex_1()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child(question),
        )
        .child(
            Button::new("action-back", "Back").on_click(cx.listener(|detail, _, _, cx| {
                detail.arm_action(None, cx);
            })),
        )
        .child(
            Button::new("action-confirm", confirm_label)
                .danger()
                .on_click(cx.listener(|detail, _, _, cx| detail.run_armed_action(cx))),
        )
        .into_any_element()
}

/// How the action is going, while it runs or after.
fn render_progress(progress: &ActionProgress) -> Option<AnyElement> {
    let (text, color) = match progress {
        ActionProgress::Idle => return None,
        ActionProgress::Working(text) => (text.clone(), theme::accent()),
        ActionProgress::Done(text) => (text.clone(), theme::added()),
        ActionProgress::Failed(text) => (text.clone(), theme::error()),
    };
    Some(render_line(text, color))
}
