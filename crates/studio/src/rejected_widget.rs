/// A widget the agent wrote that was left out of the studio, and why: an
/// anchor outside the diff, a malformed table. Shown to the reader so a
/// thin studio is explained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedWidget {
    /// The widget's kind and title, such as `finding "Lock order"`.
    pub widget: String,
    pub reason: String,
}
