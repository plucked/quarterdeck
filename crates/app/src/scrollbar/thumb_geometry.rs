/// The shortest the thumb gets, so it stays grabbable for long content.
const MIN_THUMB_HEIGHT: f32 = 24.;

/// Where the thumb sits in its track for one scroll position, in pixels from
/// the track's top, and how a thumb position maps back to a scroll position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbGeometry {
    pub top: f32,
    pub height: f32,
    /// How far the thumb can move: track height minus thumb height.
    travel: f32,
    /// How far the content can scroll.
    max_scroll: f32,
}

impl ThumbGeometry {
    /// The thumb for content scrolled `scroll_top` pixels down out of
    /// `max_scroll`, in a viewport `viewport_height` tall, drawn in a track
    /// `track_height` tall. `None` when everything fits, so no bar is drawn.
    pub fn compute(
        track_height: f32,
        viewport_height: f32,
        max_scroll: f32,
        scroll_top: f32,
    ) -> Option<Self> {
        if max_scroll <= 0. || track_height <= 0. || viewport_height <= 0. {
            return None;
        }

        // The thumb shows the visible share of the content, but never gets too small.
        let content_height = viewport_height + max_scroll;
        let height = (track_height * viewport_height / content_height)
            .max(MIN_THUMB_HEIGHT)
            .min(track_height);
        let travel = track_height - height;
        let progress = (scroll_top / max_scroll).clamp(0., 1.);
        Some(Self {
            top: travel * progress,
            height,
            travel,
            max_scroll,
        })
    }

    /// The scroll position that puts the thumb's top at `thumb_top`, clamped to the track.
    pub fn scroll_top_for(&self, thumb_top: f32) -> f32 {
        if self.travel <= 0. {
            return 0.;
        }
        thumb_top.clamp(0., self.travel) / self.travel * self.max_scroll
    }

    /// Whether `y`, measured from the track's top, is on the thumb.
    pub fn contains(&self, y: f32) -> bool {
        self.top <= y && y <= self.top + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Content that fits draws no bar.
    #[test]
    fn no_thumb_when_content_fits() {
        assert_eq!(ThumbGeometry::compute(400., 400., 0., 0.), None);
    }

    /// The thumb's height is the visible share of the content.
    #[test]
    fn thumb_height_is_visible_share() {
        // A 400px viewport over 1600px of content shows a quarter of it.
        let thumb = ThumbGeometry::compute(400., 400., 1200., 0.).unwrap();
        assert!((thumb.height - 100.).abs() < f32::EPSILON);
        assert!(thumb.top.abs() < f32::EPSILON);
    }

    /// Scrolled to the bottom, the thumb touches the track's end.
    #[test]
    fn thumb_reaches_bottom_at_max_scroll() {
        let thumb = ThumbGeometry::compute(400., 400., 1200., 1200.).unwrap();
        assert!((thumb.top + thumb.height - 400.).abs() < f32::EPSILON);
    }

    /// Very long content keeps a grabbable thumb.
    #[test]
    fn thumb_has_minimum_height() {
        let thumb = ThumbGeometry::compute(400., 400., 1_000_000., 0.).unwrap();
        assert!((thumb.height - MIN_THUMB_HEIGHT).abs() < f32::EPSILON);
    }

    /// Mapping a thumb position back gives the scroll position, clamped to the ends.
    #[test]
    fn thumb_position_maps_back_to_scroll() {
        let thumb = ThumbGeometry::compute(400., 400., 1200., 0.).unwrap();
        // The thumb travels 300px for 1200px of scrolling.
        assert!((thumb.scroll_top_for(150.) - 600.).abs() < f32::EPSILON);
        assert!(thumb.scroll_top_for(-50.).abs() < f32::EPSILON);
        assert!((thumb.scroll_top_for(9999.) - 1200.).abs() < f32::EPSILON);
    }
}
