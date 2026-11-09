use super::table_role::TableRole;

/// A compact table of short Markdown cells. Every row has one cell per column
/// (checked when the studio is read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioTable {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub role: TableRole,
}
