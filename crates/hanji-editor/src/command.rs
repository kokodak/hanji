/// A logical, platform-independent editor command.
///
/// Text insertion is intentionally absent. Platform text events must use
/// [`TextInput`](crate::TextInput) so typing policy cannot be bypassed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    DeleteBackward,
    DeleteWordBackward,
    DeleteLineBackward,
    DeleteForward,
    InsertNewline,
    Indent,
    Outdent,
    ToggleStrong,
    ToggleEmphasis,
    ToggleCode,
    InsertLink,
    ToggleTaskAt(usize),
    Undo,
    Redo,
}
