# qd-video

| Module | Holds |
|---|---|
| `video_reader` | `VideoReader`: `open(path)`, `next_frame()`: a movie's first video track decoded by AVFoundation (`AVAssetReader` → 32-bit BGRA pixel buffers) in display order |
| `video_frame` | `VideoFrame`: width, height, packed BGRA rows, presentation time |
| `media_time` | `media_time_to_duration`: Core Media `value / timescale` → `Duration` |

- macOS only: decoding is the system's (hardware where it can), so any codec macOS plays works, H.264 High profile with B-frames included (the pure-Rust openh264 decoder rejects those).
- `video_reader.rs` is the workspace's one file allowing `unsafe` (`#![allow(unsafe_code)]`): the Objective-C and Core Video calls. Every block has a `SAFETY:` comment; keep `unsafe` out of every other file.
- Blocking, and not `Send` (it holds Objective-C objects): open and read on one thread. Each call runs in its own autorelease pool, so a plain `std::thread` is fine.
- Rows are copied with one `extend_from_slice` each: workspace crates build unoptimized in debug, and a byte-wise iterator copy (1.2 MB per frame) stalled the splash after its first frame.
- Decoding the app's 3 s intro (180 frames, 1000×300) takes ~0.2 s.
