// AVFoundation and Core Video are Objective-C and C APIs: every call into
// them is `unsafe`. This file is the workspace's one exception to
// `unsafe_code = "deny"`; each block states why it is sound.
#![allow(unsafe_code)]

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2_av_foundation::{
    AVAssetReader, AVAssetReaderStatus, AVAssetReaderTrackOutput, AVMediaTypeVideo, AVURLAsset,
};
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::{
    CVPixelBuffer, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetHeight, CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelBufferPixelFormatTypeKey,
    kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use crate::media_time::media_time_to_duration;
use crate::video_frame::VideoFrame;

const BYTES_PER_PIXEL: usize = 4;

/// Reads the first video track of a movie file (MP4, MOV) frame by frame,
/// decoded by the system (hardware where it can) into BGRA, in display
/// order. Blocking; it stays on the thread that opened it (the Objective-C
/// objects it holds are not `Send`).
pub struct VideoReader {
    reader: Retained<AVAssetReader>,
    output: Retained<AVAssetReaderTrackOutput>,
}

impl VideoReader {
    /// Opens the file at `path` and starts decoding its first video track.
    pub fn open(path: &Path) -> Result<Self> {
        let path = path
            .to_str()
            .with_context(|| format!("{} is not UTF-8", path.display()))?;
        autoreleasepool(|_| {
            // SAFETY: plain object constructors and getters on objects created
            // here; `AVMediaTypeVideo` is a framework constant, present on
            // every macOS version that has `AVAssetReader`.
            unsafe {
                // The file's first video track.
                let url = NSURL::fileURLWithPath(&NSString::from_str(path));
                let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
                let media_type = AVMediaTypeVideo.context("AVFoundation lacks the video type")?;
                // The synchronous form: its successor reports through a completion
                // block, and this reader blocks anyway.
                #[allow(deprecated)]
                let tracks = asset.tracksWithMediaType(media_type);
                let track = tracks
                    .firstObject()
                    .with_context(|| format!("{path} has no video track"))?;

                // A reader that decodes it into BGRA pixel buffers.
                let reader = AVAssetReader::assetReaderWithAsset_error(&asset)
                    .map_err(|error| anyhow::anyhow!("{}", error.localizedDescription()))
                    .with_context(|| format!("cannot read {path}"))?;
                let output =
                    AVAssetReaderTrackOutput::assetReaderTrackOutputWithTrack_outputSettings(
                        &track,
                        Some(&bgra_output_settings()),
                    );
                // The frames are copied out at once, so the decoder's own buffers can be handed over.
                output.setAlwaysCopiesSampleData(false);
                reader.addOutput(&output);
                if !reader.startReading() {
                    bail!("cannot start reading {path}");
                }
                Ok(Self { reader, output })
            }
        })
    }

    /// The next frame in display order; `None` once the track has ended.
    pub fn next_frame(&mut self) -> Result<Option<VideoFrame>> {
        autoreleasepool(|_| {
            loop {
                // SAFETY: the output was added to the reader, which started
                // reading in `open`; that is all `copyNextSampleBuffer` needs.
                let Some(sample) = (unsafe { self.output.copyNextSampleBuffer() }) else {
                    return self.end_of_track();
                };
                // Marker-only samples carry no picture: skip them.
                if let Some(frame) = frame_of_sample(&sample)? {
                    return Ok(Some(frame));
                }
            }
        })
    }

    /// What running out of samples means: the end of the track, or a
    /// decoding failure.
    fn end_of_track(&self) -> Result<Option<VideoFrame>> {
        // SAFETY: getters on a live reader.
        unsafe {
            match self.reader.status() {
                AVAssetReaderStatus::Failed => {
                    let reason = self.reader.error().map_or_else(
                        || "unknown error".to_owned(),
                        |error| error.localizedDescription().to_string(),
                    );
                    bail!("decoding failed: {reason}")
                }
                _ => Ok(None),
            }
        }
    }
}

/// Output settings asking the decoder for 32-bit BGRA pixel buffers.
fn bgra_output_settings() -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: a framework constant, initialized before any code runs; it is
    // a `CFString`, toll-free bridged to `NSString`.
    let key: &NSString = unsafe { kCVPixelBufferPixelFormatTypeKey }.as_ref();
    let format = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
    NSDictionary::from_slices(&[key], &[format.as_ref()])
}

