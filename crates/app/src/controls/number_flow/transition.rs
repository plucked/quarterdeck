//! The digit wheel and interruption model, independent of the renderer.

use std::{cmp::Ordering, ops::Range, time::Instant};

use super::super::value_motion::{DURATION, eased_progress};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Key {
    Digit { run: usize, place: i32 },
    Text { run: usize, fragment: usize },
    Separator { run: usize, place: i32 },
}

#[derive(Clone, Debug)]
pub(super) struct Part {
    pub key: Key,
    pub range: Range<usize>,
    pub digit: Option<u8>,
}

/// Formatted numeric spans, including decimal points and grouping commas.
fn numbers(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len()
            && (bytes[index].is_ascii_digit()
                || (matches!(bytes[index], b'.' | b',')
                    && bytes.get(index + 1).is_some_and(u8::is_ascii_digit)))
        {
            index += 1;
        }
        result.push(start..index);
    }
    result
}

/// Stable place values: integer digits align at the units column,
/// fractions at the decimal point. Grouping punctuation gets its own slot.
pub(super) fn parts(text: &str) -> Vec<Part> {
    let mut result = Vec::new();
    let mut end = 0;
    let ranges = numbers(text);
    for (run, range) in ranges.iter().enumerate() {
        if end < range.start {
            result.push(Part {
                key: Key::Text { run, fragment: 0 },
                range: end..range.start,
                digit: None,
            });
        }
        let number = &text[range.clone()];
        let integer = number.split('.').next().unwrap_or(number);
        let mut place =
            i32::try_from(integer.bytes().filter(u8::is_ascii_digit).count()).unwrap_or(i32::MAX);
        for (offset, byte) in number.bytes().enumerate() {
            let index = range.start + offset;
            let digit = byte.is_ascii_digit().then(|| byte - b'0');
            let key = if digit.is_some() {
                place -= 1;
                Key::Digit { run, place }
            } else {
                Key::Separator { run, place }
            };
            result.push(Part {
                key,
                range: index..index + 1,
                digit,
            });
        }
        end = range.end;
    }
    if end < text.len() {
        result.push(Part {
            key: Key::Text {
                run: ranges.len(),
                fragment: 0,
            },
            range: end..text.len(),
            digit: None,
        });
    }
    result
}

/// Compare formatted values without rounding large identifiers through a
/// float. All wheels in one number share its direction, including carries
/// across the decimal point. Separate counts have independent trends.
fn trend(before: &str, after: &str, run: usize) -> Ordering {
    let number = |text: &str| {
        let Some(range) = numbers(text).get(run).cloned() else {
            return (false, String::new(), String::new());
        };
        let negative = text[..range.start].ends_with(['-', '−']);
        let value = text[range].replace(',', "");
        let (integer, fraction) = value.split_once('.').unwrap_or((&value, ""));
        (
            negative,
            integer.trim_start_matches('0').to_owned(),
            fraction.trim_end_matches('0').to_owned(),
        )
    };
    let (before_negative, before_integer, before_fraction) = number(before);
    let (after_negative, after_integer, after_fraction) = number(after);
    let magnitude = after_integer
        .len()
        .cmp(&before_integer.len())
        .then_with(|| after_integer.cmp(&before_integer))
        .then_with(|| after_fraction.cmp(&before_fraction));
    match (before_negative, after_negative) {
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        (true, true) => magnitude.reverse(),
        (false, false) => magnitude,
    }
}

#[derive(Clone, Debug)]
pub(super) struct Cell {
    pub key: Key,
    pub text: String,
    /// Unbounded wheel position: 9 -> 10 draws 9 -> 0, not 9 -> 0 backwards.
    pub wheel: Option<f32>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub opacity: f32,
}

#[derive(Default)]
pub(super) struct Transition {
    label: String,
    from: Vec<Cell>,
    to: Vec<Cell>,
    started: Option<Instant>,
}

impl Transition {
    /// A new value starts from the actual in-flight frame, even when
    /// another update arrives before the preceding roll has finished.
    pub fn update(&mut self, label: &str, mut target: Vec<Cell>, now: Instant, animated: bool) {
        if !animated || self.to.is_empty() {
            self.settle(label, target);
            return;
        }
        if self.label == label {
            if !self
                .to
                .iter()
                .filter(|cell| cell.opacity > 0.)
                .map(|cell| &cell.key)
                .eq(target.iter().map(|cell| &cell.key))
            {
                self.settle(label, target);
                return;
            }
            // Layout changes (resizing, scrolling) are not value changes.
            for (old, new) in self.to.iter_mut().zip(target) {
                old.x = new.x;
                old.y = new.y;
                old.width = new.width;
            }
            if !self.is_active(now) {
                self.started = None;
                self.from.clear();
            }
            return;
        }
        let current = self.frame(now);
        let mut from = Vec::new();
        for next in &mut target {
            let previous = current.iter().find(|cell| {
                cell.key == next.key && (cell.wheel.is_some() || cell.text == next.text)
            });
            if let Some(previous) = previous {
                if let (Some(value), Some(digit), Key::Digit { run, .. }) =
                    (previous.wheel, next.wheel, &next.key)
                {
                    next.wheel = Some(destination(value, digit, trend(&self.label, label, *run)));
                }
                from.push(previous.clone());
            } else {
                from.push(Cell {
                    opacity: 0.,
                    ..next.clone()
                });
            }
        }
        // Lost columns and changed words fade out, rather than vanishing.
        for previous in current.into_iter().filter(|cell| cell.opacity > 0.001) {
            if !target.iter().any(|next| {
                next.key == previous.key && (next.wheel.is_some() || next.text == previous.text)
            }) {
                target.push(Cell {
                    opacity: 0.,
                    ..previous.clone()
                });
                from.push(previous);
            }
        }
        label.clone_into(&mut self.label);
        self.from = from;
        self.to = target;
        self.started = Some(now);
    }

    fn settle(&mut self, label: &str, target: Vec<Cell>) {
        label.clone_into(&mut self.label);
        self.to = target;
        self.from.clear();
        self.started = None;
    }

    pub fn is_active(&self, now: Instant) -> bool {
        self.started
            .is_some_and(|start| now.saturating_duration_since(start) < DURATION)
    }

    pub fn frame(&self, now: Instant) -> Vec<Cell> {
        let Some(start) = self.started else {
            return self.to.clone();
        };
        let progress = eased_progress(start, now);
        self.from
            .iter()
            .zip(&self.to)
            .map(|(from, to)| {
                let mix = |a, b| a + (b - a) * progress;
                Cell {
                    key: to.key.clone(),
                    text: to.text.clone(),
                    wheel: from.wheel.zip(to.wheel).map(|(a, b)| mix(a, b)),
                    x: mix(from.x, to.x),
                    y: mix(from.y, to.y),
                    width: mix(from.width, to.width),
                    opacity: mix(from.opacity, to.opacity),
                }
            })
            .collect()
    }
}

/// Normalizing negative zero matters when a decreasing reel crosses 0:
/// `rem_euclid` may return -0.0, which must never paint an extra minus sign.
pub(super) fn face(wheel: f32, step: u8) -> String {
    let digit = (wheel.floor() + f32::from(step)).rem_euclid(10.).abs();
    format!("{digit:.0}")
}

fn destination(from: f32, digit: f32, trend: Ordering) -> f32 {
    let base = (from / 10.).floor() * 10. + digit;
    match trend {
        Ordering::Greater if base < from => base + 10.,
        Ordering::Less if base > from => base - 10.,
        _ => base,
    }
}

#[cfg(test)]
mod tests;
