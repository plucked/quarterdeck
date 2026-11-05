//! Decodes video files into BGRA frames with macOS `AVFoundation` (the
//! system's decoders: H.264 in every profile, HEVC, `ProRes`). UI-free and
//! blocking: the app plays the frames.
//!
//! Files:
//! - `video_reader.rs`: `VideoReader`, the frames of a file's first video track in display order.
//! - `video_frame.rs`: `VideoFrame`, one decoded picture and when it is shown.
//! - `media_time.rs`: `media_time_to_duration`, a media timestamp as a `Duration`.

mod media_time;
mod video_frame;
mod video_reader;

pub use video_frame::VideoFrame;
pub use video_reader::VideoReader;
