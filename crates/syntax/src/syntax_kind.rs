/// What a highlighted token is. The app picks its color from the active
/// color theme when it draws, so changing the theme needs no new
/// highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Comment,
    /// Keywords and storage words (`fn`, `let`, `pub`, `u8`).
    Keyword,
    String,
    /// Numbers and named constants (`true`, `None`).
    Constant,
    /// Function and method names, where defined and where called.
    Definition,
    Type,
    /// Code the grammar marks as invalid.
    Invalid,
}

impl SyntaxKind {
    pub const ALL: [Self; 7] = [
        Self::Comment,
        Self::Keyword,
        Self::String,
        Self::Constant,
        Self::Definition,
        Self::Type,
        Self::Invalid,
    ];
}
