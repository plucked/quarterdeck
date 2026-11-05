use gpui::{App, KeyBinding, Menu, MenuItem, SystemMenuType, actions};

actions!(quick_check, [Quit, Refresh, Copy]);

/// Registers the app-level command handlers, their key bindings and the menu bar.
pub fn install_app_commands(cx: &mut App) {
    // Quit is handled globally; Refresh and Copy by the focused view.
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-r", Refresh, None),
        KeyBinding::new("cmd-c", Copy, None),
    ]);

    // The menu bar exposes the same actions as the key bindings.
    cx.set_menus(vec![
        Menu {
            name: "Quick Check".into(),
            items: vec![
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
    ]);
}
