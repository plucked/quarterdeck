//! `count_as_f32`: a small count as a float, for layout arithmetic.

/// `count` as an `f32`, exact for counts up to `u16::MAX` and clamped
/// there: the counts this is for are small (characters of a label, lanes
/// of a diagram, digits of a line number, checks of a pull request), and a
/// float holds every `u16` exactly, where a plain `as` cast would round or
/// wrap silently.
pub fn count_as_f32<Count: TryInto<u16>>(count: Count) -> f32 {
    count.try_into().map_or(f32::from(u16::MAX), f32::from)
}
