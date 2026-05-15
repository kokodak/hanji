use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
