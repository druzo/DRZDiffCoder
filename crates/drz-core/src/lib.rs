pub mod diff;
pub mod document;
pub mod edit;
pub mod error;
pub mod io;

pub use diff::{diff_lines, Hunk};
pub use document::Document;
pub use edit::TextEdit;
pub use error::CoreError;
