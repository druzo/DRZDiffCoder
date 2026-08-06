pub use drz_highlight::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

/// Per-line change status from a diff, document-line indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStatus {
    #[default]
    Unchanged,
    Added,
    Removed,
}
