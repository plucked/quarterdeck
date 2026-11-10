use gpui::{AnyElement, App, Context, FontWeight, IntoElement, SharedString, div, prelude::*, px};
use qd_cache::{BranchSide, HiddenBranchRule};

use super::empty_list_note::EmptyListNote;
use super::field_error::FieldError;
use super::removable_row::RemovableRow;
use super::setting_row::{SETTING_FIELD_WIDTH, SettingRow};
use super::settings_window::SettingsWindow;
use crate::app_settings::AppSettings;
use crate::button::Button;
use crate::section_label::SectionLabel;
use crate::segmented_control::SegmentedControl;
use crate::theme;
use crate::view_option::view_option;

impl SettingsWindow {
    /// The Filters page: rules hiding pull requests from every queue tab by
    /// branch name (a wildcard pattern and which branch it looks at), a
    /// field to add one and the list with a Remove each.
    pub(super) fn render_filters_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let rules = AppSettings::hidden_branch_rules(cx).to_vec();
        let picked_side = self.branch_rule_side;
        let settings = cx.entity().downgrade();
        let side_picker = BranchSide::ALL.iter().fold(
            SegmentedControl::new("branch-rule-side").fill_width(),
            |control, &side| {
                control.option(
                    side_label(side),
                    side == picked_side,
                    view_option(&settings, side, |settings, side, cx| {
                        settings.branch_rule_side = side;
                        cx.notify();
                    }),
                )
            },
        );

        div()
            .flex()
            .flex_col()
            .child(SettingRow::new(
                "Hide branches",
                "Pull requests whose branch matches a pattern are left out of Mine (⌘1), Review (⌘2) and Others (⌘3). * stands for any characters, / included, and ? for one: feat/legacy-* hides feat/legacy-login. Pick whether the pattern looks at the branch a pull request comes from, the one it goes into, or either.",
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .w(px(SETTING_FIELD_WIDTH))
                    .child(self.branch_pattern_input.clone())
                    .child(side_picker)
                    .child(
                        div().flex().child(
                            Button::new("add-branch-rule", "Add")
                                .primary()
                                .on_click(cx.listener(|settings, _, _, cx| settings.add_typed_branch_rule(cx))),
                        ),
                    )
                    .children(self.branch_rule_error.clone().map(FieldError::new)),
            ))
            .child(
                div()
                    .pt_4()
                    .pb_1()
                    .child(SectionLabel::new(format!("Hidden branches · {}", rules.len())).weight(FontWeight::MEDIUM)),
            )
            .child(render_rule_list(&rules))
            .into_any_element()
    }

    /// Adds the pattern typed into the field, for the picked side; refuses
    /// an empty one and one listed already for that side.
    pub(super) fn add_typed_branch_rule(&mut self, cx: &mut Context<Self>) {
        // Read what was typed, trimmed.
        let pattern = self.branch_pattern_input.read(cx).text().trim().to_owned();
        if pattern.is_empty() {
            self.fail_to_add_branch_rule("Type a branch name or pattern, like feat/legacy-*.", cx);
            return;
        }
        let rule = HiddenBranchRule {
            pattern,
            side: self.branch_rule_side,
        };
        let mut rules = AppSettings::hidden_branch_rules(cx).to_vec();
        if rules.contains(&rule) {
            self.fail_to_add_branch_rule("That pattern is in the list already.", cx);
            return;
        }

        // Save it, and empty the field for the next one.
        rules.push(rule);
        AppSettings::set_hidden_branch_rules(cx, rules);
        self.branch_rule_error = None;
        self.branch_pattern_input
            .update(cx, |input, cx| input.set_text("", cx));
        cx.notify();
    }

    fn fail_to_add_branch_rule(&mut self, reason: &'static str, cx: &mut Context<Self>) {
        self.branch_rule_error = Some(SharedString::new_static(reason));
        cx.notify();
    }
}

/// The picker's label for `side`.
fn side_label(side: BranchSide) -> &'static str {
    match side {
        BranchSide::Either => "Either",
        BranchSide::Head => "From",
        BranchSide::Base => "Into",
    }
}

/// What a listed rule's badge says it looks at.
fn side_description(side: BranchSide) -> &'static str {
    match side {
        BranchSide::Either => "from or into",
        BranchSide::Head => "from",
        BranchSide::Base => "into",
    }
}

/// The hidden branch rules, one row each with its pattern, side and a Remove.
fn render_rule_list(rules: &[HiddenBranchRule]) -> AnyElement {
    if rules.is_empty() {
        return EmptyListNote::new("None yet. Every pull request shows.").into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .children(rules.iter().enumerate().map(|(index, rule)| {
            let removed = rule.clone();
            RemovableRow::new(("remove-branch-rule", index), move |cx| {
                remove_branch_rule(&removed, cx);
            })
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded_full()
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::surface())
                    .text_xs()
                    .text_color(theme::muted())
                    .child(side_description(rule.side)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_family(theme::MONO_FONT)
                    .child(rule.pattern.clone()),
            )
        }))
        .into_any_element()
}

/// Removes `rule` from the hidden branch rules.
fn remove_branch_rule(rule: &HiddenBranchRule, cx: &mut App) {
    let rules: Vec<HiddenBranchRule> = AppSettings::hidden_branch_rules(cx)
        .iter()
        .filter(|kept| *kept != rule)
        .cloned()
        .collect();
    AppSettings::set_hidden_branch_rules(cx, rules);
}
