/// The page of a brief a reader had open when they reported it, so a report
/// that says "here" can be traced to the widgets they were looking at.
pub struct ShownPage {
    /// The page's place in the brief, counted from 1.
    pub number: usize,
    /// How many pages the brief has.
    pub count: usize,
    /// The page's tab title.
    pub title: String,
}
