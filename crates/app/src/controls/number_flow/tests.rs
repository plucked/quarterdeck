use super::super::value_motion::DURATION;
use super::*;

/// Fixed-width cells make the outgoing units column's position explicit.
fn cells(label: &str) -> Vec<Cell> {
    parts(label)
        .into_iter()
        .enumerate()
        .map(|(index, part)| Cell {
            key: part.key,
            text: label[part.range].to_owned(),
            wheel: part.digit.map(f32::from),
            x: f32::from(u16::try_from(index).unwrap()) * 8.,
            y: 0.,
            width: 8.,
            opacity: 1.,
        })
        .collect()
}

fn bounds(width: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(50.), px(20.)), size(px(width), px(16.)))
}

#[test]
fn shrinking_counter_keeps_its_outgoing_units_column_visible() {
    let now = Instant::now();
    let mut transition = Transition::default();
    transition.update("+100", cells("+100"), now, true);
    transition.update("+99", cells("+99"), now, true);

    let target = bounds(24.);
    let first = animation_bounds(target, &transition.frame(now), px(16.));
    assert_eq!(first, bounds(32.));
    let middle = animation_bounds(target, &transition.frame(now + DURATION / 4), px(16.));
    assert!(middle.right() > target.right());
    assert!(middle.right() < first.right());
    assert_eq!(
        animation_bounds(target, &transition.frame(now + DURATION), px(16.)),
        target
    );

    // Ancestor clipping still wins, as it does in Window::with_content_mask.
    let parent = ContentMask {
        bounds: bounds(28.),
    };
    assert_eq!(
        ContentMask { bounds: first }.intersect(&parent).bounds,
        parent.bounds
    );
}

#[test]
fn interrupted_shrink_preserves_visible_extent_until_reduced_motion_settles() {
    let now = Instant::now();
    let mut transition = Transition::default();
    transition.update("+1000", cells("+1000"), now, true);
    transition.update("+99", cells("+99"), now, true);
    let interrupted = now + DURATION / 4;
    let before = animation_bounds(bounds(24.), &transition.frame(interrupted), px(16.));

    transition.update("+9", cells("+9"), interrupted, true);
    let target = bounds(16.);
    assert_eq!(
        animation_bounds(target, &transition.frame(interrupted), px(16.)),
        before
    );

    transition.update("+9", cells("+9"), interrupted, false);
    assert!(!transition.is_active(interrupted));
    assert_eq!(
        animation_bounds(target, &transition.frame(interrupted), px(16.)),
        target
    );
}
