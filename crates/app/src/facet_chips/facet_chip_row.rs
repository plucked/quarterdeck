use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, RenderOnce, SharedString, Window,
    prelude::*,
};
use qd_file_tree::{FacetChoice, FacetCount};

use super::facet_chip_rendering::{render_chip_row, render_more_chip};
use super::{ChipClick, ChipRowClick, FacetChip};

/// What a click on one of the row's chips does.
type ChipRowClickHandler = Rc<dyn Fn(&ChipRowClick, &mut Window, &mut App)>;
/// What a click on the row's more / less link does.
type MoreClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One chip the row draws, as its caller decided it.
struct RowChip {
    count: FacetCount<SharedString>,
    choice: FacetChoice,
    mark: Option<AnyElement>,
}

/// A labelled row of filter chips that folds. The caller decides which
/// chips show (all of them, or a collapsed few) and the more / less link
/// closing the row, and gets every chip click as a [`ChipRowClick`]; the
/// row knows no filter. A chip's element id is `{id}-{value}`, the link's
/// is the child `more` of `id`, so the two never meet.
#[derive(IntoElement)]
pub struct FacetChipRow {
    id: SharedString,
    label: &'static str,
    chips: Vec<RowChip>,
    more_label: Option<String>,
    on_chip: Option<ChipRowClickHandler>,
    on_toggle_more: Option<MoreClickHandler>,
}

impl FacetChipRow {
    /// An empty row labelled `label`, whose element ids start with `id`.
    pub fn new(id: impl Into<SharedString>, label: &'static str) -> Self {
        Self {
            id: id.into(),
            label,
            chips: Vec::new(),
            more_label: None,
            on_chip: None,
            on_toggle_more: None,
        }
    }

    /// Adds the chip for `count`, labelled with its value, picked, hidden
    /// or open as `choice` says, with `mark` before the label when there is one.
    #[must_use]
    pub fn chip(
        mut self,
        count: &FacetCount<String>,
        choice: FacetChoice,
        mark: Option<AnyElement>,
    ) -> Self {
        self.chips.push(RowChip {
            count: FacetCount {
                value: count.value.clone().into(),
                total: count.total,
                matching: count.matching,
            },
            choice,
            mark,
        });
        self
    }

    /// Closes the row with the more / less link `label`, when there is one.
    #[must_use]
    pub fn more_label(mut self, label: Option<String>) -> Self {
        self.more_label = label;
        self
    }

    /// Called with the chip's value and the kind of click when a chip is clicked.
    #[must_use]
    pub fn on_chip(
        mut self,
        handler: impl Fn(&ChipRowClick, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_chip = Some(Rc::new(handler));
        self
    }

    /// Called when the more / less link is clicked.
    #[must_use]
    pub fn on_toggle_more(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_more = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for FacetChipRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // The chips, each telling the row's handler its value and the kind of click.
        let id = self.id;
        let on_chip = self.on_chip;
        let mut chips: Vec<AnyElement> = self
            .chips
            .into_iter()
            .map(|chip| {
                let value = chip.count.value.clone();
                FacetChip::new(
                    format!("{id}-{value}"),
                    value.clone(),
                    &chip.count,
                    chip.choice,
                )
                .leading(chip.mark)
                .when_some(on_chip.clone(), |facet_chip, on_chip| {
                    facet_chip.on_click(move |event, window, cx| {
                        let click = ChipRowClick {
                            value: value.clone(),
                            click: ChipClick::of(event),
                        };
                        on_chip(&click, window, cx);
                    })
                })
                .into_any_element()
            })
            .collect();

        // The more / less link, when the caller gave one.
        if let Some(label) = self.more_label {
            let more_id = ElementId::NamedChild(Box::new(ElementId::Name(id)), "more".into());
            chips.push(
                render_more_chip(more_id, label)
                    .when_some(self.on_toggle_more, |more, handler| more.on_click(handler))
                    .into_any_element(),
            );
        }
        render_chip_row(self.label, chips)
    }
}
