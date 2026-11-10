//! The settings window (cmd-,): every preference of the app, grouped into
//! categories listed on the left. Changes apply at once through
//! `AppSettings` and are saved.
//!
//! Files:
//! - `open_settings_window.rs`: opens the window, or brings the open one to the front.
//! - `settings_window.rs`: `SettingsWindow`, the window's view: category list and page.
//! - `settings_category.rs`: `SettingsCategory`, the categories in order.
//! - `setting_row.rs`: `SettingRow`, one setting's title, explanation and control; `SETTING_FIELD_WIDTH`.
//! - `empty_list_note.rs`: `EmptyListNote`, the note in place of an empty list on a page.
//! - `removable_row.rs`: `RemovableRow`, a list row on a page with a Remove button.
//! - `field_error.rs`: `FieldError`, why what was typed into a field could not be used.
//! - `appearance_page.rs`: the Appearance category: the color theme cards and the preview.
//! - `theme_preview.rs`: a small pull request diff drawn in any theme's colors.
//! - `file_diff_page.rs`: the File Diff category's settings.
//! - `github_page.rs`: the GitHub category: the account rows, cache TTL, clearing the cache, API usage.
//! - `github_sign_in.rs`: `TokenSignIn`, the token field and its state; the GitHub category's account rows: signing in with a token, signing out.
//! - `sign_in_status.rs`: `SignInStatus`, where signing in with a token stands.
//! - `repositories_page.rs`: the Repositories category: the repositories the Others tab lists, adding (checked with GitHub; `RepositoryAdder`, the field, its check and the quick picks) and removing.
//! - `add_repository_status.rs`: `AddRepositoryStatus`, where adding a typed repository stands.
//! - `filters_page.rs`: the Filters category: the rules hiding pull requests from every queue tab by branch; `BranchRuleForm`, the pattern field, side and error.
//! - `updates_page.rs`: the Updates category: the version, update status, release notes, restart.

mod add_repository_status;
mod appearance_page;
mod empty_list_note;
mod field_error;
mod file_diff_page;
mod filters_page;
mod github_page;
mod github_sign_in;
mod open_settings_window;
mod removable_row;
mod repositories_page;
mod setting_row;
mod settings_category;
mod settings_window;
mod sign_in_status;
mod theme_preview;
mod updates_page;

pub use open_settings_window::{open_settings_window, open_settings_window_at};
pub use settings_category::SettingsCategory;
