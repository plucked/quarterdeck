/// What a graph arrow means, drawn as its stroke: control solid and dark,
/// data solid in the accent color, async dashed, optional dotted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    /// A call or a decision passed on: the target runs because of it.
    Control,
    /// Data handed over, without passing control.
    Data,
    /// A message that does not wait for the receiver.
    Async,
    /// Happens only in some cases.
    Optional,
}

impl EdgeStyle {
    pub const ALL: [Self; 4] = [Self::Control, Self::Data, Self::Async, Self::Optional];

    /// The style as the legend names it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Control => "Control",
            Self::Data => "Data",
            Self::Async => "Async",
            Self::Optional => "Optional",
        }
    }
}
