use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("binary file not supported: {0}")]
    BinaryFile(PathBuf),
    #[error("file too large: {0} ({1} bytes)")]
    TooLarge(PathBuf, u64),
    #[error("document has no path")]
    NoPath,
}