/// The picture of a decoded sample and its time; `None` for a sample
/// without one.
fn frame_of_sample(sample: &CMSampleBuffer) -> Result<Option<VideoFrame>> {
    // SAFETY: getters on a valid sample buffer, which owns its image buffer
    // for as long as `sample` is borrowed.
    let (time, image_buffer) = unsafe { (sample.presentation_time_stamp(), sample.image_buffer()) };
    let Some(image_buffer) = image_buffer else {
        return Ok(None);
    };
    let presentation_time =
        media_time_to_duration(time.value, time.timescale).context("invalid frame time")?;
    let (width, height, bgra) = copy_bgra_pixels(&image_buffer)?;
    Ok(Some(VideoFrame::new(
        width,
        height,
        bgra,
        presentation_time,
    )))
}

/// Copies a BGRA pixel buffer's rows into one packed buffer (the decoder
/// may pad each row): width, height and pixels.
fn copy_bgra_pixels(pixel_buffer: &CVPixelBuffer) -> Result<(u32, u32, Vec<u8>)> {
    let width = CVPixelBufferGetWidth(pixel_buffer);
    let height = CVPixelBufferGetHeight(pixel_buffer);
    let packed_row = width * BYTES_PER_PIXEL;

    // The pixels may only be read while the base address is locked.
    // SAFETY: locking a valid pixel buffer for reading; unlocked below on every path.
    let locked =
        unsafe { CVPixelBufferLockBaseAddress(pixel_buffer, CVPixelBufferLockFlags::ReadOnly) };
    if locked != kCVReturnSuccess {
        bail!("cannot lock the frame's pixels ({locked})");
    }
    let base = CVPixelBufferGetBaseAddress(pixel_buffer).cast::<u8>();
    let bytes_per_row = CVPixelBufferGetBytesPerRow(pixel_buffer);
    let copied = if base.is_null() || bytes_per_row < packed_row {
        None
    } else {
        // SAFETY: while locked, `base` points at `height` rows of
        // `bytes_per_row` bytes each, and the buffer outlives this borrow.
        let pixels = unsafe { std::slice::from_raw_parts(base, bytes_per_row * height) };
        // One copy per row (a memcpy), never per byte: a frame is over a
        // megabyte, and a byte-wise copy is far too slow in a debug build.
        let mut packed = Vec::with_capacity(packed_row * height);
        for row in pixels.chunks_exact(bytes_per_row) {
            packed.extend_from_slice(&row[..packed_row]);
        }
        Some(packed)
    };
    // SAFETY: balances the lock above with the same flags.
    unsafe { CVPixelBufferUnlockBaseAddress(pixel_buffer, CVPixelBufferLockFlags::ReadOnly) };

    let bgra = copied.context("the frame is not a packed BGRA picture")?;
    Ok((u32::try_from(width)?, u32::try_from(height)?, bgra))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file that is not a movie (a Git LFS pointer, say) fails to open
    /// instead of yielding frames.
    #[test]
    fn rejects_a_file_that_is_no_movie() {
        let path = std::env::temp_dir().join(format!("qd-video-test-{}.mp4", std::process::id()));
        std::fs::write(&path, "version https://git-lfs.github.com/spec/v1\n").unwrap();
        let opened = VideoReader::open(&path);
        std::fs::remove_file(&path).ok();
        assert!(opened.is_err());
    }

    /// The app's intro movie (H.264 High profile with B-frames, 3 s at
    /// 60 fps) decodes into 180 frames, in display order. Skipped while the
    /// file is only a Git LFS pointer (`git lfs pull` fetches it).
    #[test]
    fn decodes_the_intro_in_display_order() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/assets/startup-intro.mp4");
        if std::fs::read(&path)
            .unwrap()
            .starts_with(b"version https://git-lfs")
        {
            return;
        }
        let mut reader = VideoReader::open(&path).unwrap();
        let mut times = Vec::new();
        while let Some(frame) = reader.next_frame().unwrap() {
            assert_eq!((frame.width(), frame.height()), (1000, 300));
            times.push(frame.presentation_time());
            assert_eq!(frame.into_bgra().len(), 1000 * 300 * BYTES_PER_PIXEL);
        }
        assert_eq!(times.len(), 180);
        assert!(times.is_sorted_by(|earlier, later| earlier < later));
    }
}
