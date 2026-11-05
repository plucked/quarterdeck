use std::sync::Arc;
use std::time::Duration;

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use qd_video::VideoFrame;

/// A frame of the startup movie, ready to draw, and when it is shown from
/// the start of the movie.
pub(super) struct SplashFrame {
    pub image: Arc<RenderImage>,
    pub presentation_time: Duration,
}

impl SplashFrame {
    /// Wraps a decoded frame for gpui; `None` when its pixels do not fill
    /// its size.
    pub fn from_video_frame(frame: VideoFrame) -> Option<Self> {
        let (width, height) = (frame.width(), frame.height());
        let presentation_time = frame.presentation_time();
        // gpui draws a `RenderImage`'s bytes as BGRA, the decoder's order:
        // the `RgbaImage` only carries them.
        let pixels = RgbaImage::from_raw(width, height, frame.into_bgra())?;
        Some(Self {
            image: Arc::new(RenderImage::new(vec![Frame::new(pixels)])),
            presentation_time,
        })
    }
}
