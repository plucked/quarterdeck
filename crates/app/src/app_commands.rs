use gpui::{App, KeyBinding, Menu, MenuItem, SystemMenuType, actions};

use crate::settings_window::open_settings_window;

actions!(
    quick_check,
    [Quit, OpenSettings, CloseWindow, Refresh, Copy]
);

/// Registers the app-level command handlers, their key bindings and the menu bar.
pub fn install_app_commands(cx: &mut App) {
    // Quit and Settings work from any window; the others go to the focused view.
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &OpenSettings, cx| open_settings_window(cx));
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-r", Refresh, None),
        KeyBinding::new("cmd-c", Copy, None),
    ]);

    // The menu bar exposes the same actions as the key bindings.
    cx.set_menus(vec![
        Menu {
            name: "Quick Check".into(),
            items: vec![
                MenuItem::action("Settings…", OpenSettings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Quick Check", Quit),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![MenuItem::action("Copy", Copy)],
        },
        Menu {
            name: "View".into(),
            items: vec![MenuItem::action("Refresh", Refresh)],
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Close Window", CloseWindow)],
        },
    ]);
}
