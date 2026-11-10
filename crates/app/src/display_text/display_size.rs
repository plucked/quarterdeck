//! `byte_size`: a byte count as people read it (`812 KB`, `151.7 MB`).

/// Bytes in a kilobyte, megabyte and gigabyte (powers of 1024, as Finder's
/// older convention; the cache is read in the same units throughout).
const KILOBYTE: f64 = 1024.;
const MEGABYTE: f64 = KILOBYTE * 1024.;
const GIGABYTE: f64 = MEGABYTE * 1024.;

/// `bytes` in the largest unit that keeps it at least 1, with one decimal
/// from megabytes up.
pub fn byte_size(bytes: u64) -> String {
    let value = f64::from(u32::try_from(bytes / 1024).unwrap_or(u32::MAX)) * KILOBYTE
        + f64::from(u32::try_from(bytes % 1024).unwrap_or(0));
    match bytes {
        0..1024 => format!("{bytes} bytes"),
        1024..1_048_576 => format!("{:.0} KB", value / KILOBYTE),
        1_048_576..1_073_741_824 => format!("{:.1} MB", value / MEGABYTE),
        _ => format!("{:.1} GB", value / GIGABYTE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each size reads in its own unit.
    #[test]
    fn picks_the_unit() {
        assert_eq!(byte_size(0), "0 bytes");
        assert_eq!(byte_size(812 * 1024), "812 KB");
        assert_eq!(byte_size(151_660_127), "144.6 MB");
        assert_eq!(byte_size(3 * 1_073_741_824 / 2), "1.5 GB");
    }
}
