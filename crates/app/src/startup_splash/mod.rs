//! The intro shown at launch: a borderless window playing the startup
//! movie while the main window, still hidden, loads and syncs with GitHub;
//! when the movie ends (or on a click) the splash fades out as the main
//! window fades in.
//!
//! Files:
//! - `startup_splash.rs`: `StartupSplash`, the splash window: playback, fade-out, revealing the main window.
//! - `decode_splash_frames.rs`: decodes the movie on its own thread into a channel of frames.
//! - `splash_frame.rs`: `SplashFrame`, a decoded frame ready to draw and its time.
//! - `splash_playback.rs`: `SplashPlayback`, which frame to show on each screen refresh.
//! - `startup_video_path.rs`: where the movie is (in the app bundle, else the source tree).

mod decode_splash_frames;
mod splash_frame;
mod splash_playback;
mod startup_splash;
mod startup_video_path;

pub use startup_splash::{STARTUP_FADE, StartupSplash};
pub use startup_video_path::startup_video_path;
