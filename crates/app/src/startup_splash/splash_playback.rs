use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Frames decoded before the movie starts, so a slow moment of the decoder
/// (the app loads and syncs meanwhile) is absorbed rather than seen.
const PRELOAD_FRAMES: usize = 10;
/// The time between frames until the movie's own is known: 60 fps.
const DEFAULT_FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

/// What the decoder has for the next tick.
pub enum FramePoll<F> {
    Frame(TimedFrame<F>),
    /// Nothing decoded yet; more is coming.
    Pending,
    /// Every frame was delivered (or the movie could not be read).
    Finished,
}

/// What to do on this screen refresh.
#[derive(Debug, PartialEq, Eq)]
pub enum PlaybackStep<F> {
    /// Put this frame on screen.
    Show(F),
    /// Keep the frame on screen.
    Hold,
    /// The last frame has had its time: the movie is over.
    Ended,
}

/// A decoded frame and its time in the movie.
#[derive(Debug, PartialEq, Eq)]
pub struct TimedFrame<F> {
    pub frame: F,
    pub time: Duration,
}

/// Where playback stands on the movie's clock, and when it last moved.
struct PlaybackClock {
    position: Duration,
    last_tick: Instant,
}

/// Paces the startup movie by the screen's refreshes: on each refresh it
/// advances the movie's clock by the time since the last one and picks the
/// newest frame due, counting a frame due half a frame early so the
/// refresh's own jitter never decides between two frames (the cause of
/// stutter when frames are timed by a timer racing the display). It starts
/// once a few frames are decoded, and when the decoder falls behind it
/// holds the clock at the frame on screen instead of skipping ahead later.
/// Generic over the frame so it can be tested without a window.
pub struct SplashPlayback<F> {
    /// Decoded frames not shown yet, in movie order.
    waiting: VecDeque<TimedFrame<F>>,
    is_finished: bool,
    /// `None` until playback starts.
    clock: Option<PlaybackClock>,
    /// The movie time of the frame on screen.
    shown: Option<Duration>,
    frame_interval: Duration,
}

impl<F> Default for SplashPlayback<F> {
    fn default() -> Self {
        Self {
            waiting: VecDeque::new(),
            is_finished: false,
            clock: None,
            shown: None,
            frame_interval: DEFAULT_FRAME_INTERVAL,
        }
    }
}

impl<F> SplashPlayback<F> {
    /// The step for a screen refresh at `now`, taking decoded frames from
    /// `poll` as needed (it is not called again once it says `Finished`).
    pub fn tick(
        &mut self,
        now: Instant,
        poll: &mut dyn FnMut() -> FramePoll<F>,
    ) -> PlaybackStep<F> {
        // Take in what the decoder has ready, keeping a preload's worth at most.
        while !self.is_finished && self.waiting.len() < PRELOAD_FRAMES {
            match poll() {
                FramePoll::Frame(frame) => self.waiting.push_back(frame),
                FramePoll::Pending => break,
                FramePoll::Finished => self.is_finished = true,
            }
        }

        // Move the clock on, or start it once enough is decoded.
        let position = match &mut self.clock {
            Some(clock) => {
                clock.position += now.saturating_duration_since(clock.last_tick);
                clock.last_tick = now;
                clock.position
            }
            None => match self.start(now) {
                Some(position) => position,
                None if self.is_finished => return PlaybackStep::Ended,
                None => return PlaybackStep::Hold,
            },
        };

        // Show the newest frame due; frames passed over during a stall are dropped.
        let horizon = position + self.frame_interval / 2;
        let mut due = None;
        while self
            .waiting
            .front()
            .is_some_and(|frame| frame.time <= horizon)
        {
            due = self.waiting.pop_front();
        }
        if let Some(due) = due {
            self.shown = Some(due.time);
            return PlaybackStep::Show(due.frame);
        }
        self.step_without_frame(position)
    }

    /// Starts the clock at the first frame when a preload is decoded (or
    /// the whole movie, when shorter), learning the movie's frame interval
    /// from its first two frames; the start position, or `None` while waiting.
    fn start(&mut self, now: Instant) -> Option<Duration> {
        let is_ready = self.waiting.len() >= PRELOAD_FRAMES || self.is_finished;
        let first = self.waiting.front().filter(|_| is_ready)?.time;
        if let Some(second) = self.waiting.get(1) {
            let interval = second.time.saturating_sub(first);
            if !interval.is_zero() {
                self.frame_interval = interval;
            }
        }
        self.clock = Some(PlaybackClock {
            position: first,
            last_tick: now,
        });
        Some(first)
    }

