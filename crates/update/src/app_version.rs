use std::fmt;

use crate::UpdateError;

/// A release's version, `major.minor.patch` as in `[workspace.package]`;
/// compared part by part, so `0.10.0` is newer than `0.9.3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl AppVersion {
    /// The version of the running app, set by Cargo from the workspace.
    ///
    /// # Panics
    /// Never in a build: the workspace version is checked by the tests.
    pub fn current() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).expect("the workspace version is major.minor.patch")
    }

    /// Reads `1.2.3` (a leading `v`, as in a tag, is allowed).
    pub fn parse(text: &str) -> Result<Self, UpdateError> {
        let invalid = || UpdateError::InvalidVersion(text.to_owned());
        let mut parts = text.trim().trim_start_matches('v').split('.');
        let mut next_part = || -> Result<u32, UpdateError> {
            parts
                .next()
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())
        };
        let version = Self {
            major: next_part()?,
            minor: next_part()?,
            patch: next_part()?,
        };
        match parts.next() {
            None => Ok(version),
            Some(_) => Err(invalid()),
        }
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Versions compare by number, not as text.
    #[test]
    fn compares_numerically() {
        let older = AppVersion::parse("0.9.3").unwrap();
        let newer = AppVersion::parse("v0.10.0").unwrap();
        assert!(newer > older);
        assert_eq!(newer.to_string(), "0.10.0");
    }

    /// Anything but three numbers is refused.
    #[test]
    fn refuses_malformed_versions() {
        for text in ["", "1.2", "1.2.3.4", "1.x.3", "1.2.-3"] {
            assert!(AppVersion::parse(text).is_err(), "{text}");
        }
    }

    /// The running app's own version parses.
    #[test]
    fn current_version_parses() {
        assert_eq!(AppVersion::current().to_string(), env!("CARGO_PKG_VERSION"));
    }
}
