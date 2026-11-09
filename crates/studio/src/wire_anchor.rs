use serde::Deserialize;

use super::anchor_targets::AnchorTargets;
use super::code_anchor::CodeAnchor;
use super::code_side::CodeSide;

/// An anchor as the agent writes it.
#[derive(Deserialize)]
pub(crate) struct WireAnchor {
    path: String,
    side: WireSide,
    start_line: i64,
    end_line: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireSide {
    Base,
    Head,
}

impl WireAnchor {
    /// The anchor cut to its first line, if the diff shows that line: for
    /// links to where something starts, whatever range was written.
    pub(crate) fn checked_start(mut self, targets: &AnchorTargets) -> Result<CodeAnchor, String> {
        self.end_line = self.start_line;
        self.checked(targets)
    }

    /// The anchor in the studio's terms, if its lines are numbers the diff shows.
    pub(crate) fn checked(self, targets: &AnchorTargets) -> Result<CodeAnchor, String> {
        let line = |number: i64| {
            u32::try_from(number).map_err(|_| format!("{}: line {number} is not a line", self.path))
        };
        let checked = CodeAnchor {
            first_line: line(self.start_line)?,
            last_line: line(self.end_line)?,
            side: match self.side {
                WireSide::Base => CodeSide::Base,
                WireSide::Head => CodeSide::Head,
            },
            path: self.path,
        };
        targets.check(&checked)?;
        Ok(checked)
    }
}
