//! One- and two-dimensional barcodes drawn with filled rectangles.

use qrcodegen::{QrCode, QrCodeEcc};

use super::barcode_tables::{CODE39_BARS, CODE128_TABLE, EAN_CODES, EAN_PARITIES};
use crate::document::Fpdf;

const DEFAULT_BAR_HEIGHT: f64 = 8.0;
const DEFAULT_BAR_WIDTH: f64 = 0.5;

/// Error-correction level of a QR Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QrEcc {
    /// About 7% of the codewords can be restored (the default of the original engine).
    #[default]
    Low,
    /// About 15%.
    Medium,
    /// About 25%.
    Quartile,
    /// About 30%.
    High,
}

impl From<QrEcc> for QrCodeEcc {
    fn from(value: QrEcc) -> Self {
        match value {
            QrEcc::Low => Self::Low,
            QrEcc::Medium => Self::Medium,
            QrEcc::Quartile => Self::Quartile,
            QrEcc::High => Self::High,
        }
    }
}

/// Check digit of an EAN code body: the rightmost digit weighs 3, then 1, 3, 1...
fn ean_check_digit(body: &str, name: &str) -> Result<u32, String> {
    let mut total = 0;
    for (j, c) in body.chars().rev().enumerate() {
        let Some(v) = c.to_digit(10) else {
            return Err(format!("Invalid Digits in {name}: {body}"));
        };
        total += v * if j % 2 == 0 { 3 } else { 1 };
    }
    Ok((10 - total % 10) % 10)
}

/// Pads, completes and validates an EAN code of `len` digits.
fn ean_normalise(code: &str, len: usize, name: &str) -> Result<String, String> {
    let mut code = code.trim().to_owned();
    if code.len() < len - 1 {
        code = format!("{}{code}", "0".repeat(len - 1 - code.len()));
    }
    if code.len() == len - 1 {
        let digit = ean_check_digit(&code, name)?;
        code.push(char::from_digit(digit, 10).unwrap_or('0'));
        Ok(code)
    } else if code.len() == len {
        let (body, last) = code.split_at(len - 1);
        if last.chars().next().and_then(|c| c.to_digit(10)) == Some(ean_check_digit(body, name)?) {
            Ok(code)
        } else {
            Err(format!("Invalid {name} Check Digit: {code}"))
        }
    } else {
        Err(format!("Invalid {name} Code Len: {code}"))
    }
}

/// Bar/space pattern (`1` = bar module) of an EAN-13 or EAN-8 code.
fn ean_bin_code(code: &str) -> String {
    let digits: Vec<usize> = code
        .chars()
        .filter_map(|c| c.to_digit(10))
        .map(|d| d as usize)
        .collect();
    let l = digits.len();
    let (offset, parity) = if l % 2 == 1 {
        (1, EAN_PARITIES[digits[0]])
    } else {
        (0, EAN_PARITIES[0])
    };
    let half = (l - offset) / 2;
    let mut out = String::from("101");
    for i in 0..half {
        out.push_str(EAN_CODES[usize::from(parity[i])][digits[offset + i]]);
    }
    out.push_str("01010");
    for &d in &digits[offset + half..] {
        out.push_str(EAN_CODES[2][d]);
    }
    out.push_str("101");
    out
}

impl Fpdf {
    fn draw_bin_code(&mut self, bin: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let bar_height = if bar_height == 0.0 {
            DEFAULT_BAR_HEIGHT
        } else {
            bar_height
        };
        let bar_width = if bar_width == 0.0 {
            DEFAULT_BAR_WIDTH
        } else {
            bar_width
        };
        for (i, c) in bin.chars().enumerate() {
            if c == '1' {
                self.rect(x + i as f64 * bar_width, y, bar_width, bar_height, "F");
            }
        }
    }

    /// Draws an EAN-13 barcode. A 12-digit code gets its check digit appended; shorter codes
    /// are left-padded with zeros. `bar_height`/`bar_width` of `0.0` use the defaults
    /// (8 and 0.5 user units).
    pub fn code_ean13(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        match ean_normalise(code, 13, "EAN13") {
            Ok(code) => self.draw_bin_code(&ean_bin_code(&code), x, y, bar_height, bar_width),
            Err(e) => self.fail(e),
        }
    }

    /// Draws an EAN-8 barcode (see [`code_ean13`](Fpdf::code_ean13)).
    pub fn code_ean8(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        match ean_normalise(code, 8, "EAN8") {
            Ok(code) => self.draw_bin_code(&ean_bin_code(&code), x, y, bar_height, bar_width),
            Err(e) => self.fail(e),
        }
    }

