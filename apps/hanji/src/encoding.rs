use std::ops::Range;

pub(crate) fn byte_offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut byte_offset = 0;

    for character in text.chars() {
        if byte_offset >= offset {
            break;
        }

        byte_offset += character.len_utf8();
        utf16_offset += character.len_utf16();
    }

    utf16_offset
}

fn utf16_offset_to_byte(text: &str, offset: usize) -> usize {
    let mut byte_offset = 0;
    let mut utf16_offset = 0;

    for character in text.chars() {
        if utf16_offset >= offset {
            break;
        }

        utf16_offset += character.len_utf16();
        byte_offset += character.len_utf8();
    }

    byte_offset
}

pub(crate) fn utf16_range_to_byte(text: &str, range: &Range<usize>) -> Range<usize> {
    utf16_offset_to_byte(text, range.start)..utf16_offset_to_byte(text, range.end)
}
