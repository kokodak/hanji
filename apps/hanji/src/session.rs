use std::{
    env, io,
    path::{Path, PathBuf},
};

use crate::file_browser::is_markdown_file;
use hanji_storage::DocumentSession;

const MARKDOWN_FILE_REQUIRED_MESSAGE: &str = "Only Markdown files can be opened.";

pub(crate) fn open_initial_session() -> io::Result<DocumentSession> {
    let Some(path) = env::args_os().nth(1).map(PathBuf::from) else {
        return Ok(new_scratch_session());
    };

    open_initial_path(path)
}

fn open_initial_path(path: PathBuf) -> io::Result<DocumentSession> {
    if path.exists() && !path.is_file() {
        return Err(markdown_file_required_error());
    }

    if !is_markdown_file(&path) {
        return Err(markdown_file_required_error());
    }

    if path.exists() {
        DocumentSession::open(path)
    } else {
        Ok(DocumentSession::new(path, ""))
    }
}

pub(crate) fn is_scratch_document_path(path: &Path) -> bool {
    path == scratch_document_path()
}

pub(crate) fn new_scratch_session() -> DocumentSession {
    DocumentSession::new(scratch_document_path(), "")
}

fn scratch_document_path() -> PathBuf {
    env::temp_dir().join("hanji-scratch.md")
}

fn markdown_file_required_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, MARKDOWN_FILE_REQUIRED_MESSAGE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn scratch_session_starts_empty() {
        let session = new_scratch_session();

        assert!(is_scratch_document_path(session.path()));
        assert!(session.editor().source().is_empty());
    }

    #[test]
    fn initial_path_allows_new_markdown_file() {
        let path = unique_test_path("md");
        let session = open_initial_path(path.clone()).unwrap();

        assert_eq!(session.path(), path);
        assert!(session.editor().source().is_empty());
    }

    #[test]
    fn initial_path_rejects_non_markdown_file() {
        let error = open_initial_path(unique_test_path("txt")).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), MARKDOWN_FILE_REQUIRED_MESSAGE);
    }

    #[test]
    fn initial_path_rejects_directory() {
        let path = unique_test_dir();
        let error = open_initial_path(path.clone()).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), MARKDOWN_FILE_REQUIRED_MESSAGE);
        fs::remove_dir_all(path).unwrap();
    }

    fn unique_test_path(extension: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        env::temp_dir().join(format!(
            "hanji-session-test-{}-{now}.{extension}",
            std::process::id()
        ))
    }

    fn unique_test_dir() -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("hanji-session-test-{}-{now}", std::process::id()));

        fs::create_dir(&path).unwrap();
        path
    }
}