    /// Draws a Code 39 barcode (start/stop `*` are added when missing).
    pub fn code39(&mut self, code: &str, mut x: f64, y: f64, bar_height: f64, bar_width: f64) {
        const CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. *$/+%";
        let bar_height = if bar_height == 0.0 {
            DEFAULT_BAR_HEIGHT
        } else {
            bar_height
        };
        let bar_width = if bar_width == 0.0 {
            DEFAULT_BAR_WIDTH
        } else {
            bar_width
        };
        let mut s = code.to_uppercase();
        if s.is_empty() {
            return;
        }
        let wide = bar_width;
        let narrow = wide / 3.0;
        let gap = narrow;
        if !s.ends_with('*') {
            s.push('*');
        }
        if !s.starts_with('*') {
            s.insert(0, '*');
        }
        for c in s.chars() {
            let Some(p) = CHARS.find(c) else {
                self.fail(format!("Code39, invalid Char: {c}"));
                return;
            };
            for (j, bar) in CODE39_BARS[p].iter().enumerate() {
                let line_width = if *bar == 0 { narrow } else { wide };
                if j % 2 == 0 {
                    self.rect(x, y, line_width, bar_height, "F");
                }
                x += line_width;
            }
            x += gap;
        }
    }

    /// Draws an Interleaved 2 of 5 barcode (a leading zero is added to odd-length codes).
    pub fn code_i25(&mut self, code: &str, mut x: f64, y: f64, bar_height: f64, bar_width: f64) {
        const CHARS: &str = "0123456789AZ";
        const BARS: [&str; 12] = [
            "nnwwn", "wnnnw", "nwnnw", "wwnnn", "nnwnw", "wnwnn", "nwwnn", "nnnww", "wnnwn",
            "nwnwn", "nn", "wn",
        ];
        let s = code.trim().to_lowercase();
        if s.is_empty() {
            return;
        }
        let bar_height = if bar_height == 0.0 {
            DEFAULT_BAR_HEIGHT
        } else {
            bar_height
        };
        let bar_width = if bar_width == 0.0 {
            DEFAULT_BAR_WIDTH
        } else {
            bar_width
        };
        let wide = bar_width;
        let narrow = wide / 3.0;
        // Leading zero for odd lengths, then the start (AA) and stop (ZA) codes.
        let mut body = if s.chars().count() % 2 == 1 {
            format!("0{s}")
        } else {
            s
        };
        body = format!("AA{body}ZA");
        // The charset lookups are uppercase for the A/Z control symbols.
        let symbols: Vec<char> = body.chars().map(|c| c.to_ascii_uppercase()).collect();
        for pair in symbols.chunks(2) {
            let (Some(bar_idx), Some(space_idx)) = (
                CHARS.find(pair[0]),
                pair.get(1).and_then(|c| CHARS.find(*c)),
            ) else {
                self.fail(format!("CodeI25, Invalid character: {}", pair[0]));
                return;
            };
            let (bars, spaces) = (BARS[bar_idx].as_bytes(), BARS[space_idx].as_bytes());
            let mut seq = Vec::new();
            for j in 0..bars.len() {
                seq.push(bars[j]);
                seq.push(spaces[j]);
            }
            for (j, w) in seq.iter().enumerate() {
                let line_width = if *w == b'n' { narrow } else { wide };
                // Only every second element is a bar; the others are the interleaved spaces.
                if j % 2 == 0 {
                    self.rect(x, y, line_width, bar_height, "F");
                }
                x += line_width;
            }
        }
    }

