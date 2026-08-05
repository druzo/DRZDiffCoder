#[derive(Debug, thiserror::Error)]
pub enum HighlightError {
    #[error("unsupported language")]
    UnsupportedLanguage,
    #[error("parse failed")]
    ParseFailed,
    #[error("query failed: {0}")]
    QueryFailed(String),
}
