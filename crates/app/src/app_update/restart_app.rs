//! `restart_app`: quits the app and starts it again.

use gpui::App;
use qd_update::relaunch_after_exit;

/// Arranges for the app to start again once this process has exited (its
/// bundle, which an update may have replaced meanwhile), then quits. When
/// the relaunch cannot be arranged the app keeps running and the reason is
/// returned, so a user is never left without the app.
pub fn restart_app(cx: &mut App) -> std::io::Result<()> {
    relaunch_after_exit()?;
    cx.quit();
    Ok(())
}
