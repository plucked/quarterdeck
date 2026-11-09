//! Quarterdeck: a macOS desktop app for reviewing GitHub pull requests.
//!
//! Files:
//! - `launch_app.rs`: starts gpui and opens the main window where it was last placed.
//! - `main_window.rs`: `MainWindow`, the root view; records the window placement; fades in after the splash.
//! - `startup_splash/`: `StartupSplash`, the intro movie at launch, played while the main window loads.
//! - `app_commands.rs`: app-level actions with their key bindings and menus.
//! - `app_update/`: `AppUpdater`, checks for, downloads and installs new versions.
//! - `brief_view/`: a written brief drawn natively (status, lead, pages of widgets), shared by the Brief pane and the gallery.
//! - `brief_gallery/`: the Brief Gallery window, every brief widget on sample data, for working on their layout.
//! - `avatar_images/`: `AvatarImages` and `Avatar`, people's GitHub avatars, loaded in the background.
//! - `app_settings.rs`: `AppSettings`, the user's preferences shared by every window.
//! - `app_services.rs`: `AppServices`, the GitHub client and cache handed to views.
//! - `scrollbar/`: `VerticalScrollbar`, a draggable scrollbar for any scroll container.
//! - `shared_cache.rs`: `SharedCache`, the cache shared between views and background tasks.
//! - `fetch_stack.rs`: gets a stack's layers from the cache, and checks them with GitHub.
//! - `word_range_at.rs`: the word around an offset, for double-click selection.
//! - `theme.rs`: the color palette (Graphite) and fonts.
//! - `tree_locate_button.rs`: `TreeLocateButton`, the file header's icon button showing the file in the file tree.
//! - `bundled_fonts.rs`: registers the fonts built into the binary (`Schibsted Grotesk`, `JetBrains Mono`).
//! - `bundled_icons.rs`: `BundledIcons`, the asset source serving the SVG icons built into the binary.
//! - `github_link.rs`: `GitHubLink`, GitHub's mark as an icon button opening a page on GitHub.
//! - `diff_view/`: per-row diff rendering and the summary bar above the files.
//! - `settings_window/`: the settings window (cmd-,), one page per category.
//! - `segmented_control.rs`: `SegmentedControl`, a row of exclusive options.
//! - `button.rs`: `Button`, a small text button.
//! - `display_date.rs`: `short_date`, a GitHub timestamp as `21 Oct 2026`.
//! - `display_size.rs`: `byte_size`, a byte count as `151.7 MB`.
//! - `short_sha.rs`: `short_sha`, a commit SHA's first seven characters.
//! - `facet_chips/`: filter chips (pick with a click, hide with ⌥-click), shared by the file tree and the Others tab.
//! - `repo_monogram.rs`: `RepoMonogram`, a repository's colored letter (sidebar, settings).
//! - `fold_chevron.rs`: `FoldChevron`, the button folding a group.
//! - `checkbox.rs`: `Checkbox`, a drawn checkbox (checked, unchecked or mixed).
//! - `text_tooltip.rs`: `TextTooltip`, a short text shown over an element on hover.
//! - `review_verdict_badge.rs`: `ReviewVerdictBadge`, the viewer's latest review verdict as a small round mark.
//! - `panel_toggle.rs`: `PanelToggle`, the icon button showing or hiding a side panel.
//! - `resize_handle.rs`: `ResizeHandle`, a divider dragged to resize the panel beside it.
//! - `text_area/`: `TextArea`, a multiline text editor (gpui has none).
//! - `markdown_view/`: `MarkdownView`, renders parsed Markdown (descriptions, comments).
//! - `review_queue/`: the main view, the queue of pull requests awaiting review.
//! - `pull_request_detail/`: `PullRequestDetail`, the selected pull request as a virtualized list.
//! - `ui_state/`: saves and restores what the UI showed across relaunches.

mod app_commands;
mod app_services;
mod app_settings;
mod app_update;
mod avatar_images;
mod brief_gallery;
mod brief_view;
mod bundled_fonts;
mod bundled_icons;
mod button;
mod checkbox;
mod diff_view;
mod display_date;
mod display_size;
mod facet_chips;
mod fetch_stack;
mod fold_chevron;
mod github_link;
mod launch_app;
mod main_window;
mod markdown_view;
mod panel_toggle;
mod pull_request_detail;
mod repo_monogram;
mod resize_handle;
mod review_queue;
mod review_verdict_badge;
mod scrollbar;
mod segmented_control;
mod settings_window;
mod shared_cache;
mod short_sha;
mod startup_splash;
mod text_area;
mod text_tooltip;
mod theme;
mod tree_locate_button;
mod ui_state;
mod word_range_at;

fn main() {
    launch_app::launch_app();
}