    /// Draws a Code 128 barcode with automatic selection of the character sets (digit runs
    /// use set C, which is how NF-e access keys are encoded). `bar_width` is the **total**
    /// width of the barcode (`0.0` = 50 units, a value below `1.0` is multiplied by 100).
    pub fn code128(&mut self, code: &str, mut x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let bar_height = if bar_height == 0.0 {
            DEFAULT_BAR_HEIGHT
        } else {
            bar_height
        };
        let mut bar_width = if bar_width == 0.0 {
            DEFAULT_BAR_WIDTH
        } else {
            bar_width
        };
        if bar_width < 1.0 {
            bar_width *= 100.0;
        }
        let symbols = match code128_symbols(code.as_bytes()) {
            Ok(s) => s,
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        if symbols.is_empty() {
            return;
        }
        let modules = (symbols.len() * 11) as f64 - 8.0;
        let module = bar_width / modules;
        for sym in symbols {
            let c = CODE128_TABLE[usize::from(sym)];
            let mut j = 0;
            while j <= 5 {
                if c[j] > 0 {
                    self.rect(x, y, f64::from(c[j]) * module, bar_height, "F");
                    j += 1;
                    x += f64::from(c[j - 1] + c[j]) * module;
                }
                j += 1;
            }
        }
    }

    /// Draws a matrix of dots (`true` = filled) with its top-left corner at `(x, y)`.
    /// `dot_size == 0.0` uses 0.5 units.
    pub fn draw_2d_matrix(&mut self, matrix: &[Vec<bool>], x: f64, mut y: f64, dot_size: f64) {
        if matrix.is_empty() || matrix[0].is_empty() {
            return;
        }
        let dot = if dot_size == 0.0 {
            DEFAULT_BAR_WIDTH
        } else {
            dot_size
        };
        for row in matrix {
            let mut cx = x;
            for filled in row {
                if *filled {
                    self.rect(cx, y, dot, dot, "F");
                }
                cx += dot;
            }
            y += dot;
        }
    }

    /// Draws a QR Code whose modules are `dot_size` units wide (a one-module quiet zone is
    /// included) and returns the side length of the drawing.
    pub fn qr_code(&mut self, x: f64, y: f64, data: &str, dot_size: f64, ecc: QrEcc) -> f64 {
        let Some(matrix) = self.qr_matrix(data, ecc) else {
            return 0.0;
        };
        let side = matrix.len() as f64 * dot_size;
        self.draw_2d_matrix(&matrix, x, y, dot_size);
        side
    }

    /// Draws a QR Code that fits a square of `size` units (quiet zone included).
    pub fn qr_code_sized(&mut self, x: f64, y: f64, size: f64, data: &str, ecc: QrEcc) {
        let Some(matrix) = self.qr_matrix(data, ecc) else {
            return;
        };
        let dot = size / matrix.len() as f64;
        self.draw_2d_matrix(&matrix, x, y, dot);
    }

    /// Encodes `data` and returns the module matrix with a one-module quiet zone.
    fn qr_matrix(&mut self, data: &str, ecc: QrEcc) -> Option<Vec<Vec<bool>>> {
        let qr = match QrCode::encode_text(data, ecc.into()) {
            Ok(qr) => qr,
            Err(_) => {
                self.fail("QRCode: data too long");
                return None;
            }
        };
        let n = qr.size();
        let rows = (-1..=n)
            .map(|r| {
                (-1..=n)
                    .map(|c| (0..n).contains(&r) && (0..n).contains(&c) && qr.get_module(c, r))
                    .collect()
            })
            .collect();
        Some(rows)
    }
}

/// Code 128 set A/B/C selection and encoding (Roland Gautier's algorithm); returns the symbol
/// values including check digit, stop and the terminating bar.
fn code128_symbols(code: &[u8]) -> Result<Vec<u8>, String> {
    const START: [u8; 3] = [103, 104, 105]; // A, B, C
    const SWAP: [u8; 3] = [101, 100, 99];
    // Eligible characters of each set (index 0 = A, 1 = B, 2 = C).
    let in_a = |c: u8| (32..=95).contains(&c) || c <= 31 || (200..=210).contains(&c);
    let in_b = |c: u8| (32..=127).contains(&c) || (200..=210).contains(&c);
    let in_c = |c: u8| c.is_ascii_digit() || c == 206;
    let mut s: Vec<u8> = code.to_vec();
    let mut a_guid: Vec<u8> = s
        .iter()
        .map(|c| if in_a(*c) { b'O' } else { b'N' })
        .collect();
    let mut b_guid: Vec<u8> = s
        .iter()
        .map(|c| if in_b(*c) { b'O' } else { b'N' })
        .collect();
    let mut c_guid: Vec<u8> = s
        .iter()
        .map(|c| if in_c(*c) { b'O' } else { b'N' })
        .collect();
    let mini_c = b"OOOO";
    let mut crypt: Vec<u8> = Vec::new();

    while !s.is_empty() {
        // Force set C when four digits in a row are available.
        if let Some(p) = c_guid.windows(mini_c.len()).position(|w| w == mini_c) {
            a_guid[p] = b'N';
            b_guid[p] = b'N';
        }
        let made = if c_guid.len() >= mini_c.len() && &c_guid[..mini_c.len()] == mini_c {
            crypt.push(if crypt.is_empty() { START[2] } else { SWAP[2] });
            let mut run = c_guid
                .iter()
                .position(|g| *g == b'N')
                .unwrap_or(c_guid.len());
            if run % 2 == 1 {
                run -= 1;
            }
            for pair in s[..run].chunks(2) {
                let (a, b) = (pair[0], pair[1]);
                if !(a.is_ascii_digit() && b.is_ascii_digit()) {
                    return Err(format!(
                        "Code128, invalid digit pair: {}{}",
                        char::from(a),
                        char::from(b)
                    ));
                }
                crypt.push((a - b'0') * 10 + (b - b'0'));
            }
            run
        } else {
            let made_a = a_guid
                .iter()
                .position(|g| *g == b'N')
                .unwrap_or(a_guid.len());
            let made_b = b_guid
                .iter()
                .position(|g| *g == b'N')
                .unwrap_or(b_guid.len());
            let (run, set) = if made_a < made_b {
                (made_b, 1)
            } else {
                (made_a, 0)
            };
            if run == 0 {
                return Err(format!("Code128, invalid character: {}", char::from(s[0])));
            }
            crypt.push(if crypt.is_empty() {
                START[set]
            } else {
                SWAP[set]
            });
            for &c in &s[..run] {
                crypt.push(code128_value(c, set));
            }
            run
        };
        s.drain(..made);
        a_guid.drain(..made);
        b_guid.drain(..made);
        c_guid.drain(..made);
    }

    // Check digit: start value plus every symbol weighted by its position.
    let mut check = usize::from(crypt[0]);
    for (i, v) in crypt.iter().enumerate() {
        check += usize::from(*v) * i;
    }
    crypt.push((check % 103) as u8);
    crypt.push(106); // stop
    crypt.push(107); // terminating bar
    Ok(crypt)
}

/// Symbol value of a character in set A (`set == 0`) or B (`set == 1`).
fn code128_value(c: u8, set: usize) -> u8 {
    match (set, c) {
        // Control codes 200..=210 map to the special symbols 96..=106.
        (_, 200..=210) => c - 104,
        (0, 0..=31) => c + 64,
        (0, 32..=95) => c - 32,
        (_, 32..=127) => c - 32,
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ean_check_digits() {
        // 4006381333931 is a valid EAN-13; 96385074 a valid EAN-8.
        assert_eq!(
            ean_normalise("400638133393", 13, "EAN13").as_deref(),
            Ok("4006381333931")
        );
        assert_eq!(
            ean_normalise("4006381333931", 13, "EAN13").as_deref(),
            Ok("4006381333931")
        );
        assert!(ean_normalise("4006381333930", 13, "EAN13").is_err());
        assert_eq!(
            ean_normalise("9638507", 8, "EAN8").as_deref(),
            Ok("96385074")
        );
        assert_eq!(
            ean_normalise("123", 13, "EAN13").as_deref().map(str::len),
            Ok(13)
        );
        assert!(ean_normalise("12345678901234", 13, "EAN13").is_err());
        assert!(ean_normalise("40063813339x", 13, "EAN13").is_err());
    }

    #[test]
    fn ean13_pattern_has_95_modules() {
        let bin = ean_bin_code("4006381333931");
        assert_eq!(bin.len(), 95);
        assert!(bin.starts_with("101") && bin.ends_with("101"));
        assert_eq!(&bin[45..50], "01010");
    }

    #[test]
    fn code128_set_c_for_digit_runs() {
        // "1234" -> Start C, 12, 34, check, stop, bar.
        let s = code128_symbols(b"1234").expect("symbols");
        assert_eq!(s[..3], [105, 12, 34]);
        let check = (105 + 12 + 2 * 34) % 103;
        assert_eq!(s[3], check as u8);
        assert_eq!(s[4..], [106, 107]);
    }

    #[test]
    fn code128_access_key_has_22_pairs() {
        let key = "35260512345678000195550010000012341000012343";
        let s = code128_symbols(key.as_bytes()).expect("symbols");
        // Start C + 22 pairs + check + stop + bar.
        assert_eq!(s.len(), 1 + 22 + 3);
        assert_eq!(s[0], 105);
        assert_eq!(s[1], 35);
        assert_eq!(s[22], 43);
    }

    #[test]
    fn code128_switches_sets() {
        // Lowercase letters only exist in set B; the digit run switches to set C.
        let s = code128_symbols(b"ab123456").expect("symbols");
        assert_eq!(s[0], 104);
        assert_eq!(s[1], b'a' - 32);
        assert_eq!(s[2], b'b' - 32);
        assert!(s.contains(&99), "swaps to set C for the digit run");
        assert!(code128_symbols(&[0xFF]).is_err());
    }

    #[test]
    fn code128_known_vector() {
        // The example of the Code 128 specification. Sets A and B share these characters and
        // ties go to set A (Start A = 103), unlike the specification's walkthrough (set B).
        let s = code128_symbols(b"PJJ123C").expect("symbols");
        assert_eq!(s[..8], [103, 48, 42, 42, 17, 18, 19, 35]);
        let weighted: usize = s[..8]
            .iter()
            .enumerate()
            .map(|(i, v)| usize::from(*v) * i.max(1))
            .sum();
        assert_eq!(usize::from(s[8]), weighted % 103);
        assert_eq!(s[8], 54);
    }
}
