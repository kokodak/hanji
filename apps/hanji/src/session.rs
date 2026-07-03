use std::{
    env, io,
    path::{Path, PathBuf},
};

use hanji_storage::DocumentSession;

pub(crate) fn open_initial_session() -> io::Result<DocumentSession> {
    let Some(path) = env::args_os().nth(1).map(PathBuf::from) else {
        return Ok(new_scratch_session());
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_session_starts_empty() {
        let session = new_scratch_session();

        assert!(is_scratch_document_path(session.path()));
        assert!(session.document().text().is_empty());
    }
}
