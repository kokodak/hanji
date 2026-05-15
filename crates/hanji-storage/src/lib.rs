use std::fs;
use std::io;
use std::path::Path;

pub fn read_markdown(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path)
}

pub fn write_markdown(path: impl AsRef<Path>, text: &str) -> io::Result<()> {
    fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_markdown() {
        let path = std::env::temp_dir().join(format!("hanji-storage-{}.md", std::process::id()));

        write_markdown(&path, "# Hanji\n").unwrap();

        assert_eq!(read_markdown(&path).unwrap(), "# Hanji\n");

        fs::remove_file(path).unwrap();
    }
}
