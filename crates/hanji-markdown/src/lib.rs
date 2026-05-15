pub fn first_heading(markdown: &str) -> Option<&str> {
    markdown.lines().find_map(|line| {
        let heading = line.strip_prefix("# ")?;
        let heading = heading.trim();

        if heading.is_empty() {
            None
        } else {
            Some(heading)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_first_heading() {
        assert_eq!(
            first_heading("# Hanji\n\nCapture the thought."),
            Some("Hanji")
        );
    }
}
