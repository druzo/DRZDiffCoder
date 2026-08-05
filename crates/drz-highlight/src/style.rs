#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Keyword,
    StringLit,
    Comment,
    Function,
    Type,
    Number,
    Constant,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyledSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}
