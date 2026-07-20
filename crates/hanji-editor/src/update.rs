/// Describes the observable effects of one editor operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Update {
    text_changed: bool,
    selection_changed: bool,
    history_changed: bool,
}

impl Update {
    pub(crate) const fn new(
        text_changed: bool,
        selection_changed: bool,
        history_changed: bool,
    ) -> Self {
        Self {
            text_changed,
            selection_changed,
            history_changed,
        }
    }

    pub const fn text_changed(self) -> bool {
        self.text_changed
    }

    pub const fn selection_changed(self) -> bool {
        self.selection_changed
    }

    pub const fn history_changed(self) -> bool {
        self.history_changed
    }

    pub const fn changed(self) -> bool {
        self.text_changed || self.selection_changed || self.history_changed
    }
}
