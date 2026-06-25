use std::{
    io,
    process::{Command, ExitStatus},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExternalUrlCommand {
    program: &'static str,
    args: Vec<String>,
}

impl ExternalUrlCommand {
    pub(crate) fn status(&self) -> io::Result<ExitStatus> {
        Command::new(self.program).args(&self.args).status()
    }
}

pub(crate) fn external_url_command(url: &str) -> Option<ExternalUrlCommand> {
    if !is_supported_external_url(url) {
        return None;
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        Some(platform_open_command(url))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        None
    }
}

pub(crate) fn is_supported_external_url(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once(':') else {
        return false;
    };

    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") && rest.starts_with("//")
}

#[cfg(target_os = "macos")]
fn platform_open_command(url: &str) -> ExternalUrlCommand {
    ExternalUrlCommand {
        program: "open",
        args: vec![url.to_string()],
    }
}

#[cfg(target_os = "linux")]
fn platform_open_command(url: &str) -> ExternalUrlCommand {
    ExternalUrlCommand {
        program: "xdg-open",
        args: vec![url.to_string()],
    }
}

#[cfg(target_os = "windows")]
fn platform_open_command(url: &str) -> ExternalUrlCommand {
    ExternalUrlCommand {
        program: "cmd",
        args: vec![
            "/C".to_string(),
            "start".to_string(),
            "".to_string(),
            url.to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_http_and_https_urls() {
        assert!(is_supported_external_url("https://hanji.local"));
        assert!(is_supported_external_url("http://hanji.local"));
        assert!(!is_supported_external_url("file:///tmp/note.md"));
        assert!(!is_supported_external_url("javascript:alert(1)"));
        assert!(!is_supported_external_url("mailto:hello@example.com"));
        assert!(!is_supported_external_url("hanji.local"));
    }

    #[test]
    fn builds_platform_open_command_for_supported_urls() {
        let command = external_url_command("https://hanji.local").expect("command");

        #[cfg(target_os = "macos")]
        {
            assert_eq!(command.program, "open");
            assert_eq!(command.args, vec!["https://hanji.local"]);
        }

        #[cfg(target_os = "linux")]
        {
            assert_eq!(command.program, "xdg-open");
            assert_eq!(command.args, vec!["https://hanji.local"]);
        }

        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.program, "cmd");
            assert_eq!(command.args, vec!["/C", "start", "", "https://hanji.local"]);
        }
    }
}
