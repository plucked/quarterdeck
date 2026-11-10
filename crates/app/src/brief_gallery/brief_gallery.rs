use std::rc::Rc;

use gpui::{Context, FocusHandle, ListAlignment, ListOffset, ListState, Subscription, Window, px};

use super::gallery_row::GalleryRow;
use super::sample_brief::SampleBrief;
use qd_studio::ReaderFamiliarity;
use qd_update::AppVersion;

use crate::app_settings::AppSettings;
use crate::brief_view::{
    BriefHandlers, BriefPageRow, BriefReport, DEFAULT_DIAGRAM_WIDTH, ReportHost, view_action,
};

/// The report panel's text field while empty.
const REPORT_PLACEHOLDER: &str = "What is wrong or weird in this brief…";
/// How far beyond the visible area the gallery's rows are laid out.
const GALLERY_OVERDRAW: f32 = 400.;

/// The brief gallery's view: every status card and every widget of a
/// sample brief, drawn by the same code as the Brief pane, to work on how
/// briefs look without running an agent. Anchor links and the buttons do
/// nothing here; tabs switch pages. Redraws when the theme
/// changes in Settings.
pub struct BriefGallery {
    pub(super) focus: FocusHandle,
    /// The sample, or why it could not be built.
    pub(super) sample: Result<SampleBrief, String>,
    pub(super) rows: Vec<GalleryRow>,
    pub(super) list: ListState,
    /// The sample brief's page shown.
    pub(super) page: usize,
    /// The reader familiarity picked in the gallery's pickers.
    pub(super) reader: ReaderFamiliarity,
    /// Reporting the sample brief: saving writes a real report of the sample.
    pub(super) report: BriefReport,
    pub(super) handlers: BriefHandlers,
    /// The width of the brief's column, which the sample's diagrams are laid out for.
    pub(super) column_width: f32,
    /// Keeps the theme observer registered for the view's lifetime.
    _settings_subscription: Subscription,
}

impl BriefGallery {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so cmd-w reaches it.
        let focus = cx.focus_handle();
        window.focus(&focus);

        // Lay out the sample's rows: intro, status cards, then the brief.
        let sample = SampleBrief::build();
        let rows = gallery_rows(sample.as_ref().ok(), 0);
        let list = ListState::new(rows.len(), ListAlignment::Top, px(GALLERY_OVERDRAW));

        // Pages switch and the report works; the rest has no diff or agent to reach.
        let gallery = cx.entity().downgrade();
        let handlers = BriefHandlers {
            show_anchor: Rc::new(|_, _, _| {}),
            write: Rc::new(|_, _| {}),
            cancel: Rc::new(|_, _| {}),
            show_page: Rc::new({
                let gallery = gallery.clone();
                move |page, _, cx| {
                    gallery
                        .update(cx, |gallery, cx| gallery.show_page(page, cx))
                        .ok();
                }
            }),
            choose_reader: Rc::new({
                let gallery = gallery.clone();
                move |familiarity, _, cx| {
                    gallery
                        .update(cx, |gallery, cx| gallery.choose_reader(familiarity, cx))
                        .ok();
                }
            }),
            open_report: view_action(&gallery, |gallery, window, cx| {
                gallery.open_report(REPORT_PLACEHOLDER, window, cx);
            }),
            save_report: view_action(&gallery, |gallery, _, cx| gallery.save_sample_report(cx)),
            close_report: view_action(&gallery, |gallery, _, cx| gallery.close_report(cx)),
            copy_report_path: view_action(&gallery, |gallery, _, cx| gallery.copy_report_path(cx)),
        };
        Self {
            focus,
            sample,
            rows,
            list,
            page: 0,
            reader: ReaderFamiliarity::default(),
            report: BriefReport::default(),
            column_width: DEFAULT_DIAGRAM_WIDTH,
            handlers,
            _settings_subscription: cx.observe_global::<AppSettings>(|_, cx| cx.notify()),
        }
    }

    /// Picks `familiarity` in every picker; the rows showing one are laid
    /// out again, since the line explaining the choice changes.
    fn choose_reader(&mut self, familiarity: ReaderFamiliarity, cx: &mut Context<Self>) {
        self.reader = familiarity;
        for (index, row) in self.rows.iter().enumerate() {
            if matches!(
                row,
                GalleryRow::Status(_) | GalleryRow::Page(BriefPageRow::Footer)
            ) {
                self.list.splice(index..index + 1, 1);
            }
        }
        cx.notify();
    }

    /// Writes a real report of the sample brief with the words in the panel,
    /// as the Brief pane does, to try the report itself.
    fn save_sample_report(&mut self, cx: &mut Context<Self>) {
        if let Ok(sample) = &self.sample {
            let record = sample.brief.record.clone();
            let shown_page = sample.brief.shown_page(self.page);
            let version = format!("{} (gallery sample)", AppVersion::current());
            self.save_report(record, shown_page, version, cx);
        }
    }

    /// Lays the sample's diagrams out for a column `width` wide (the window
    /// was resized), and the rows showing them out again.
    pub(super) fn set_column_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.column_width = width;
        let Ok(sample) = &mut self.sample else {
            return;
        };
        let changed = sample.brief.fit_diagrams(width);
        // Measured again where they are; the reader's place stays.
        let top = self.list.logical_scroll_top();
        for (index, row) in self.rows.iter().enumerate() {
            if let GalleryRow::Page(BriefPageRow::Widget(widget)) = row
                && changed.contains(widget)
            {
                self.list.splice(index..index + 1, 1);
            }
        }
        self.list.scroll_to(top);
        cx.notify();
    }

    /// Shows page `page` of the sample brief, scrolled to its tabs.
    fn show_page(&mut self, page: usize, cx: &mut Context<Self>) {
        self.page = page;
        self.report.close();
        self.rows = gallery_rows(self.sample.as_ref().ok(), page);
        self.list.reset(self.rows.len());
        if let Some(tabs) = self
            .rows
            .iter()
            .position(|row| *row == GalleryRow::PageTabs)
        {
            self.list.scroll_to(ListOffset {
                item_ix: tabs,
                offset_in_item: px(0.),
            });
        }
        cx.notify();
    }
}

/// The gallery's rows for `sample` showing `page`: the intro, each status
/// card, then the written brief's tabs and the page as the Brief pane lays
/// it out; without a sample, why not.
fn gallery_rows(sample: Option<&SampleBrief>, page: usize) -> Vec<GalleryRow> {
    let Some(sample) = sample else {
        return vec![GalleryRow::Intro, GalleryRow::Broken];
    };
    let mut rows = vec![GalleryRow::Intro, GalleryRow::Heading("Status card")];
    rows.extend((0..sample.statuses.len()).map(GalleryRow::Status));
    rows.push(GalleryRow::Heading("Written brief"));
    rows.push(GalleryRow::Version);
    if sample.brief.pages.len() > 1 {
        rows.push(GalleryRow::PageTabs);
    }
    rows.extend(
        BriefPageRow::rows_of(&sample.brief, page)
            .into_iter()
            .map(GalleryRow::Page),
    );
    rows
}

/// The report panel is the Version row's; it is laid out again when the panel changes.
impl ReportHost for BriefGallery {
    fn report_mut(&mut self) -> &mut BriefReport {
        &mut self.report
    }

    fn report_changed(&mut self) {
        if let Some(index) = self.rows.iter().position(|row| *row == GalleryRow::Version) {
            self.list.splice(index..index + 1, 1);
        }
    }
}