    /// The step when no new frame is due at `position`: the movie ended
    /// once its last frame had its time; a decoder behind holds the clock.
    fn step_without_frame(&mut self, position: Duration) -> PlaybackStep<F> {
        let shown_until = self.shown.unwrap_or_default() + self.frame_interval;
        match (self.waiting.is_empty(), self.is_finished) {
            (true, true) if position >= shown_until => PlaybackStep::Ended,
            // Behind: the clock waits at the frame on screen, so the next
            // frame shows a refresh after it arrives and none is skipped.
            (true, false) => {
                if let (Some(clock), Some(shown)) = (&mut self.clock, self.shown) {
                    clock.position = position.min(shown);
                }
                PlaybackStep::Hold
            }
            (true, true) | (false, _) => PlaybackStep::Hold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = DEFAULT_FRAME_INTERVAL;

    /// A decoder holding `count` frames numbered from 0, 60 fps apart, then finished.
    fn decoder(count: u32) -> impl FnMut() -> FramePoll<u32> {
        let mut next = 0;
        move || {
            if next == count {
                return FramePoll::Finished;
            }
            let frame = TimedFrame {
                frame: next,
                time: FRAME * next,
            };
            next += 1;
            FramePoll::Frame(frame)
        }
    }

    /// Ticks at `refreshes` (offsets from the start) and lists the steps.
    fn play(
        playback: &mut SplashPlayback<u32>,
        poll: &mut dyn FnMut() -> FramePoll<u32>,
        refreshes: &[Duration],
    ) -> Vec<PlaybackStep<u32>> {
        let start = Instant::now();
        refreshes
            .iter()
            .map(|offset| playback.tick(start + *offset, poll))
            .collect()
    }

    /// At one refresh per frame, every frame shows once, in order, even
    /// when refreshes come a few milliseconds early or late.
    #[test]
    fn shows_one_frame_per_refresh_despite_jitter() {
        let mut playback = SplashPlayback::default();
        let mut poll = decoder(5);
        let jitter_millis: [i32; 6] = [0, 3, -3, 4, -4, 2];
        let refreshes: Vec<Duration> = jitter_millis
            .iter()
            .zip(0..)
            .map(|(offset_millis, index)| {
                let nominal = FRAME * index;
                let shift = Duration::from_millis(u64::from(offset_millis.unsigned_abs()));
                if *offset_millis < 0 {
                    nominal.saturating_sub(shift)
                } else {
                    nominal + shift
                }
            })
            .collect();
        assert_eq!(
            play(&mut playback, &mut poll, &refreshes),
            [
                PlaybackStep::Show(0),
                PlaybackStep::Show(1),
                PlaybackStep::Show(2),
                PlaybackStep::Show(3),
                PlaybackStep::Show(4),
                PlaybackStep::Ended,
            ]
        );
    }

    /// Playback waits for a preload, and after a stall shows the newest due frame.
    #[test]
    fn waits_for_a_preload_and_catches_up_after_a_stall() {
        let mut playback = SplashPlayback::default();
        let mut polls = 0;
        let mut source = decoder(30);
        // The decoder has nothing for the first two refreshes, then everything.
        let mut poll = move || {
            polls += 1;
            if polls <= 2 {
                FramePoll::Pending
            } else {
                source()
            }
        };
        assert_eq!(
            play(
                &mut playback,
                &mut poll,
                &[Duration::ZERO, FRAME, FRAME * 2, FRAME * 3, FRAME * 7],
            ),
            [
                PlaybackStep::Hold,
                PlaybackStep::Hold,
                PlaybackStep::Show(0),
                PlaybackStep::Show(1),
                PlaybackStep::Show(5),
            ]
        );
    }

    /// A decoder that falls behind holds the clock: the late frame shows
    /// when it arrives, and the ones after it are not skipped.
    #[test]
    fn holds_while_the_decoder_is_behind() {
        let mut playback = SplashPlayback::default();
        let mut frames: VecDeque<FramePoll<u32>> = (0..10)
            .map(|index| {
                FramePoll::Frame(TimedFrame {
                    frame: index,
                    time: FRAME * index,
                })
            })
            .collect();
        let mut poll = move || frames.pop_front().unwrap_or(FramePoll::Pending);
        let start = Instant::now();
        // Ten refreshes show the ten frames.
        for index in 0..10 {
            assert_eq!(
                playback.tick(start + FRAME * index, &mut poll),
                PlaybackStep::Show(index)
            );
        }
        // Nothing decoded for five refreshes: the last frame stays.
        for index in 10..15 {
            assert_eq!(
                playback.tick(start + FRAME * index, &mut poll),
                PlaybackStep::Hold
            );
        }
        // Frame 10 arrives late and shows at once; frame 11 follows a refresh later.
        let mut late = VecDeque::from([
            FramePoll::Frame(TimedFrame {
                frame: 10,
                time: FRAME * 10,
            }),
            FramePoll::Pending,
        ]);
        let mut poll_late = move || late.pop_front().unwrap_or(FramePoll::Pending);
        assert_eq!(
            playback.tick(start + FRAME * 15, &mut poll_late),
            PlaybackStep::Show(10)
        );
        let mut next = VecDeque::from([FramePoll::Frame(TimedFrame {
            frame: 11,
            time: FRAME * 11,
        })]);
        let mut poll_next = move || next.pop_front().unwrap_or(FramePoll::Finished);
        assert_eq!(
            playback.tick(start + FRAME * 16, &mut poll_next),
            PlaybackStep::Show(11)
        );
    }

    /// A movie that cannot be read (no frames at all) ends at once.
    #[test]
    fn an_empty_movie_ends_at_once() {
        let mut playback: SplashPlayback<u32> = SplashPlayback::default();
        let mut poll = || FramePoll::Finished;
        assert_eq!(
            playback.tick(Instant::now(), &mut poll),
            PlaybackStep::Ended
        );
    }
}
