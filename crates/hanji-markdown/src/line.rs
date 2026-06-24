#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownLine {
    Blank,
    Paragraph,
    Heading { level: u8 },
    Blockquote,
}

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

pub fn classify_line(line: &str) -> MarkdownLine {
    if line.trim().is_empty() {
        return MarkdownLine::Blank;
    }

    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return MarkdownLine::Paragraph;
    }

    let content = &line[indent..];
    if content.starts_with("> ") {
        return MarkdownLine::Blockquote;
    }

    let level = content.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&level) {
        return MarkdownLine::Paragraph;
    }

    match content.as_bytes().get(level) {
        None | Some(b' ' | b'\t') => MarkdownLine::Heading { level: level as u8 },
        _ => MarkdownLine::Paragraph,
    }
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

    #[test]
    fn classifies_atx_headings() {
        assert_eq!(classify_line("# Hanji"), MarkdownLine::Heading { level: 1 });
        assert_eq!(
            classify_line("### Notes"),
            MarkdownLine::Heading { level: 3 }
        );
        assert_eq!(
            classify_line("   ## Indented"),
            MarkdownLine::Heading { level: 2 }
        );
    }

    #[test]
    fn classifies_blockquotes() {
        assert_eq!(classify_line(">"), MarkdownLine::Paragraph);
        assert_eq!(classify_line(">Quote"), MarkdownLine::Paragraph);
        assert_eq!(classify_line("> "), MarkdownLine::Blockquote);
        assert_eq!(classify_line("> Quote"), MarkdownLine::Blockquote);
        assert_eq!(classify_line("   > Indented"), MarkdownLine::Blockquote);
        assert_eq!(classify_line("    > Code"), MarkdownLine::Paragraph);
    }

    #[test]
    fn classifies_blank_and_paragraph_lines() {
        assert_eq!(classify_line(""), MarkdownLine::Blank);
        assert_eq!(classify_line("   "), MarkdownLine::Blank);
        assert_eq!(classify_line("Hanji notes"), MarkdownLine::Paragraph);
    }

    #[test]
    fn rejects_non_heading_hash_lines() {
        assert_eq!(classify_line("#Hanji"), MarkdownLine::Paragraph);
        assert_eq!(classify_line("####### Hanji"), MarkdownLine::Paragraph);
        assert_eq!(classify_line("    # Code"), MarkdownLine::Paragraph);
    }
}
