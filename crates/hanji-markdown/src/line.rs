#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownLine {
    Blank,
    Paragraph,
    Heading {
        level: u8,
    },
    Blockquote,
    CodeBlock {
        role: MarkdownCodeBlockLine,
    },
    ListItem {
        marker: MarkdownListMarker,
        task: Option<MarkdownTaskState>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCodeBlockLine {
    OpeningFence,
    Content,
    ClosingFence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownListMarker {
    Unordered {
        marker: char,
    },
    Ordered {
        number: u64,
        delimiter: OrderedListDelimiter,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderedListDelimiter {
    Dot,
    Paren,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownTaskState {
    Unchecked,
    Checked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownListItem {
    pub marker: MarkdownListMarker,
    pub task: Option<MarkdownTaskState>,
    pub content_start: usize,
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

    if blockquote_content_start(line).is_some() {
        return MarkdownLine::Blockquote;
    }

    if let Some(list_item) = list_item(line) {
        return MarkdownLine::ListItem {
            marker: list_item.marker,
            task: list_item.task,
        };
    }

    classify_non_blockquote_line(line)
}

pub fn blockquote_content_start(line: &str) -> Option<usize> {
    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return None;
    }

    let content = &line[indent..];
    if content.starts_with("> ") {
        Some(indent + "> ".len())
    } else {
        None
    }
}

pub fn heading_content_start(line: &str) -> Option<usize> {
    heading_level_and_content_start(line).map(|(_, content_start)| content_start)
}

pub fn list_item_content_start(line: &str) -> Option<usize> {
    list_item(line).map(|list_item| list_item.content_start)
}

pub fn list_item(line: &str) -> Option<MarkdownListItem> {
    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return None;
    }

    let content = &line[indent..];

    unordered_list_item(content, indent).or_else(|| ordered_list_item(content, indent))
}

fn unordered_list_item(content: &str, indent: usize) -> Option<MarkdownListItem> {
    let marker = match content.as_bytes().first()? {
        b'-' => '-',
        b'*' => '*',
        b'+' => '+',
        _ => return None,
    };
    let padding = marker_padding_len(content, 1)?;
    let content_start = indent + 1 + padding;
    let content_start_without_indent = content_start - indent;
    if pending_task_marker_without_space(content, content_start_without_indent) {
        return None;
    }
    let (task, content_start) = task_marker_content_start(content, content_start - indent);

    Some(MarkdownListItem {
        marker: MarkdownListMarker::Unordered { marker },
        task,
        content_start: indent + content_start,
    })
}

fn ordered_list_item(content: &str, indent: usize) -> Option<MarkdownListItem> {
    let bytes = content.as_bytes();
    let digit_len = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if !(1..=9).contains(&digit_len) {
        return None;
    }

    let delimiter = match bytes.get(digit_len)? {
        b'.' => OrderedListDelimiter::Dot,
        b')' => OrderedListDelimiter::Paren,
        _ => return None,
    };
    let padding = marker_padding_len(content, digit_len + 1)?;
    let number = content[..digit_len].parse().ok()?;
    let content_start = digit_len + 1 + padding;
    if pending_task_marker_without_space(content, content_start) {
        return None;
    }
    let (task, content_start) = task_marker_content_start(content, content_start);

    Some(MarkdownListItem {
        marker: MarkdownListMarker::Ordered { number, delimiter },
        task,
        content_start: indent + content_start,
    })
}

fn marker_padding_len(source: &str, marker_end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if !matches!(bytes.get(marker_end), Some(b' ' | b'\t')) {
        return None;
    }

    let padding = bytes[marker_end..]
        .iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();

    Some(padding)
}

fn task_marker_content_start(
    source: &str,
    content_start: usize,
) -> (Option<MarkdownTaskState>, usize) {
    let Some((state, task_marker_len)) = task_marker(source, content_start) else {
        return (None, content_start);
    };

    (Some(state), content_start + task_marker_len)
}

fn pending_task_marker_without_space(source: &str, start: usize) -> bool {
    matches!(
        source.as_bytes().get(start..start + 3),
        Some([b'[', b' ', b']'] | [b'[', b'x' | b'X', b']'])
    ) && source.as_bytes().get(start + 3).is_none()
}

fn task_marker(source: &str, start: usize) -> Option<(MarkdownTaskState, usize)> {
    let bytes = source.as_bytes();
    let state = match bytes.get(start..start + 3)? {
        [b'[', b' ', b']'] => MarkdownTaskState::Unchecked,
        [b'[', b'x' | b'X', b']'] => MarkdownTaskState::Checked,
        _ => return None,
    };

    let marker_end = start + 3;
    let padding = match bytes.get(marker_end) {
        Some(b' ' | b'\t') => bytes[marker_end..]
            .iter()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count(),
        None | Some(_) => return None,
    };

    Some((state, 3 + padding))
}

fn classify_non_blockquote_line(line: &str) -> MarkdownLine {
    if let Some((level, _)) = heading_level_and_content_start(line) {
        return MarkdownLine::Heading { level };
    }

    MarkdownLine::Paragraph
}

fn heading_level_and_content_start(line: &str) -> Option<(u8, usize)> {
    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ')
        .take(4)
        .count();
    if indent >= 4 {
        return None;
    }

    let content = &line[indent..];

    let level = content.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }

    match content.as_bytes().get(level) {
        Some(b' ' | b'\t') => {
            let padding = content.as_bytes()[level..]
                .iter()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count();

            Some((level as u8, indent + level + padding))
        }
        None | Some(_) => None,
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
        assert_eq!(classify_line("# "), MarkdownLine::Heading { level: 1 });
        assert_eq!(classify_line("#"), MarkdownLine::Paragraph);
        assert_eq!(classify_line("###"), MarkdownLine::Paragraph);
    }

    #[test]
    fn finds_heading_content_start() {
        assert_eq!(heading_content_start("# Hanji"), Some(2));
        assert_eq!(heading_content_start("###   Notes"), Some(6));
        assert_eq!(heading_content_start("   ## Indented"), Some(6));
        assert_eq!(heading_content_start("# "), Some(2));
        assert_eq!(heading_content_start("#"), None);
        assert_eq!(heading_content_start("###"), None);
        assert_eq!(heading_content_start("#Hanji"), None);
        assert_eq!(heading_content_start("####### Hanji"), None);
        assert_eq!(heading_content_start("    # Code"), None);
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
    fn finds_blockquote_content_start() {
        assert_eq!(blockquote_content_start("> "), Some(2));
        assert_eq!(blockquote_content_start("> Quote"), Some(2));
        assert_eq!(blockquote_content_start("   > Indented"), Some(5));
        assert_eq!(blockquote_content_start(">Quote"), None);
        assert_eq!(blockquote_content_start("    > Code"), None);
    }

    #[test]
    fn classifies_list_items() {
        assert_eq!(
            classify_line("- Item"),
            MarkdownLine::ListItem {
                marker: MarkdownListMarker::Unordered { marker: '-' },
                task: None,
            }
        );
        assert_eq!(
            classify_line("* Item"),
            MarkdownLine::ListItem {
                marker: MarkdownListMarker::Unordered { marker: '*' },
                task: None,
            }
        );
        assert_eq!(
            classify_line("+ Item"),
            MarkdownLine::ListItem {
                marker: MarkdownListMarker::Unordered { marker: '+' },
                task: None,
            }
        );
        assert_eq!(
            classify_line("  3. Item"),
            MarkdownLine::ListItem {
                marker: MarkdownListMarker::Ordered {
                    number: 3,
                    delimiter: OrderedListDelimiter::Dot,
                },
                task: None,
            }
        );
        assert_eq!(
            classify_line("2) Item"),
            MarkdownLine::ListItem {
                marker: MarkdownListMarker::Ordered {
                    number: 2,
                    delimiter: OrderedListDelimiter::Paren,
                },
                task: None,
            }
        );
    }

    #[test]
    fn finds_list_item_content_start() {
        assert_eq!(list_item_content_start("- Item"), Some(2));
        assert_eq!(list_item_content_start("-   Item"), Some(4));
        assert_eq!(list_item_content_start("  3. Item"), Some(5));
        assert_eq!(list_item_content_start("2) Item"), Some(3));
        assert_eq!(list_item_content_start("-Item"), None);
        assert_eq!(list_item_content_start("1.Item"), None);
        assert_eq!(list_item_content_start("    - Code"), None);
    }

    #[test]
    fn classifies_task_list_items() {
        assert_eq!(
            list_item("- [ ] Task"),
            Some(MarkdownListItem {
                marker: MarkdownListMarker::Unordered { marker: '-' },
                task: Some(MarkdownTaskState::Unchecked),
                content_start: 6,
            })
        );
        assert_eq!(
            list_item("- [x] Done"),
            Some(MarkdownListItem {
                marker: MarkdownListMarker::Unordered { marker: '-' },
                task: Some(MarkdownTaskState::Checked),
                content_start: 6,
            })
        );
        assert_eq!(
            list_item("1. [X] Done"),
            Some(MarkdownListItem {
                marker: MarkdownListMarker::Ordered {
                    number: 1,
                    delimiter: OrderedListDelimiter::Dot,
                },
                task: Some(MarkdownTaskState::Checked),
                content_start: 7,
            })
        );
        assert_eq!(classify_line("- [ ]"), MarkdownLine::Paragraph);
        assert_eq!(classify_line("1. [x]"), MarkdownLine::Paragraph);
        assert_eq!(list_item_content_start("- [ ]"), None);
        assert_eq!(list_item_content_start("- [ ]Task"), Some(2));
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
