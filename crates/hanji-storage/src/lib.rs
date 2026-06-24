use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hanji_core::{CommandError, Document, DocumentChange, EditorCommand, Selection, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSession {
    path: PathBuf,
    document: Document,
    saved_text: String,
    saved_revision: u64,
    revision: u64,
}

impl DocumentSession {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let text = read_markdown(&path)?;

        Ok(Self::new(path, text))
    }

    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        let text = text.into();

        Self {
            path: path.into(),
            document: Document::new(text.clone()),
            saved_text: text,
            saved_revision: 0,
            revision: 0,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn is_dirty(&self) -> bool {
        self.document.text() != self.saved_text
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn execute(&mut self, command: EditorCommand) -> Result<bool, CommandError> {
        let changed = self.document.execute(command)?;
        self.mark_changed_if(changed);
        Ok(changed)
    }

    pub fn edit_document<E>(
        &mut self,
        edit: impl FnOnce(&mut Document) -> Result<bool, E>,
    ) -> Result<bool, E> {
        let before_text = self.document.text().to_owned();
        let changed = edit(&mut self.document)?;
        self.mark_changed_if(changed && self.document.text() != before_text);
        Ok(changed)
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), hanji_core::EditError> {
        self.document.set_selection(selection)
    }

    pub fn apply(
        &mut self,
        transaction: Transaction,
    ) -> Result<DocumentChange, hanji_core::EditError> {
        let before_text = self.document.text().to_owned();
        let change = self.document.apply(transaction)?;
        self.mark_changed_if(self.document.text() != before_text);
        Ok(change)
    }

    pub fn undo(&mut self) -> Option<DocumentChange> {
        let before_text = self.document.text().to_owned();
        let change = self.document.undo()?;
        self.mark_changed_if(self.document.text() != before_text);
        Some(change)
    }

    pub fn redo(&mut self) -> Option<DocumentChange> {
        let before_text = self.document.text().to_owned();
        let change = self.document.redo()?;
        self.mark_changed_if(self.document.text() != before_text);
        Some(change)
    }

    pub fn save(&mut self) -> io::Result<()> {
        let text = self.document.text().to_owned();

        write_markdown(&self.path, &text)?;
        self.saved_text = text;
        self.saved_revision = self.revision;
        Ok(())
    }

    fn mark_changed_if(&mut self, changed: bool) {
        if changed {
            self.mark_changed();
        }
    }

    fn mark_changed(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

pub fn read_markdown(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path)
}

pub fn write_markdown(path: impl AsRef<Path>, text: &str) -> io::Result<()> {
    write_atomic(path, text.as_bytes())
}

pub fn write_atomic(path: impl AsRef<Path>, bytes: &[u8]) -> io::Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp_path = unique_temp_path(path)?;

    let result = write_temp_and_rename(path, &temp_path, bytes);

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result?;
    sync_directory(parent);

    Ok(())
}

fn write_temp_and_rename(path: &Path, temp_path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)?;

    temp_file.write_all(bytes)?;
    temp_file.sync_all()?;
    drop(temp_file);

    fs::rename(temp_path, path)
}

fn unique_temp_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic writes require a file path",
        )
    })?;

    let file_name = file_name.to_string_lossy();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    Ok(parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        timestamp
    )))
}

fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_markdown() {
        let directory = TestDirectory::new("read-write");
        let path = directory.path().join("note.md");

        write_markdown(&path, "# Hanji\n").unwrap();

        assert_eq!(read_markdown(&path).unwrap(), "# Hanji\n");
    }

    #[test]
    fn replaces_existing_file_atomically() {
        let directory = TestDirectory::new("replace");
        let path = directory.path().join("note.md");

        fs::write(&path, "before").unwrap();
        write_markdown(&path, "after").unwrap();

        assert_eq!(read_markdown(&path).unwrap(), "after");
    }

    #[test]
    fn removes_temp_file_when_write_fails() {
        let directory = TestDirectory::new("cleanup");
        let missing_parent = directory.path().join("missing");
        let path = missing_parent.join("note.md");

        let error = write_markdown(&path, "content").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn rejects_directory_path() {
        let directory = TestDirectory::new("directory-path");

        let error = write_markdown(directory.path(), "content").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::IsADirectory);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn opens_document_session_from_disk() {
        let directory = TestDirectory::new("session-open");
        let path = directory.path().join("note.md");
        fs::write(&path, "# Hanji\n").unwrap();

        let session = DocumentSession::open(&path).unwrap();

        assert_eq!(session.path(), path.as_path());
        assert_eq!(session.document().text(), "# Hanji\n");
        assert!(!session.is_dirty());
    }

    #[test]
    fn session_tracks_dirty_state_after_command() {
        let directory = TestDirectory::new("session-dirty");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        let changed = session
            .execute(EditorCommand::insert_text(" notes"))
            .unwrap();

        assert!(changed);
        assert!(session.is_dirty());
        assert_eq!(session.revision(), 1);
        assert_eq!(session.document().text(), " notesHanji");
    }

    #[test]
    fn session_does_not_mark_noop_command_dirty() {
        let directory = TestDirectory::new("session-noop");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        let changed = session.execute(EditorCommand::DeleteBackward).unwrap();

        assert!(!changed);
        assert!(!session.is_dirty());
        assert_eq!(session.revision(), 0);
    }

    #[test]
    fn session_tracks_dirty_state_after_external_document_edit() {
        let directory = TestDirectory::new("session-external-edit");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        let changed = session
            .edit_document(|document| document.execute(EditorCommand::insert_text(" notes")))
            .unwrap();

        assert!(changed);
        assert!(session.is_dirty());
        assert_eq!(session.revision(), 1);
        assert_eq!(session.document().text(), " notesHanji");
    }

    #[test]
    fn session_does_not_mark_selection_only_external_document_edit_dirty() {
        let directory = TestDirectory::new("session-external-selection");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        let changed = session
            .edit_document(|document| {
                document.set_selection(Selection::caret(5))?;
                Ok::<_, hanji_core::EditError>(true)
            })
            .unwrap();

        assert!(changed);
        assert!(!session.is_dirty());
        assert_eq!(session.revision(), 0);
        assert_eq!(
            session.document().selection().primary(),
            hanji_core::TextRange::caret(5)
        );
    }

    #[test]
    fn session_save_clears_dirty_state_and_writes_file() {
        let directory = TestDirectory::new("session-save");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        session
            .execute(EditorCommand::insert_text(" notes"))
            .unwrap();
        session.save().unwrap();

        assert!(!session.is_dirty());
        assert_eq!(read_markdown(&path).unwrap(), " notesHanji");
    }

    #[test]
    fn session_undo_back_to_saved_text_clears_dirty_state() {
        let directory = TestDirectory::new("session-undo-clean");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        session
            .execute(EditorCommand::insert_text(" notes"))
            .unwrap();
        assert!(session.is_dirty());

        session.undo().unwrap();

        assert!(!session.is_dirty());
        assert_eq!(session.document().text(), "Hanji");
    }

    #[test]
    fn session_does_not_mark_selection_only_transaction_dirty() {
        let directory = TestDirectory::new("session-selection");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        session
            .apply(Transaction::new(
                Vec::new(),
                Some(hanji_core::Selection::caret(5)),
            ))
            .unwrap();

        assert!(!session.is_dirty());
        assert_eq!(session.revision(), 0);
        assert_eq!(
            session.document().selection().primary(),
            hanji_core::TextRange::caret(5)
        );
    }

    #[test]
    fn session_selection_change_is_not_undoable_or_dirty() {
        let directory = TestDirectory::new("session-selection-setter");
        let path = directory.path().join("note.md");
        let mut session = DocumentSession::new(&path, "Hanji");

        session
            .set_selection(hanji_core::Selection::caret(5))
            .unwrap();

        assert!(!session.is_dirty());
        assert!(!session.document().can_undo());
        assert_eq!(
            session.document().selection().primary(),
            hanji_core::TextRange::caret(5)
        );
    }

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("hanji-storage-{}-{}", name, std::process::id()));

            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();

            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
