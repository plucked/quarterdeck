use std::path::{Path, PathBuf};

/// The movie's file name, in the bundle's `Contents/Resources` and in the
/// app crate's `assets` folder (tracked by Git LFS).
const STARTUP_VIDEO_FILE: &str = "startup-intro.mp4";

/// Where the startup movie is: in `Quarterdeck.app/Contents/Resources` when
/// the app runs from its bundle, else in the source tree it was built from
/// (`cargo run`); `None` when neither has it, and the app starts without a
/// splash.
pub fn startup_video_path() -> Option<PathBuf> {
    bundled_video_path()
        .filter(|path| path.is_file())
        .or_else(|| {
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(STARTUP_VIDEO_FILE);
            source.is_file().then_some(source)
        })
}

/// The movie's place in the app bundle: the executable is
/// `Contents/MacOS/Quarterdeck`, resources sit in `Contents/Resources`.
fn bundled_video_path() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let contents = executable.parent()?.parent()?;
    Some(contents.join("Resources").join(STARTUP_VIDEO_FILE))
}
