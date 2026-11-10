use gpui::Context;
use qd_cache::StackReviewMode;
use qd_github::{PullRequestStack, RepoRef};

use super::detail_event::DetailEvent;
use super::file_direction::FileDirection;
use super::layer_request::LayerRequest;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use super::side_panel_tab::SidePanelTab;
use crate::checks_panel::ChecksTarget;
use crate::fetch_stack::{cached_stack, refresh_stack};
use crate::ui_state::UiStateRecorder;

impl PullRequestDetail {
    /// Points the Checks tab at the shown pull request (in a stack, the
    /// shown layer, with the open layers below it once the stack is
    /// known), or at nothing.
    pub(super) fn follow_checks(&mut self, cx: &mut Context<Self>) {
        let layers_below: Vec<u64> = match (self.stack.loaded.as_ref(), self.current_layer_index())
        {
            (Some(stack), Some(current)) => stack.layers[..current]
                .iter()
                .filter(|layer| layer.is_open)
                .map(|layer| layer.number)
                .collect(),
            _ => Vec::new(),
        };
        let target = self.pull_request.as_ref().map(|pull_request| ChecksTarget {
            repo: pull_request.repo.clone(),
            number: pull_request.number,
            updated_at: pull_request.updated_at.clone(),
            layers_below,
        });
        self.checks
            .update(cx, |checks, cx| checks.show_pull_request(target, cx));
    }

    /// Tells the Checks tab whether it is on screen (the side panel shows,
    /// on that tab): it reads and refreshes its checks only then.
    pub(super) fn sync_checks_visibility(&mut self, cx: &mut Context<Self>) {
        let is_on_screen = self.is_side_panel_shown() && self.review.tab == SidePanelTab::Checks;
        self.checks.update(cx, |checks, cx| {
            if is_on_screen {
                checks.show(cx);
            } else {
                checks.hide();
            }
        });
    }

    /// The target for the shown pull request, mode, stack and range start.
    pub(super) fn decide_target(&self) -> ReviewTarget {
        self.pull_request
            .as_ref()
            .map_or(ReviewTarget::Layer, |pull_request| {
                ReviewTarget::decide(
                    self.review_mode,
                    pull_request,
                    self.stack.loaded.as_ref(),
                    self.stack.combined_start,
                )
            })
    }

    /// Recomputes the target and reloads the files when it changed: after
    /// the stack arrived or changed, the mode flipped or the range moved.
    fn retarget(&mut self, cx: &mut Context<Self>) {
        let target = self.decide_target();
        if target == self.target {
            return;
        }
        self.target = target;
        self.pending_scroll = None;
        cx.emit(DetailEvent::TargetChanged);
        self.load_changed_files(cx);
    }

