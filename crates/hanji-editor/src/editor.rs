use hanji_core::{
    Document, EditError, EditorCommand as CoreCommand, Selection, TextEdit, Transaction,
};
use hanji_markdown::{
    ListIndentDirection, MarkdownCommand, MarkdownLine, MarkdownProjection,
    blockquote_newline_edit_for_line, execute_markdown_command, list_indent_edit,
    list_newline_edit_for_line, marker_autocomplete_edit, marker_skip_offset, project_document,
    table_cell_at_offset, table_cell_line_start_for_offset, table_line_break_delete_edit,
    table_newline_edit, task_marker_state_char_range,
};

use crate::{
    Command, Error, TextInput, TextInputMode, TextPosition, TextRange, TextSelection, Update,
};

/// The standard, platform-independent Hanji editor.
#[derive(Debug)]
pub struct Editor {
    document: Document,
    selection: TextSelection,
}

impl Editor {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            document: Document::new(source),
            selection: TextSelection::caret(0),
        }
    }

    pub fn source(&self) -> &str {
        self.document.text()
    }

    pub fn len(&self) -> usize {
        self.document.len()
    }

    pub fn is_empty(&self) -> bool {
        self.document.is_empty()
    }

    pub fn selected_source(&self) -> Option<&str> {
        let range = self.selection.range();
        if range.is_empty() {
            return None;
        }

        let projection = project_document(&self.document);
        let mut expanded = range.start..range.end;
        if let Some(cell) = projection
            .lines()
            .iter()
            .flat_map(|line| line.table_cells())
            .find(|cell| {
                cell.content_range.start == range.start && range.end >= cell.content_range.end
            })
        {
            expanded.start = cell.source_outer_range.start;
        }
        if let Some(cell) = projection
            .lines()
            .iter()
            .flat_map(|line| line.table_cells())
            .rev()
            .find(|cell| {
                cell.content_range.end == range.end && range.start <= cell.content_range.start
            })
        {
            expanded.end = cell.source_outer_range.end;
        }

        self.source().get(expanded)
    }

    pub const fn selection(&self) -> TextSelection {
        self.selection
    }

    pub fn set_selection(&mut self, selection: TextSelection) -> Result<Update, Error> {
        let before = self.state();
        self.document
            .set_selection(Selection::single(selection.range()))?;
        self.selection = selection;
        Ok(self.update_since(before))
    }

    pub fn replace_text(&mut self, input: TextInput) -> Result<Update, Error> {
        let before = self.state();
        let (mode, text, range, selection_after) = input.into_parts();
        let range = range.unwrap_or_else(|| self.selection.range());
        let source_range = range.start..range.end;

        if matches!(mode, TextInputMode::Typing) {
            if let Some(offset) = marker_skip_offset(self.source(), &source_range, &text) {
                self.document.set_selection(Selection::caret(offset))?;
                self.selection = TextSelection::caret(offset);
                return Ok(self.update_since(before));
            }

            if let Some((range, replacement, selection_after)) =
                marker_autocomplete_edit(self.source(), &source_range, &text)
            {
                self.apply_replacement(range, replacement, selection_after)?;
                return Ok(self.update_since(before));
            }
        }

        let selection_after = selection_after.unwrap_or_else(|| {
            let caret = range.start + text.len();
            TextSelection::caret(caret)
        });
        self.apply_replacement(
            source_range,
            text,
            selection_after.anchor()..selection_after.head(),
        )?;

        Ok(self.update_since(before))
    }

    pub fn execute(&mut self, command: Command) -> Result<Update, Error> {
        let before = self.state();

        match command {
            Command::DeleteBackward => self.delete_backward()?,
            Command::DeleteWordBackward => self.delete_word_backward()?,
            Command::DeleteLineBackward => self.delete_line_backward()?,
            Command::DeleteForward => self.delete_forward()?,
            Command::InsertNewline => self.insert_newline()?,
            Command::Indent => self.change_list_indent(ListIndentDirection::Increase)?,
            Command::Outdent => self.change_list_indent(ListIndentDirection::Decrease)?,
            Command::ToggleStrong => {
                execute_markdown_command(&mut self.document, MarkdownCommand::ToggleStrong)?;
            }
            Command::ToggleEmphasis => {
                execute_markdown_command(&mut self.document, MarkdownCommand::ToggleEmphasis)?;
            }
            Command::ToggleCode => {
                execute_markdown_command(&mut self.document, MarkdownCommand::ToggleCode)?;
            }
            Command::InsertLink => {
                execute_markdown_command(&mut self.document, MarkdownCommand::InsertLink)?;
            }
            Command::ToggleTaskAt(offset) => self.toggle_task_at(offset)?,
            Command::Undo => {
                self.document.undo();
            }
            Command::Redo => {
                self.document.redo();
            }
        }

        self.sync_selection_from_document();
        Ok(self.update_since(before))
    }

    pub fn projection(&self) -> MarkdownProjection<'_> {
        project_document(&self.document)
    }

    pub fn can_undo(&self) -> bool {
        self.document.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.document.can_redo()
    }

    pub fn line_count(&self) -> usize {
        self.document.line_count()
    }

    pub fn line_range(&self, line_index: usize) -> Option<TextRange> {
        self.document.line_range(line_index)
    }

    pub fn line_index_at_offset(&self, offset: usize) -> Result<usize, Error> {
        self.document
            .line_index_at_offset(offset)
            .map_err(Into::into)
    }

    pub fn position_at_offset(&self, offset: usize) -> Result<TextPosition, Error> {
        self.document.position_at_offset(offset).map_err(Into::into)
    }

    pub fn offset_at_position(&self, position: TextPosition) -> Result<usize, Error> {
        self.document
            .offset_at_position(position)
            .map_err(Into::into)
    }

    pub fn nearest_grapheme_offset(&self, offset: usize) -> Result<usize, Error> {
        self.document
            .nearest_grapheme_offset(offset)
            .map_err(Into::into)
    }

    pub fn previous_grapheme_offset(&self, offset: usize) -> Result<Option<usize>, Error> {
        self.document
            .previous_grapheme_offset(offset)
            .map_err(Into::into)
    }

    pub fn next_grapheme_offset(&self, offset: usize) -> Result<Option<usize>, Error> {
        self.document
            .next_grapheme_offset(offset)
            .map_err(Into::into)
    }

    pub fn previous_word_offset(&self, offset: usize) -> Result<Option<usize>, Error> {
        self.document
            .previous_word_offset(offset)
            .map_err(Into::into)
    }

    pub fn next_word_offset(&self, offset: usize) -> Result<Option<usize>, Error> {
        self.document.next_word_offset(offset).map_err(Into::into)
    }

    pub fn word_range_at_offset(&self, offset: usize) -> Result<Option<TextRange>, Error> {
        self.document
            .word_range_at_offset(offset)
            .map_err(Into::into)
    }

    fn delete_backward(&mut self) -> Result<(), Error> {
        let range = self.selection.range();
        let source_range = range.start..range.end;
        let cell = self.table_cell_at_offset(range.start);

        if range.is_empty() && cell.is_some_and(|cell| range.start <= cell.content_range.start) {
            return Ok(());
        }
        if cell.is_some()
            && let Some((range, replacement, selection_after)) =
                table_line_break_delete_edit(self.source(), &source_range, -1)
        {
            self.apply_replacement(range, replacement, selection_after)?;
            return Ok(());
        }
        if let Some((range, replacement, selection_after)) =
            hanji_markdown::empty_marker_pair_delete_backward_edit(self.source(), &source_range)
        {
            self.apply_replacement(range, replacement, selection_after)?;
            return Ok(());
        }

        self.document.execute(CoreCommand::DeleteBackward)?;
        Ok(())
    }

    fn delete_word_backward(&mut self) -> Result<(), Error> {
        let range = self.selection.range();
        if range.is_empty()
            && let Some(cell) = self.table_cell_at_offset(range.start)
        {
            let boundary = self
                .document
                .previous_word_offset(range.start)?
                .map_or(cell.content_range.start, |offset| {
                    offset.max(cell.content_range.start)
                });
            if boundary < range.start {
                self.apply_replacement(boundary..range.start, String::new(), boundary..boundary)?;
            }
            return Ok(());
        }

        self.document.execute(CoreCommand::DeleteWordBackward)?;
        Ok(())
    }

    fn delete_line_backward(&mut self) -> Result<(), Error> {
        let range = self.selection.range();
        if range.is_empty()
            && let Some(cell) = self.table_cell_at_offset(range.start)
        {
            let boundary = table_cell_line_start_for_offset(self.source(), cell, range.start);
            if boundary < range.start {
                self.apply_replacement(boundary..range.start, String::new(), boundary..boundary)?;
            }
            return Ok(());
        }

        self.document.execute(CoreCommand::DeleteLineBackward)?;
        Ok(())
    }

    fn delete_forward(&mut self) -> Result<(), Error> {
        let range = self.selection.range();
        let source_range = range.start..range.end;
        let cell = self.table_cell_at_offset(range.end);

        if range.is_empty() && cell.is_some_and(|cell| range.end >= cell.content_range.end) {
            return Ok(());
        }
        if cell.is_some()
            && let Some((range, replacement, selection_after)) =
                table_line_break_delete_edit(self.source(), &source_range, 1)
        {
            self.apply_replacement(range, replacement, selection_after)?;
            return Ok(());
        }

        self.document.execute(CoreCommand::DeleteForward)?;
        Ok(())
    }

    fn insert_newline(&mut self) -> Result<(), Error> {
        let range = self.selection.range();
        let source_range = range.start..range.end;

        let table_edit = {
            let projection = project_document(&self.document);
            projection
                .lines()
                .iter()
                .find(|line| line.range.start <= range.start && range.end <= line.range.end)
                .and_then(|line| table_newline_edit(line.table_cells(), &source_range))
        };
        if let Some((range, replacement, selection_after)) = table_edit {
            self.apply_replacement(range, replacement, selection_after)?;
            return Ok(());
        }

        let line_index = self.document.line_index_at_offset(range.start)?;
        let line_range = self
            .document
            .line_range(line_index)
            .ok_or(EditError::InvalidRange)?;
        let line_source = self
            .source()
            .get(line_range.start..line_range.end)
            .ok_or(EditError::InvalidRange)?;

        if let Some((range, replacement, selection_after)) =
            blockquote_newline_edit_for_line(line_source, line_range, &source_range)
                .or_else(|| list_newline_edit_for_line(line_source, line_range, &source_range))
        {
            self.apply_replacement(range, replacement, selection_after)?;
            return Ok(());
        }

        let caret = range.start + "\n".len();
        self.apply_replacement(source_range, "\n".to_string(), caret..caret)?;
        Ok(())
    }

    fn table_cell_at_offset(&self, offset: usize) -> Option<hanji_markdown::ProjectedTableCell> {
        let projection = project_document(&self.document);
        let cells = projection
            .lines()
            .iter()
            .flat_map(|line| line.table_cells().iter().copied())
            .collect::<Vec<_>>();

        table_cell_at_offset(&cells, offset)
    }

    fn change_list_indent(&mut self, direction: ListIndentDirection) -> Result<(), Error> {
        let range = self.selection.range();
        let source_range = range.start..range.end;
        let Some((edits, selection_after)) =
            list_indent_edit(self.source(), &source_range, direction)
        else {
            return Ok(());
        };
        let selection_after =
            Selection::single(TextRange::new(selection_after.start, selection_after.end));
        self.document
            .apply(Transaction::new(edits, Some(selection_after)))?;
        Ok(())
    }

    fn toggle_task_at(&mut self, offset: usize) -> Result<(), Error> {
        let marker = {
            let projection = project_document(&self.document);
            projection
                .lines()
                .iter()
                .find(|line| {
                    line.range.start <= offset
                        && offset <= line.range.end
                        && matches!(line.kind, MarkdownLine::ListItem { task: Some(_), .. })
                })
                .and_then(|line| line.marker_range)
        };
        let Some(marker) = marker else {
            return Ok(());
        };
        let state_range =
            task_marker_state_char_range(self.source(), marker).ok_or(Error::Internal)?;
        let replacement = match self.source().get(state_range.clone()) {
            Some(" ") => "x",
            Some("x" | "X") => " ",
            _ => return Err(Error::Internal),
        };
        let selection_after = self.document.selection().clone();
        self.document.apply(Transaction::new(
            vec![TextEdit::replace(
                TextRange::new(state_range.start, state_range.end),
                replacement,
            )],
            Some(selection_after),
        ))?;
        Ok(())
    }

    fn apply_replacement(
        &mut self,
        range: std::ops::Range<usize>,
        replacement: String,
        selection_after: std::ops::Range<usize>,
    ) -> Result<(), Error> {
        let selection = TextSelection::new(selection_after.start, selection_after.end);
        self.document.apply(
            Transaction::replace(TextRange::new(range.start, range.end), replacement)
                .with_selection_after(Selection::single(selection.range())),
        )?;
        self.selection = selection;
        Ok(())
    }

    fn sync_selection_from_document(&mut self) {
        let range = self.document.selection().primary();
        self.selection = TextSelection::new(range.start, range.end);
    }

    fn state(&self) -> State {
        State {
            source: self.source().to_string(),
            selection: self.selection,
        }
    }

    fn update_since(&self, before: State) -> Update {
        let text_changed = self.source() != before.source;
        Update::new(
            text_changed,
            self.selection != before.selection,
            text_changed,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    source: String,
    selection: TextSelection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_text_and_markdown_commands_through_one_facade() {
        let mut editor = Editor::new("Hanji notes");
        editor.set_selection(TextSelection::new(6, 11)).unwrap();

        let update = editor.execute(Command::ToggleStrong).unwrap();

        assert!(update.text_changed());
        assert_eq!(editor.source(), "Hanji **notes**");
        assert_eq!(editor.selection(), TextSelection::new(8, 13));
    }

    #[test]
    fn distinguishes_typing_policy_from_literal_input() {
        let mut typing = Editor::new("*");
        typing.set_selection(TextSelection::caret(1)).unwrap();
        typing.replace_text(TextInput::typing("*")).unwrap();

        let mut literal = Editor::new("*");
        literal.set_selection(TextSelection::caret(1)).unwrap();
        literal.replace_text(TextInput::literal("*")).unwrap();

        assert_eq!(typing.source(), "****");
        assert_eq!(typing.selection(), TextSelection::caret(2));
        assert_eq!(literal.source(), "**");
        assert_eq!(literal.selection(), TextSelection::caret(2));
    }

    #[test]
    fn inserts_markdown_aware_newlines() {
        let mut editor = Editor::new("- item");
        editor.set_selection(TextSelection::caret(6)).unwrap();

        editor.execute(Command::InsertNewline).unwrap();

        assert_eq!(editor.source(), "- item\n- ");
        assert_eq!(editor.selection(), TextSelection::caret(9));
    }

    #[test]
    fn preserves_directional_selection_until_an_edit_resolves_it() {
        let mut editor = Editor::new("Hanji");

        editor.set_selection(TextSelection::new(5, 0)).unwrap();

        assert!(editor.selection().is_reversed());
        assert_eq!(editor.selection().range(), TextRange::new(0, 5));
    }

    #[test]
    fn exposes_projection_and_history_without_core_document_access() {
        let mut editor = Editor::new("# Hanji");
        editor.set_selection(TextSelection::caret(7)).unwrap();
        editor.replace_text(TextInput::typing(" editor")).unwrap();

        assert_eq!(
            editor.projection().lines()[0].visible_text(),
            "Hanji editor"
        );
        assert!(editor.can_undo());

        editor.execute(Command::Undo).unwrap();
        assert_eq!(editor.source(), "# Hanji");
        assert!(editor.can_redo());
    }

    #[test]
    fn keeps_deletion_inside_a_table_cell() {
        let mut editor = Editor::new("| Hanji | Ready |\n| --- | --- |");
        let cell = editor.projection().lines()[0].table_cells()[0];
        editor
            .set_selection(TextSelection::caret(cell.content_range.start))
            .unwrap();

        let update = editor.execute(Command::DeleteBackward).unwrap();

        assert!(!update.text_changed());
        assert_eq!(editor.source(), "| Hanji | Ready |\n| --- | --- |");
    }

    #[test]
    fn deletes_a_table_line_break_as_one_unit() {
        let mut editor = Editor::new("| Hanji<br>Editor | Ready |\n| --- | --- |");
        let caret = editor.source().find("<br>").unwrap() + "<br>".len();
        editor.set_selection(TextSelection::caret(caret)).unwrap();

        editor.execute(Command::DeleteBackward).unwrap();

        assert_eq!(editor.source(), "| HanjiEditor | Ready |\n| --- | --- |");
        assert_eq!(
            editor.selection(),
            TextSelection::caret(caret - "<br>".len())
        );
    }

    #[test]
    fn toggles_a_task_from_a_source_position() {
        let mut editor = Editor::new("- [ ] Task");

        editor.execute(Command::ToggleTaskAt(3)).unwrap();

        assert_eq!(editor.source(), "- [x] Task");
    }

    #[test]
    fn copies_complete_markdown_source_for_selected_table_cells() {
        let source = "| Name | Status |\n| --- | --- |\n| Hanji | Ready |";
        let mut editor = Editor::new(source);
        let projection = editor.projection();
        let first_cell = projection.lines()[0].table_cells()[0];
        let last_cell = projection.lines()[2].table_cells()[1];
        editor
            .set_selection(TextSelection::new(
                first_cell.content_range.start,
                last_cell.content_range.end,
            ))
            .unwrap();

        assert_eq!(editor.selected_source(), Some(source));
    }

    #[test]
    fn copies_only_partial_table_cell_source() {
        let mut editor = Editor::new("| Hanji | Ready |\n| --- | --- |");
        let cell = editor.projection().lines()[0].table_cells()[0];
        editor
            .set_selection(TextSelection::new(
                cell.content_range.start + 1,
                cell.content_range.end,
            ))
            .unwrap();

        assert_eq!(editor.selected_source(), Some("anji"));
    }

    #[test]
    fn maps_internal_errors_to_editor_owned_errors() {
        let mut editor = Editor::new("한지");

        assert_eq!(
            editor.set_selection(TextSelection::caret(1)),
            Err(Error::InvalidBoundary)
        );
        assert_eq!(
            editor.set_selection(TextSelection::caret(99)),
            Err(Error::InvalidRange)
        );
    }
}
