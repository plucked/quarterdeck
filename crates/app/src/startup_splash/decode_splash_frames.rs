use std::path::PathBuf;

use futures::SinkExt as _;
use futures::channel::mpsc;
use qd_video::VideoReader;

use super::splash_frame::SplashFrame;

/// How many decoded frames may wait in the channel (each is 1.2 MB for a
/// 1000×300 movie), on top of the playback's preload: about half a second
/// ahead, enough to ride out a busy moment of a loaded machine, without
/// decoding the whole movie into memory.
const FRAMES_AHEAD: usize = 20;

/// Decodes the movie at `path` into a channel of frames, in display order.
/// The channel ends after the last frame, or early when the movie cannot be
/// read (the reason goes to stderr); dropping the receiver stops the
/// decoding.
///
/// Decoding runs on a thread of its own rather than the background
/// executor: the reader holds Objective-C objects that must stay on one
/// thread, and a task may resume on another after each await.
pub(super) fn decode_splash_frames(path: PathBuf) -> mpsc::Receiver<SplashFrame> {
    let (mut sender, receiver) = mpsc::channel(FRAMES_AHEAD);
    let spawned = std::thread::Builder::new()
        .name("startup-splash-video".into())
        .spawn(move || {
            let mut reader = match VideoReader::open(&path) {
                Ok(reader) => reader,
                Err(error) => {
                    eprintln!("startup movie unavailable: {error:#}");
                    return;
                }
            };
            loop {
                match reader.next_frame() {
                    Ok(Some(frame)) => {
                        let Some(frame) = SplashFrame::from_video_frame(frame) else {
                            eprintln!("startup movie: a frame does not match its size");
                            return;
                        };
                        // Waits while the channel is full; fails once the splash is gone.
                        if futures::executor::block_on(sender.send(frame)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => return,
                    Err(error) => {
                        eprintln!("startup movie: {error:#}");
                        return;
                    }
                }
            }
        });
    // Without the thread the sender is dropped, so the channel ends at once.
    if let Err(error) = spawned {
        eprintln!("cannot start decoding the startup movie: {error}");
    }
    receiver
}
