use crate::parse_patch::hunk_starts;

/// A numbered line of the original patch, including its unmodified diff marker and text.
pub struct RawPatchLine<'a> {
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
    pub source: &'a str,
    pub(crate) hunk: usize,
}

impl<'a> RawPatchLine<'a> {
    /// Walks each hunk once, retaining the source spelling and coordinates before display folding.
    pub fn in_patch(patch: &'a str) -> impl Iterator<Item = Self> {
        let mut hunk = 0;
        let mut numbers = None;
        patch.lines().filter_map(move |source| {
            // Numbering restarts at each header. Ignore unknown input until a valid header.
            if source.starts_with("@@") {
                hunk += 1;
                numbers = hunk_starts(source);
                return None;
            }
            let (old, new) = numbers?;
            let (old_number, new_number) = match source.as_bytes().first() {
                Some(b' ') => (Some(old), Some(new)),
                Some(b'-') => (Some(old), None),
                Some(b'+') => (None, Some(new)),
                Some(b'\\') => return None,
                _ => {
                    numbers = None;
                    return None;
                }
            };
            // Context advances both sides. A change advances only the side carrying the line.
            numbers = old
                .checked_add(u32::from(old_number.is_some()))
                .zip(new.checked_add(u32::from(new_number.is_some())));
            Some(Self {
                old_number,
                new_number,
                source,
                hunk,
            })
        })
    }
}
