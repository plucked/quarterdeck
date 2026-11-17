use std::fmt::Write as _;

use crate::McpError;

const CREDENTIAL_BYTES: usize = 32;

/// A fresh bearer capability generated from operating-system randomness.
///
/// Deliberately has no Debug implementation to keep credentials out of logs.
pub(crate) struct RunCredential(String);

impl RunCredential {
    /// Creates a 256-bit secret represented as lowercase hexadecimal.
    pub(crate) fn generate() -> Result<Self, McpError> {
        // Ask the operating system for independent bytes for every run.
        let mut bytes = [0_u8; CREDENTIAL_BYTES];
        getrandom::fill(&mut bytes).map_err(|error| McpError::Randomness(error.to_string()))?;

        // Hex encoding is safe in a child environment and HTTP header.
        let mut token = String::with_capacity(CREDENTIAL_BYTES * 2);
        for byte in bytes {
            write!(token, "{byte:02x}").expect("formatting into String cannot fail");
        }
        Ok(Self(token))
    }

    /// Returns the capability only for configuring the authorized child.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
