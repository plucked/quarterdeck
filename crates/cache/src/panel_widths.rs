/// Default width of the queue sidebar, in pixels.
const DEFAULT_SIDEBAR_WIDTH: f32 = 380.;
/// Default width of the review pane's side panel (stack map and file tree).
const DEFAULT_SIDE_PANEL_WIDTH: f32 = 300.;

/// The widths the resizable panels were dragged to, kept across launches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelWidths {
    /// The queue sidebar on the left.
    pub sidebar: f32,
    /// The review pane's side panel on the right.
    pub side_panel: f32,
}

impl Default for PanelWidths {
    fn default() -> Self {
        Self {
            sidebar: DEFAULT_SIDEBAR_WIDTH,
            side_panel: DEFAULT_SIDE_PANEL_WIDTH,
        }
    }
}
