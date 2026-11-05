//! quick-check: a macOS desktop app for reviewing GitHub pull requests.
//!
//! Files:
//! - `launch_app.rs`: starts gpui and opens the main window where it was last placed.
//! - `main_window.rs`: `MainWindow`, the root view; records the window placement.
//! - `app_commands.rs`: app-level actions with their key bindings and menus.
//! - `app_settings.rs`: `AppSettings`, the user's preferences shared by every window.
//! - `app_services.rs`: `AppServices`, the GitHub client and cache handed to views.
//! - `scrollbar/`: `VerticalScrollbar`, a draggable scrollbar for any scroll container.
//! - `shared_cache.rs`: `SharedCache`, the cache shared between views and background tasks.
//! - `fetch_stack.rs`: gets a stack's layers from the cache, and checks them with GitHub.
//! - `word_range_at.rs`: the word around an offset, for double-click selection.
//! - `theme.rs`: the color palette and fonts.
//! - `diff_view/`: per-row diff rendering and the summary bar above the files.
//! - `settings_window/`: the settings window (cmd-,), one page per category.
//! - `segmented_control.rs`: `SegmentedControl`, a row of exclusive options.
//! - `button.rs`: `Button`, a small text button.
//! - `display_date.rs`: `short_date`, a GitHub timestamp as `21 Oct 2026`.
//! - `checkbox.rs`: `Checkbox`, a drawn checkbox (checked, unchecked or mixed).
//! - `resize_handle.rs`: `ResizeHandle`, a divider dragged to resize the panel beside it.
//! - `text_area/`: `TextArea`, a multiline text editor (gpui has none).
//! - `markdown_view/`: `MarkdownView`, renders parsed Markdown (descriptions, comments).
//! - `review_queue/`: the main view, the queue of pull requests awaiting review.
//! - `pull_request_detail/`: `PullRequestDetail`, the selected pull request as a virtualized list.
//! - `ui_state/`: saves and restores what the UI showed across relaunches.

mod app_commands;
mod app_services;
mod app_settings;
mod button;
mod checkbox;
mod diff_view;
mod display_date;
mod fetch_stack;
mod launch_app;
mod main_window;
mod markdown_view;
mod pull_request_detail;
mod resize_handle;
mod review_queue;
mod scrollbar;
mod segmented_control;
mod settings_window;
mod shared_cache;
mod text_area;
mod theme;
mod ui_state;
mod word_range_at;

fn main() {
    launch_app::launch_app();
}
