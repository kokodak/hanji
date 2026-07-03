use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MarkdownFile {
    pub(crate) path: PathBuf,
    pub(crate) label: String,
}

pub(crate) fn markdown_files_in(root: &Path) -> io::Result<Vec<MarkdownFile>> {
    let mut files = Vec::new();

    collect_markdown_files(root, root, &mut files)?;
    files.sort_by(|left, right| {
        left.label
            .to_lowercase()
            .cmp(&right.label.to_lowercase())
            .then_with(|| left.label.cmp(&right.label))
    });

    Ok(files)
}

pub(crate) fn folder_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn collect_markdown_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<MarkdownFile>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let file_type = entry.file_type()?;
        let path = entry.path();

        if file_type.is_dir() {
            collect_markdown_files(root, &path, files)?;
        } else if file_type.is_file() && is_markdown_file(&path) {
            files.push(MarkdownFile {
                label: relative_label(root, &path),
                path,
            });
        }
    }

    Ok(())
}

pub(crate) fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn folder_label_prefers_last_path_component() {
        assert_eq!(folder_label(Path::new("/tmp/notes")), "notes");
        assert_eq!(folder_label(Path::new("/")), "/");
    }

    #[test]
    fn markdown_files_include_nested_md_files_only() {
        let temp = unique_test_dir();
        let root = temp.as_path();
        let nested = root.join("nested");

        fs::create_dir(&nested).unwrap();
        fs::write(root.join("b.txt"), "ignored").unwrap();
        fs::write(root.join("c.MD"), "C").unwrap();
        fs::write(nested.join("a.md"), "A").unwrap();

        let files = markdown_files_in(root).unwrap();
        let labels = files.into_iter().map(|file| file.label).collect::<Vec<_>>();

        assert_eq!(labels, vec!["c.MD", "nested/a.md"]);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn markdown_file_detection_requires_md_extension() {
        assert!(is_markdown_file(Path::new("note.md")));
        assert!(is_markdown_file(Path::new("note.MD")));
        assert!(!is_markdown_file(Path::new("note.markdown")));
        assert!(!is_markdown_file(Path::new("note.txt")));
        assert!(!is_markdown_file(Path::new("note")));
    }

    fn unique_test_dir() -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hanji-file-browser-test-{}-{now}",
            std::process::id()
        ));

        fs::create_dir(&path).unwrap();
        path
    }
}
