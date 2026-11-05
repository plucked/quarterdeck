//! The settings window (cmd-,): every preference of the app, grouped into
//! categories listed on the left. Changes apply at once through
//! `AppSettings` and are saved.
//!
//! Files:
//! - `open_settings_window.rs`: opens the window, or brings the open one to the front.
//! - `settings_window.rs`: `SettingsWindow`, the window's view: category list and page.
//! - `settings_category.rs`: `SettingsCategory`, the categories in order.
//! - `setting_row.rs`: `SettingRow`, one setting's title, explanation and control.
//! - `appearance_page.rs`: the Appearance category: the color theme cards and the preview.
//! - `theme_preview.rs`: a small pull request diff drawn in any theme's colors.
//! - `file_diff_page.rs`: the File Diff category's settings.
//! - `github_page.rs`: the GitHub category: cache TTL, clearing the cache, API usage.

mod appearance_page;
mod file_diff_page;
mod github_page;
mod open_settings_window;
mod setting_row;
mod settings_category;
mod settings_window;
mod theme_preview;

pub use open_settings_window::open_settings_window;
