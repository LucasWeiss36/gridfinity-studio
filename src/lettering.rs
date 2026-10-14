// Small original 5x7 block alphabet. No external fonts or font runtime needed.
pub fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'Ä' => [10, 0, 14, 17, 31, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'Ö' => [10, 0, 14, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'Ü' => [10, 0, 17, 17, 17, 17, 14],
        'ß' | 'ẞ' => [14, 17, 17, 22, 17, 17, 22],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        _ => [0; 7],
    }
}
pub fn text(s: &str) -> String {
    let mut chars = s.chars().peekable();
    let mut result = String::with_capacity(s.len());
    while let Some(c) = chars.next() {
        // Accept both common Unicode spellings of German umlauts. This is
        // deliberately limited composition, not replacement of letter pairs.
        let uppercase = match c {
            'ä' => 'Ä',
            'ö' => 'Ö',
            'ü' => 'Ü',
            'ẞ' => 'ß',
            _ => c.to_ascii_uppercase(),
        };
        if matches!(uppercase, 'A' | 'O' | 'U') && chars.peek() == Some(&'\u{0308}') {
            chars.next();
            result.push(match uppercase {
                'A' => 'Ä',
                'O' => 'Ö',
                _ => 'Ü',
            });
        } else {
            result.push(uppercase);
        }
    }
    result
}

pub fn is_supported(s: &str) -> bool {
    text(s)
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || " -_ÄÖÜß".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_letters_remain_single_glyphs_in_both_unicode_spellings() {
        assert_eq!(text("äöüßẞ"), "ÄÖÜßß");
        assert_eq!(text("a\u{0308}o\u{0308}u\u{0308}ß"), "ÄÖÜß");
        assert_eq!(text("Maße für Öl"), "MAßE FÜR ÖL");
        assert_eq!(text("Michael Muenze AE OE UE"), "MICHAEL MUENZE AE OE UE");
        assert!(is_supported("ÄÖÜäöüßẞ a\u{0308} 123-_"));
        for unsupported in ["\u{0308}", "e\u{0308}", "é", "★", "ﬃ"] {
            assert!(!is_supported(unsupported), "{unsupported}");
        }
        assert_ne!(glyph('Ä'), glyph('A'));
        assert_ne!(glyph('Ö'), glyph('O'));
        assert_ne!(glyph('Ü'), glyph('U'));
        assert_ne!(glyph('ß'), glyph('B'));
        assert_ne!(glyph('ß'), glyph('S'));
    }
}
