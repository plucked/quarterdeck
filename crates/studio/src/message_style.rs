/// What a sequence message is: a call that waits (solid arrow), the reply
/// to one (dashed), or a message sent without waiting (open arrowhead).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStyle {
    Call,
    Reply,
    Async,
}

impl MessageStyle {
    pub const ALL: [Self; 3] = [Self::Call, Self::Reply, Self::Async];

    /// The style as the legend names it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Call => "Call (waits)",
            Self::Reply => "Reply",
            Self::Async => "Async (no wait)",
        }
    }
}
