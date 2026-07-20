//! Platform-independent Hanji editor API.
//!
//! Native and WebAssembly adapters use [`Editor`] for every document mutation. Core documents,
//! transactions, and Markdown policy types remain implementation details.

mod command;
mod editor;
mod error;
mod input;
mod selection;
mod update;

pub use command::Command;
pub use editor::Editor;
pub use error::Error;
pub use hanji_core::{TextPosition, TextRange};
pub use input::{TextInput, TextInputMode};
pub use selection::TextSelection;
pub use update::Update;
