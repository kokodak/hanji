use std::ops::Range;

pub fn byte_offset_to_utf16(text: &str, offset: usize) -> usize {
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

pub fn utf16_offset_to_byte(text: &str, offset: usize) -> usize {
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

pub fn utf16_range_to_byte(text: &str, range: &Range<usize>) -> Range<usize> {
    utf16_offset_to_byte(text, range.start)..utf16_offset_to_byte(text, range.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_utf8_and_utf16_offsets_for_korean_and_emoji() {
        let text = "A한🇰🇷B";

        assert_eq!(byte_offset_to_utf16(text, 0), 0);
        assert_eq!(byte_offset_to_utf16(text, "A".len()), 1);
        assert_eq!(byte_offset_to_utf16(text, "A한".len()), 2);
        assert_eq!(byte_offset_to_utf16(text, "A한🇰🇷".len()), 6);

        assert_eq!(utf16_offset_to_byte(text, 0), 0);
        assert_eq!(utf16_offset_to_byte(text, 1), "A".len());
        assert_eq!(utf16_offset_to_byte(text, 2), "A한".len());
        assert_eq!(utf16_offset_to_byte(text, 6), "A한🇰🇷".len());
    }

    #[test]
    fn converts_utf16_ranges_to_byte_ranges() {
        let text = "A한🇰🇷B";

        assert_eq!(utf16_range_to_byte(text, &(1..6)), "A".len().."A한🇰🇷".len());
    }
}
