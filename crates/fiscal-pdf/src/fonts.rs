//! The 14 PDF core fonts with their Adobe width metrics.
//!
//! The tables in [`fonts_data`](crate::fonts_data) come from the FPDF-Pascal sources, which
//! in turn take them from the Adobe font metrics files.

use crate::fonts_data::{
    CW_HELVETICA, CW_HELVETICA_BOLD, CW_HELVETICA_BOLD_OBLIQUE, CW_HELVETICA_OBLIQUE, CW_SYMBOL,
    CW_TIMES_BOLD, CW_TIMES_BOLD_ITALIC, CW_TIMES_ITALIC, CW_TIMES_ROMAN, CW_ZAPFDINGBATS,
    UV_CP1252, UV_SYMBOL, UV_ZAPFDINGBATS,
};

/// A core font definition.
#[derive(Debug, Clone)]
pub(crate) struct Font {
    /// `BaseFont` written to the PDF.
    pub name: String,
    /// Key used by [`Fpdf::set_font`](crate::Fpdf::set_font) to find the font.
    pub font_name: String,
    /// Glyph widths (1/1000 em) indexed by code page 1252 byte.
    pub cw: [u16; 256],
    /// `true` when the font uses the code page 1252 encoding (everything but the symbolic
    /// fonts).
    pub cp1252: bool,
    /// Underline position.
    pub up: f64,
    /// Underline thickness.
    pub ut: f64,
    /// First Unicode code point of each byte, `-1` when undefined.
    pub uv1: [i32; 256],
    /// Length of the run starting at `uv1`, `-1` for a single character.
    pub uv2: [i32; 256],
}

fn uv_table(sparse: &[(u8, i32, i32)]) -> ([i32; 256], [i32; 256]) {
    let mut uv1 = [-1; 256];
    let mut uv2 = [-1; 256];
    for &(i, a, b) in sparse {
        uv1[usize::from(i)] = a;
        uv2[usize::from(i)] = b;
    }
    (uv1, uv2)
}

fn font(name: &str, font_name: &str, cw: [u16; 256]) -> Font {
    let (uv1, uv2) = uv_table(UV_CP1252);
    Font {
        name: name.to_owned(),
        font_name: font_name.to_owned(),
        cw,
        cp1252: true,
        up: -100.0,
        ut: 50.0,
        uv1,
        uv2,
    }
}

fn symbolic(name: &str, cw: [u16; 256], uv: &[(u8, i32, i32)]) -> Font {
    let (uv1, uv2) = uv_table(uv);
    Font {
        name: name.to_owned(),
        font_name: name.to_owned(),
        cw,
        cp1252: false,
        up: -100.0,
        ut: 50.0,
        uv1,
        uv2,
    }
}

/// Builds the table of the 14 core fonts.
pub(crate) fn core_fonts() -> Vec<Font> {
    let courier = [600; 256];
    vec![
        font("Courier", "Courier", courier),
        font("Courier-Bold", "Courier-Bold", courier),
        font("Courier-Oblique", "Courier-Oblique", courier),
        font("Courier-BoldOblique", "Courier-BoldOblique", courier),
        font("Helvetica", "Helvetica", CW_HELVETICA),
        font("Helvetica-Bold", "Helvetica-Bold", CW_HELVETICA_BOLD),
        font(
            "Helvetica-Oblique",
            "Helvetica-Oblique",
            CW_HELVETICA_OBLIQUE,
        ),
        font(
            "Helvetica-BoldOblique",
            "Helvetica-BoldOblique",
            CW_HELVETICA_BOLD_OBLIQUE,
        ),
        font("Times-Roman", "Times", CW_TIMES_ROMAN),
        font("Times-Bold", "Times-Bold", CW_TIMES_BOLD),
        font("Times-Italic", "Times-Oblique", CW_TIMES_ITALIC),
        font(
            "Times-BoldItalic",
            "Times-BoldOblique",
            CW_TIMES_BOLD_ITALIC,
        ),
        symbolic("Symbol", CW_SYMBOL, UV_SYMBOL),
        symbolic("ZapfDingbats", CW_ZAPFDINGBATS, UV_ZAPFDINGBATS),
    ]
}

/// Builds the `ToUnicode` CMap stream of a font, or an empty string when it has none.
pub(crate) fn to_unicode_cmap(uv1: &[i32; 256], uv2: &[i32; 256]) -> String {
    let mut ranges = String::new();
    let mut chars = String::new();
    let (mut nbr, mut nbc) = (0, 0);
    for i in 0..256usize {
        if uv1[i] < 0 {
            continue;
        }
        if uv2[i] >= 0 {
            ranges.push_str(&format!(
                "<{:02X}> <{:02X}> <{:04X}>\n",
                i,
                i as i32 + uv2[i] - 1,
                uv1[i]
            ));
            nbr += 1;
        } else {
            chars.push_str(&format!("<{:02X}> <{:04X}>\n", i, uv1[i]));
            nbc += 1;
        }
    }
    if nbr == 0 && nbc == 0 {
        return String::new();
    }
    let mut s = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo\n\
         <</Registry (Adobe)\n/Ordering (UCS)\n/Supplement 0\n>> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<00> <FF>\n\
         endcodespacerange\n",
    );
    if nbr > 0 {
        s.push_str(&format!("{nbr} beginbfrange\n{ranges}endbfrange\n"));
    }
    if nbc > 0 {
        s.push_str(&format!("{nbc} beginbfchar\n{chars}endbfchar\n"));
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    s
}
