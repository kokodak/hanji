//! Syntax-agnostic text editing primitives for Hanji.
//!
//! This crate owns source buffers, selections, transactions, undo history, plain-text commands,
//! and offset conversion. Markdown policy and platform types belong in higher-level crates.

mod command;
mod document;
mod encoding;
mod selection;
mod text;
mod transaction;

pub use command::{CommandError, EditorCommand};
pub use document::{Document, DocumentChange};
pub use encoding::{byte_offset_to_utf16, utf16_offset_to_byte, utf16_range_to_byte};
pub use selection::{Selection, SelectionError};
pub use text::{EditError, TextBuffer, TextEdit, TextPosition, TextRange};
pub use transaction::Transaction;
