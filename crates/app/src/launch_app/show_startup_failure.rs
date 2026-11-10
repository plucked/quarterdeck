use std::process::Command;

/// Tells the user why the app cannot start, in a system alert: an app
/// started from Finder has no terminal for the error. Runs before gpui, so
/// the alert comes from `osascript`; `reason` is passed as an argument, never
/// as script text.
pub fn show_startup_failure(reason: &str) {
    let shown = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            "on run arguments",
            "-e",
            "display alert \"Quarterdeck cannot start\" message (item 1 of arguments) as critical",
            "-e",
            "end run",
            reason,
        ])
        .output();
    if let Err(error) = shown {
        eprintln!("cannot show the startup failure: {error}");
    }
}
