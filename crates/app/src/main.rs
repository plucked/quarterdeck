//! Quarterdeck: a macOS desktop app for reviewing GitHub pull requests.
//!
//! Files:
//! - `launch_app/`: starts gpui and opens the main window where it was last placed.
//! - `main_window.rs`: `MainWindow`, the root view; records the window placement; fades in after the splash.
//! - `startup_splash/`: `StartupSplash`, the intro movie at launch, played while the main window loads.
//! - `app_commands.rs`: app-level actions with their key bindings and menus.
//! - `app_update/`: `AppUpdater`, checks for, downloads and installs new versions.
//! - `brief_view/`: a written brief drawn natively (status, lead, pages of widgets), shared by the Brief pane and the gallery.
//! - `brief_gallery/`: the Brief Gallery window, every brief widget on sample data, for working on their layout.
//! - `avatar_images/`: `AvatarImages` and `Avatar`, people's GitHub avatars, loaded in the background.
//! - `app_settings.rs`: `AppSettings`, the user's preferences shared by every window.
//! - `app_services.rs`: `AppServices`, the GitHub client and cache handed to views.
//! - `controls/`: small reusable components (buttons, checkboxes, option rows, fold chevrons, tooltips, labels) and the option handler helper.
//! - `scrollbar/`: `VerticalScrollbar`, a draggable scrollbar for any scroll container.
//! - `shared_cache.rs`: `SharedCache`, the cache shared between views and background tasks.
//! - `fetch_stack.rs`: gets a stack's layers from the cache, and checks them with GitHub.
//! - `word_range_at.rs`: the word around an offset, for double-click selection.
//! - `theme/`: the eight color themes, the active one's colors, and the fonts.
//! - `tree_locate_button.rs`: `TreeLocateButton`, the file header's icon button showing the file in the file tree.
//! - `bundled_fonts.rs`: registers the fonts built into the binary (`Schibsted Grotesk`, `JetBrains Mono`).
//! - `bundled_icons.rs`: `BundledIcons`, the asset source serving the SVG icons built into the binary.
//! - `file_icon_svgs.rs`: the SVG of each file icon in the file tree, and which one the active theme draws.
//! - `github_link.rs`: `GitHubLink`, GitHub's mark as an icon button opening a page on GitHub.
//! - `diff_line_sides.rs`: a diff line's number on the old or new side, and a brief anchor's side as one.
//! - `diff_view/`: per-row diff rendering and the summary bar above the files.
//! - `settings_window/`: the settings window (cmd-,), one page per category.
//! - `display_text/`: values as people read them: dates and ages, sizes, short SHAs, counts with their noun.
//! - `count_as_f32.rs`: `count_as_f32`, a small count as a float for layout arithmetic.
//! - `distinct_repositories.rs`: `distinct_repositories`, repositories each once in the order first seen.
//! - `facet_chips/`: filter chips (pick with a click, hide with ⌥-click), shared by the file tree and the Others tab.
//! - `review_verdict_badge.rs`: `ReviewVerdictBadge`, the viewer's latest review verdict as a small round mark.
//! - `panel_toggle.rs`: `PanelToggle`, the icon button showing or hiding a side panel.
//! - `text_area/`: `TextArea`, a multiline text editor (gpui has none).
//! - `markdown_view/`: `MarkdownView`, renders parsed Markdown (descriptions, comments).
//! - `review_queue/`: the main view, the queue of pull requests awaiting review.
//! - `pull_request_detail/`: `PullRequestDetail`, the selected pull request as a virtualized list.
//! - `checks_panel/`: `ChecksPanel`, the side panel's Checks tab: the shown pull request's CI checks.
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
mod checks_panel;
mod controls;
mod count_as_f32;
mod diff_line_sides;
mod diff_view;
mod display_text;
mod distinct_repositories;
mod facet_chips;
mod fetch_stack;
mod file_icon_svgs;
mod github_link;
mod launch_app;
mod main_window;
mod markdown_view;
mod panel_toggle;
mod pull_request_detail;
mod review_queue;
mod review_verdict_badge;
mod scrollbar;
mod settings_window;
mod shared_cache;
mod startup_splash;
mod text_area;
mod theme;
mod tree_locate_button;
mod ui_state;
mod word_range_at;

fn main() {
    launch_app::launch_app();
}
