/// An opaque color from the syntax theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl SyntaxColor {
    /// The color as `0xRRGGBB`.
    pub fn to_rgb(self) -> u32 {
        u32::from(self.red) << 16 | u32::from(self.green) << 8 | u32::from(self.blue)
    }
}
