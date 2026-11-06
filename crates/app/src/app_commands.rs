use gpui::{App, KeyBinding, Menu, MenuItem, SystemMenuType, actions};

use crate::app_update::AppUpdater;
use crate::settings_window::{SettingsCategory, open_settings_window, open_settings_window_at};
use crate::text_area::bind_text_area_keys;

/// The key context of the review queue; single-letter keys only act inside it.
pub const REVIEW_QUEUE_CONTEXT: &str = "ReviewQueue";

/// Where the single-letter keys act: in the review queue, but not while a
/// text area (key context `TextArea`) takes the typing.
const SINGLE_KEY_CONTEXT: &str = "ReviewQueue && !TextArea";

actions!(
    quarterdeck,
    [
        Quit,
        OpenSettings,
        CheckForUpdates,
        CloseWindow,
        Refresh,
        Copy,
        NextFile,
        PreviousFile,
        MarkViewedAndNext,
        ToggleFileTree,
        ToggleSidebar,
        NextLayer,
        PreviousLayer,
        ToggleStackReview,
        ShowAuthored,
        ShowReviewRequested,
        ShowOthers
    ]
);

/// Registers the app-level command handlers, their key bindings and the menu bar.
pub fn install_app_commands(cx: &mut App) {
    // Quit and Settings work from any window; the others go to the focused view.
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &OpenSettings, cx| open_settings_window(cx));
    cx.on_action(|_: &CheckForUpdates, cx| {
        AppUpdater::global(cx).update(cx, AppUpdater::check_now);
        open_settings_window_at(SettingsCategory::Updates, cx);
    });
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-r", Refresh, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-b", ToggleFileTree, None),
        KeyBinding::new("ctrl-cmd-s", ToggleSidebar, None),
        KeyBinding::new("]", NextFile, Some(SINGLE_KEY_CONTEXT)),
        KeyBinding::new("[", PreviousFile, Some(SINGLE_KEY_CONTEXT)),
        KeyBinding::new("v", MarkViewedAndNext, Some(SINGLE_KEY_CONTEXT)),
        KeyBinding::new("}", NextLayer, Some(SINGLE_KEY_CONTEXT)),
        KeyBinding::new("{", PreviousLayer, Some(SINGLE_KEY_CONTEXT)),
        KeyBinding::new("c", ToggleStackReview, Some(SINGLE_KEY_CONTEXT)),
        KeyBinding::new("cmd-1", ShowAuthored, None),
        KeyBinding::new("cmd-2", ShowReviewRequested, None),
        KeyBinding::new("cmd-3", ShowOthers, None),
    ]);

    // Text areas have their own editing keys, and the queue's single keys stay out of them.
    bind_text_area_keys(cx);

    // The menu bar exposes the same actions as the key bindings.
    cx.set_menus(vec![
        Menu {
            name: "Quarterdeck".into(),
            items: vec![
                MenuItem::action("Check for Updates…", CheckForUpdates),
                MenuItem::action("Settings…", OpenSettings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Quarterdeck", Quit),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![MenuItem::action("Copy", Copy)],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Refresh", Refresh),
                MenuItem::separator(),
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle File Tree", ToggleFileTree),
                MenuItem::action("Next File", NextFile),
                MenuItem::action("Previous File", PreviousFile),
                MenuItem::action("Mark Viewed and Next File", MarkViewedAndNext),
                MenuItem::separator(),
                MenuItem::action("My Pull Requests", ShowAuthored),
                MenuItem::action("Review Requests", ShowReviewRequested),
                MenuItem::action("Others' Pull Requests", ShowOthers),
                MenuItem::separator(),
                MenuItem::action("Next Stack Layer", NextLayer),
                MenuItem::action("Previous Stack Layer", PreviousLayer),
                MenuItem::action("Toggle Combined Stack Review", ToggleStackReview),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Close Window", CloseWindow)],
        },
    ]);
}
