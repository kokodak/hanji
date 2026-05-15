mod document;
mod selection;
mod text;
mod transaction;

pub use document::{Document, DocumentChange};
pub use selection::{Selection, SelectionError};
pub use text::{EditError, TextBuffer, TextEdit, TextRange};
pub use transaction::Transaction;
