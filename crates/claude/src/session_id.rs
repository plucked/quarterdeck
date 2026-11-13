use anyhow::{Result, ensure};

/// A Claude-generated UUID checked before it becomes a destination.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Rejects names, paths and malformed registry identities.
    pub(crate) fn parse(value: String) -> Result<Self> {
        ensure!(value.len() == 36, "Invalid Claude session ID");
        ensure!(
            value.bytes().enumerate().all(|(index, byte)| {
                match index {
                    8 | 13 | 18 | 23 => byte == b'-',
                    _ => byte.is_ascii_hexdigit(),
                }
            }),
            "Invalid Claude session ID"
        );
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
