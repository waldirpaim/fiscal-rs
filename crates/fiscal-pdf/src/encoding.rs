//! Text encoding: the engine only draws with the PDF core fonts, which use `WinAnsiEncoding`
//! (code page 1252).

/// Maps a Unicode scalar to its code page 1252 byte (`?` when not representable).
pub(crate) fn char_to_cp1252(c: char) -> u8 {
    let code = u32::from(c);
    match code {
        0..=0x7F | 0xA0..=0xFF => code as u8,
        0x20AC => 0x80,
        0x201A => 0x82,
        0x0192 => 0x83,
        0x201E => 0x84,
        0x2026 => 0x85,
        0x2020 => 0x86,
        0x2021 => 0x87,
        0x02C6 => 0x88,
        0x2030 => 0x89,
        0x0160 => 0x8A,
        0x2039 => 0x8B,
        0x0152 => 0x8C,
        0x017D => 0x8E,
        0x2018 => 0x91,
        0x2019 => 0x92,
        0x201C => 0x93,
        0x201D => 0x94,
        0x2022 => 0x95,
        0x2013 => 0x96,
        0x2014 => 0x97,
        0x02DC => 0x98,
        0x2122 => 0x99,
        0x0161 => 0x9A,
        0x203A => 0x9B,
        0x0153 => 0x9C,
        0x017E => 0x9E,
        0x0178 => 0x9F,
        _ => b'?',
    }
}

/// Converts UTF-8 text to code page 1252 bytes.
pub(crate) fn to_cp1252(text: &str) -> Vec<u8> {
    text.chars().map(char_to_cp1252).collect()
}

/// Raw bytes of a PDF *text string* (document info entries, annotation URIs): plain ASCII
/// stays as is, anything else becomes UTF-16BE with a byte-order mark, as the PDF
/// specification requires.
pub(crate) fn text_string_raw(text: &str) -> Vec<u8> {
    if text.is_ascii() {
        text.as_bytes().to_vec()
    } else {
        let mut v = vec![0xFE, 0xFF];
        for unit in text.encode_utf16() {
            v.extend_from_slice(&unit.to_be_bytes());
        }
        v
    }
}

/// Wraps raw string bytes in parentheses, escaping them.
pub(crate) fn literal_string(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 2);
    out.push(b'(');
    out.extend(escape(raw));
    out.push(b')');
    out
}

/// Escapes `\`, `(`, `)` and carriage returns for a PDF literal string.
pub(crate) fn escape(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 4);
    for &b in bytes {
        match b {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b')' => out.extend_from_slice(b"\\)"),
            b'(' => out.extend_from_slice(b"\\("),
            b'\r' => out.extend_from_slice(b"\\r"),
            _ => out.push(b),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_portuguese_text() {
        assert_eq!(
            to_cp1252("Ação nº 1"),
            vec![b'A', 0xE7, 0xE3, b'o', b' ', b'n', 0xBA, b' ', b'1']
        );
    }

    #[test]
    fn maps_cp1252_specials_and_unknowns() {
        assert_eq!(to_cp1252("€–“”"), vec![0x80, 0x96, 0x93, 0x94]);
        assert_eq!(to_cp1252("日"), vec![b'?']);
    }

    #[test]
    fn escapes_literal_strings() {
        assert_eq!(escape(b"a(b)c\\d\re"), b"a\\(b\\)c\\\\d\\re".to_vec());
    }

    #[test]
    fn text_string_uses_utf16_for_non_ascii() {
        assert_eq!(literal_string(&text_string_raw("ab")), b"(ab)".to_vec());
        assert_eq!(
            literal_string(&text_string_raw("á")),
            vec![b'(', 0xFE, 0xFF, 0x00, 0xE1, b')']
        );
    }
}
