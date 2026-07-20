use std::fmt;

use hanji_core::{CommandError as CoreCommandError, EditError};
use hanji_markdown::MarkdownCommandError;

/// Errors exposed by the platform-independent editor API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A source range points outside the document or has invalid ordering.
    InvalidRange,
    /// A source offset is not on a valid grapheme boundary.
    InvalidBoundary,
    /// The requested selection cannot be represented by the editor.
    InvalidSelection,
    /// An internal edit plan violated an editor invariant.
    Internal,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRange => "invalid source range",
            Self::InvalidBoundary => "invalid text boundary",
            Self::InvalidSelection => "invalid selection",
            Self::Internal => "internal editor error",
        })
    }
}

impl std::error::Error for Error {}

impl From<EditError> for Error {
    fn from(error: EditError) -> Self {
        match error {
            EditError::InvalidRange => Self::InvalidRange,
            EditError::InvalidBoundary => Self::InvalidBoundary,
            EditError::EmptySelection => Self::InvalidSelection,
            EditError::OverlappingEdits => Self::Internal,
        }
    }
}

impl From<CoreCommandError> for Error {
    fn from(error: CoreCommandError) -> Self {
        match error {
            CoreCommandError::Edit(error) => error.into(),
            CoreCommandError::MultipleSelectionsUnsupported => Self::InvalidSelection,
        }
    }
}

impl From<MarkdownCommandError> for Error {
    fn from(error: MarkdownCommandError) -> Self {
        match error {
            MarkdownCommandError::Edit(error) => error.into(),
            MarkdownCommandError::MultipleSelectionsUnsupported => Self::InvalidSelection,
        }
    }
}
