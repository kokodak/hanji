use std::{
    env, io,
    path::{Path, PathBuf},
};

use hanji_storage::DocumentSession;

const SAMPLE_DOCUMENT: &str = "# Hanji\n\nCapture the **thought** with `code`.";

pub(crate) fn open_initial_session() -> io::Result<DocumentSession> {
    let Some(path) = env::args_os().nth(1).map(PathBuf::from) else {
        return Ok(DocumentSession::new(
            scratch_document_path(),
            SAMPLE_DOCUMENT,
        ));
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

fn scratch_document_path() -> PathBuf {
    env::temp_dir().join("hanji-scratch.md")
}
