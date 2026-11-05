use std::time::Duration;

/// One decoded picture of a video, made by `VideoReader`: `height` rows of
/// `width` pixels, 4 bytes each in B, G, R, A order, with no padding between
/// rows. `presentation_time` is when it is shown, from the start of the
/// video.
pub struct VideoFrame {
    width: u32,
    height: u32,
    bgra: Vec<u8>,
    presentation_time: Duration,
}

impl VideoFrame {
    /// A frame of `bgra` pixels; the caller guarantees `width * height * 4` bytes.
    pub(crate) fn new(width: u32, height: u32, bgra: Vec<u8>, presentation_time: Duration) -> Self {
        Self {
            width,
            height,
            bgra,
            presentation_time,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// When the frame is shown, from the start of the video.
    pub fn presentation_time(&self) -> Duration {
        self.presentation_time
    }

    /// The pixels, BGRA, rows packed without padding.
    pub fn into_bgra(self) -> Vec<u8> {
        self.bgra
    }
}