    /// Reads the shown pull request's stack from the cache, then checks it
    /// with GitHub (free when unchanged); each answer may change the target.
    pub(super) fn load_stack(&mut self, cx: &mut Context<Self>) {
        let Some((repo, number)) = self.pull_request.as_ref().and_then(|pull_request| {
            Some((
                pull_request.repo.clone(),
                pull_request.stack.as_ref()?.number,
            ))
        }) else {
            self.stack.stop_loading();
            return;
        };
        let cache = self.services.cache.clone();
        let client = self.services.github.clone();
        self.stack.is_loading = true;
        self.stack.error = None;
        cx.notify();
        self.stack.task = Some(cx.spawn(async move |detail, cx| {
            // The cached layers first, so a combined review starts at once.
            let read_cache = cache.clone();
            let read_repo = repo.clone();
            let cached = cx
                .background_executor()
                .spawn(async move { cached_stack(&read_cache, &read_repo, number) })
                .await;
            let applied = detail.update(cx, |detail, cx| {
                if let Ok(Some(stack)) = cached {
                    detail.apply_stack(stack, cx);
                }
            });
            if applied.is_err() {
                return;
            }

            // Then GitHub's current layers.
            let refreshed = cx
                .background_executor()
                .spawn(async move { refresh_stack(client, &cache, &repo, number) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.stack.is_loading = false;
                    match refreshed {
                        Ok(stack) => detail.apply_stack(stack, cx),
                        Err(error) => {
                            detail.stack.error = Some(format!("{error:#}").into());
                            // Without layers a combined review cannot start; show the layer.
                            if detail.target == ReviewTarget::AwaitingStack {
                                detail.review_mode = StackReviewMode::Layer;
                                detail.retarget(cx);
                            }
                        }
                    }
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Shows `stack`'s layers if it is still the shown pull request's stack.
    fn apply_stack(&mut self, stack: PullRequestStack, cx: &mut Context<Self>) {
        let belongs = self.pull_request.as_ref().is_some_and(|pull_request| {
            pull_request.repo == stack.repo
                && pull_request
                    .stack
                    .as_ref()
                    .is_some_and(|membership| membership.number == stack.number)
        });
        if !belongs || self.stack.loaded.as_ref() == Some(&stack) {
            return;
        }
        self.stack.loaded = Some(stack);
        self.retarget(cx);
        // The layers below tell the Checks tab where a conflict may hold its workflows up.
        self.follow_checks(cx);
        cx.notify();
    }

    /// Switches between one layer and combined layers (`c`), and saves it.
    pub fn toggle_review_mode(&mut self, cx: &mut Context<Self>) {
        let is_stacked = self
            .pull_request
            .as_ref()
            .is_some_and(|pull_request| pull_request.stack.is_some());
        if !is_stacked {
            return;
        }
        self.review_mode = self.review_mode.toggled();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        self.retarget(cx);
        cx.notify();
    }

    /// Reviews the shown stack combined from its lowest unmerged layer up to
    /// the shown one (a click on a stack in the sidebar), and saves the mode.
    pub fn review_combined_stack(&mut self, cx: &mut Context<Self>) {
        self.prepare_combined_stack_review(cx);
        self.retarget(cx);
        cx.notify();
    }

    /// Sets the combined review from the lowest unmerged layer without
    /// reloading, for a pull request about to be shown, and saves the mode.
    pub fn prepare_combined_stack_review(&mut self, cx: &mut Context<Self>) {
        self.stack.combined_start = None;
        if self.review_mode != StackReviewMode::Combined {
            self.review_mode = StackReviewMode::Combined;
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
    }

    /// Whether the files shown (or awaited) are a combined stack review.
    pub fn is_combined_review(&self) -> bool {
        match self.target {
            ReviewTarget::Combined(_) | ReviewTarget::AwaitingStack => true,
            ReviewTarget::Layer => false,
        }
    }

    /// Sets the review mode, from the header's control.
    pub(super) fn set_review_mode(&mut self, mode: StackReviewMode, cx: &mut Context<Self>) {
        if self.review_mode != mode {
            self.toggle_review_mode(cx);
        }
    }

    /// Starts the combined range at layer `number` of `repo` (a shift-click
    /// on a layer in the sidebar) when this stack is shown; `false` when it
    /// is another stack's layer, which the caller opens instead.
    pub fn start_combined_at(
        &mut self,
        repo: &RepoRef,
        number: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        let index = self
            .stack
            .loaded
            .as_ref()
            .filter(|stack| stack.repo == *repo)
            .and_then(|stack| stack.layers.iter().position(|layer| layer.number == number));
        let Some(index) = index else {
            return false;
        };
        self.set_combined_start(index, cx);
        true
    }

    /// Starts the combined range at layer `index`, switching to the combined
    /// review; a layer above the shown one is not a start.
    fn set_combined_start(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(number) = self
            .stack
            .loaded
            .as_ref()
            .and_then(|stack| stack.layers.get(index))
            .map(|layer| layer.number)
        else {
            return;
        };
        if self
            .current_layer_index()
            .is_none_or(|current| index > current)
        {
            return;
        }
        self.stack.combined_start = Some(number);
        if self.review_mode == StackReviewMode::Layer {
            self.review_mode = StackReviewMode::Combined;
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
        self.retarget(cx);
        cx.notify();
    }

    /// Asks the owner to show layer `index` of the stack.
    pub(super) fn open_layer(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(request) = self
            .stack
            .loaded
            .as_ref()
            .and_then(|stack| LayerRequest::of_layer(stack, index))
        else {
            return;
        };
        cx.emit(DetailEvent::OpenLayer(request));
    }

    /// Shows the layer above the shown one (`}`).
    pub fn open_next_layer(&mut self, cx: &mut Context<Self>) {
        self.open_neighbour_layer(FileDirection::Next, cx);
    }

    /// Shows the layer below the shown one (`{`).
    pub fn open_previous_layer(&mut self, cx: &mut Context<Self>) {
        self.open_neighbour_layer(FileDirection::Previous, cx);
    }

    /// Shows the layer above or below the shown one.
    fn open_neighbour_layer(&mut self, direction: FileDirection, cx: &mut Context<Self>) {
        let Some(current) = self.current_layer_index() else {
            return;
        };
        let target = match direction {
            FileDirection::Next => current + 1,
            FileDirection::Previous => match current.checked_sub(1) {
                Some(below) => below,
                None => return,
            },
        };
        self.open_layer(target, cx);
    }
}
