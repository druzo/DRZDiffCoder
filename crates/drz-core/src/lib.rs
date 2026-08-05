pub mod align;
pub mod diff;
pub mod document;
pub mod edit;
pub mod error;
pub mod inline;
pub mod io;

pub use align::{build_alignment, Alignment};
pub use diff::{diff_lines, Hunk};
pub use document::{Document, NewlinePolicy};
pub use edit::TextEdit;
pub use error::CoreError;
pub use inline::inline_diff_ranges;
