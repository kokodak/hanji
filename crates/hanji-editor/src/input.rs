use crate::{TextRange, TextSelection};

/// Selects whether a replacement should run typing policy or preserve the supplied source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputMode {
    /// Interactive typing may complete, wrap, or skip Markdown markers.
    Typing,
    /// Paste and IME updates insert exactly the supplied source.
    Literal,
}

/// A platform-independent text replacement request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    mode: TextInputMode,
    text: String,
    range: Option<TextRange>,
    selection_after: Option<TextSelection>,
}

impl TextInput {
    pub fn typing(text: impl Into<String>) -> Self {
        Self::new(TextInputMode::Typing, text)
    }

    pub fn literal(text: impl Into<String>) -> Self {
        Self::new(TextInputMode::Literal, text)
    }

    fn new(mode: TextInputMode, text: impl Into<String>) -> Self {
        Self {
            mode,
            text: text.into(),
            range: None,
            selection_after: None,
        }
    }

    /// Replaces an explicit source range instead of the current selection.
    pub fn replacing(mut self, range: TextRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Sets the directional selection after the replacement is applied.
    pub fn selecting_after(mut self, selection: TextSelection) -> Self {
        self.selection_after = Some(selection);
        self
    }

    pub const fn mode(&self) -> TextInputMode {
        self.mode
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn range(&self) -> Option<TextRange> {
        self.range
    }

    pub const fn selection_after(&self) -> Option<TextSelection> {
        self.selection_after
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        TextInputMode,
        String,
        Option<TextRange>,
        Option<TextSelection>,
    ) {
        (self.mode, self.text, self.range, self.selection_after)
    }
}
