//! Drawing primitives: lines, rectangles, text, cells and images.

use std::io::Write as _;

use crate::document::Fpdf;
use crate::encoding::{escape, to_cp1252};
use crate::fmt::fixed;
use crate::image::{ImageInfo, parse_image};
use crate::types::{Align, Border, CellOpts, Link, Ln, Orientation};

/// Style of [`Fpdf::rect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RectStyle {
    /// Outline only.
    #[default]
    Draw,
    /// Filled, no outline.
    Fill,
    /// Filled and outlined.
    FillDraw,
}

impl RectStyle {
    /// Parses the string form: `""`/`"D"` outline, `"F"` fill, `"FD"`/`"DF"` both.
    pub fn parse(spec: &str) -> Self {
        match spec.to_ascii_uppercase().as_str() {
            "F" => Self::Fill,
            "FD" | "DF" => Self::FillDraw,
            _ => Self::Draw,
        }
    }

    pub(crate) fn operator(self) -> &'static str {
        match self {
            Self::Draw => "S",
            Self::Fill => "f",
            Self::FillDraw => "B",
        }
    }
}

impl From<&str> for RectStyle {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

/// Placement of an image.
#[derive(Debug, Clone, Default)]
pub struct ImageOpts {
    /// Left edge; `None` uses the current x.
    pub x: Option<f64>,
    /// Top edge; `None` uses the current y and flows the cursor below the image (breaking
    /// the page if needed).
    pub y: Option<f64>,
    /// Width; `0.0` derives it from the height, a negative value is a resolution in dpi.
    pub width: f64,
    /// Height; `0.0` derives it from the width, a negative value is a resolution in dpi.
    pub height: f64,
    /// Optional link on the image.
    pub link: Option<Link>,
}

/// Finds a 1-based, Pascal-style `Copy(s, start, count)` slice (clamped, never panics).
fn pcopy(s: &[u8], start: usize, count: usize) -> &[u8] {
    let begin = start.saturating_sub(1).min(s.len());
    let end = begin.saturating_add(count).min(s.len());
    &s[begin..end]
}

/// Geometry shared by the lines of one `multi_cell` call.
struct LineCfg {
    height: f64,
    w_first: f64,
    width: f64,
    indent: f64,
    align: Align,
    fill: bool,
}

impl Fpdf {
    /// Draws one line of a multi-line cell; the first line may be indented and narrower.
    fn emit_line(&mut self, cfg: &LineCfg, text: &[u8], border: Border, first_line: &mut bool) {
        let (line_width, line_indent) = if *first_line {
            (cfg.w_first, cfg.indent)
        } else {
            (cfg.width, 0.0)
        };
        let save_x = self.x;
        if line_indent > 0.0 {
            self.set_x(self.x + line_indent);
        }
        let opts = CellOpts {
            border,
            ln: Ln::Below,
            align: cfg.align,
            fill: cfg.fill,
            link: None,
        };
        self.cell_bytes(line_width, cfg.height, text, &opts);
        self.set_x(save_x);
        *first_line = false;
    }

    // ---------------------------------------------------------------- output

    /// Appends one operator line to the current page content.
    pub(crate) fn out(&mut self, line: &str) {
        self.out_bytes(line.as_bytes());
    }

    pub(crate) fn out_bytes(&mut self, line: &[u8]) {
        match self.state {
            2 => {
                if let Some(content) = self.pages.get_mut(self.page - 1) {
                    content.extend_from_slice(line);
                    content.push(b'\n');
                }
            }
            0 => self.fail("No page has been added yet"),
            1 => self.fail("Invalid call"),
            _ => self.fail("The document is closed"),
        }
    }

    // ---------------------------------------------------------------- shapes

    /// Draws a line.
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let k = self.k;
        let s = format!(
            "{} {} m {} {} l S",
            fixed(x1 * k, 2),
            fixed((self.h - y1) * k, 2),
            fixed(x2 * k, 2),
            fixed((self.h - y2) * k, 2)
        );
        self.out(&s);
    }

    /// Draws a rectangle.
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64, style: impl Into<RectStyle>) {
        let k = self.k;
        let s = format!(
            "{} {} {} {} re {}",
            fixed(x * k, 2),
            fixed((self.h - y) * k, 2),
            fixed(width * k, 2),
            fixed(-height * k, 2),
            style.into().operator()
        );
        self.out(&s);
    }

    // ------------------------------------------------------------------ text

    /// Prints a string with its baseline origin at `(x, y)`.
    pub fn text(&mut self, x: f64, y: f64, text: &str) {
        if self.current_font.is_none() {
            self.fail("No font has been set");
            return;
        }
        let t = to_cp1252(text);
        let mut s = Vec::new();
        let _ = write!(
            s,
            "BT {} {} Td (",
            fixed(x * self.k, 2),
            fixed((self.h - y) * self.k, 2)
        );
        s.extend(escape(&t));
        s.extend_from_slice(b") Tj ET");
        if self.underline && !t.is_empty() {
            s.push(b' ');
            s.extend(self.underline_op(x, y, &t).into_bytes());
        }
        if self.color_flag {
            let mut wrapped = format!("q {} ", self.text_color).into_bytes();
            wrapped.extend(s);
            wrapped.extend_from_slice(b" Q");
            s = wrapped;
        }
        self.out_bytes(&s);
    }

    /// The rectangle that underlines `text` drawn at `(x, y)`.
    fn underline_op(&self, x: f64, y: f64, text: &[u8]) -> String {
        let (up, ut) = self
            .current_font
            .and_then(|i| self.fonts.get(i))
            .map_or((-100.0, 50.0), |f| (f.up, f.ut));
        let spaces = text.iter().filter(|b| **b == b' ').count() as f64;
        let w = self.string_width(text) + self.ws * spaces;
        format!(
            "{} {} {} {} re f",
            fixed(x * self.k, 2),
            fixed((self.h - (y - up / 1000.0 * self.font_size)) * self.k, 2),
            fixed(w * self.k, 2),
            fixed(-ut / 1000.0 * self.font_size_pt, 2)
        )
    }

    /// Prints a cell (rectangular area) with optional borders, background and text; the
    /// cursor moves to the right. `width == 0.0` extends to the right margin.
    pub fn cell(&mut self, width: f64, height: f64, text: &str) {
        self.cell_with(width, height, text, &CellOpts::default());
    }

    /// Prints a cell with explicit [`CellOpts`].
    pub fn cell_with(&mut self, width: f64, height: f64, text: &str, opts: &CellOpts) {
        let t = to_cp1252(text);
        self.cell_bytes(width, height, &t, opts);
    }

    /// Cell on already encoded (code page 1252) text.
    pub(crate) fn cell_bytes(&mut self, width: f64, height: f64, text: &[u8], opts: &CellOpts) {
        let k = self.k;
        if self.y + height > self.page_break_trigger
            && !self.in_header
            && !self.in_footer
            && self.accept_page_break()
        {
            // Automatic page break.
            let x = self.x;
            let ws = self.ws;
            if ws > 0.0 {
                self.ws = 0.0;
                self.out("0 Tw");
            }
            self.add_page();
            self.x = x;
            if ws > 0.0 {
                self.ws = ws;
                self.out(&format!("{} Tw", fixed(ws * k, 3)));
            }
        }
        let width = if width == 0.0 {
            self.w - self.r_margin - self.x
        } else {
            width
        };

        let mut s: Vec<u8> = Vec::new();
        let border = opts.border;
        if opts.fill || border.frame {
            let op = match (opts.fill, border.frame) {
                (true, true) => "B",
                (true, false) => "f",
                _ => "S",
            };
            let _ = write!(
                s,
                "{} {} {} {} re {op} ",
                fixed(self.x * k, 2),
                fixed((self.h - self.y) * k, 2),
                fixed(width * k, 2),
                fixed(-height * k, 2)
            );
        }
        let (x, y) = (self.x, self.y);
        let mut side = |x1: f64, y1: f64, x2: f64, y2: f64| {
            let _ = write!(
                s,
                "{} {} m {} {} l S ",
                fixed(x1 * k, 2),
                fixed(y1 * k, 2),
                fixed(x2 * k, 2),
                fixed(y2 * k, 2)
            );
        };
        if border.left {
            side(x, self.h - y, x, self.h - (y + height));
        }
        if border.top {
            side(x, self.h - y, x + width, self.h - y);
        }
        if border.right {
            side(x + width, self.h - y, x + width, self.h - (y + height));
        }
        if border.bottom {
            side(x, self.h - (y + height), x + width, self.h - (y + height));
        }

        if !text.is_empty() {
            if self.current_font.is_none() {
                self.fail("No font has been set");
                return;
            }
            let text_width = self.string_width(text);
            let dx = match opts.align {
                Align::Right => width - self.c_margin - text_width,
                Align::Center => (width - text_width) / 2.0,
                Align::Left | Align::Justify => self.c_margin,
            };
            if self.color_flag {
                let _ = write!(s, "q {} ", self.text_color);
            }
            let baseline = self.y + 0.5 * height + 0.3 * self.font_size;
            let _ = write!(
                s,
                "BT {} {} Td (",
                fixed((self.x + dx) * k, 2),
                fixed((self.h - baseline) * k, 2)
            );
            s.extend(escape(text));
            s.extend_from_slice(b") Tj ET");
            if self.underline {
                s.push(b' ');
                s.extend(self.underline_op(self.x + dx, baseline, text).into_bytes());
            }
            if self.color_flag {
                s.extend_from_slice(b" Q");
            }
            if let Some(link) = &opts.link {
                self.link(
                    self.x + dx,
                    self.y + 0.5 * height - 0.5 * self.font_size,
                    text_width,
                    self.font_size,
                    link,
                );
            }
        }
        if !s.is_empty() {
            self.out_bytes(&s);
        }
        self.lasth = height;
        match opts.ln {
            Ln::Right => self.x += width,
            Ln::NextLine => {
                self.y += height;
                self.x = self.l_margin;
            }
            Ln::Below => self.y += height,
        }
    }

    /// Prints text with automatic or explicit (`\n`) line breaks inside a column `width`
    /// wide; `width == 0.0` extends to the right margin. Lines are `height` tall.
    pub fn multi_cell(
        &mut self,
        width: f64,
        height: f64,
        text: &str,
        border: impl Into<Border>,
        align: impl Into<Align>,
        fill: bool,
    ) {
        self.multi_cell_indent(width, height, text, border, align, fill, 0.0);
    }

    /// Like [`multi_cell`](Fpdf::multi_cell) with the first line indented by `indent`.
    #[allow(clippy::too_many_arguments)]
    pub fn multi_cell_indent(
        &mut self,
        width: f64,
        height: f64,
        text: &str,
        border: impl Into<Border>,
        align: impl Into<Align>,
        fill: bool,
        indent: f64,
    ) {
        let s: Vec<u8> = to_cp1252(text)
            .into_iter()
            .filter(|b| *b != b'\r')
            .collect();
        self.multi_cell_bytes(width, height, &s, border.into(), align.into(), fill, indent);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn multi_cell_bytes(
        &mut self,
        width: f64,
        height: f64,
        s: &[u8],
        border: Border,
        align: Align,
        fill: bool,
        indent: f64,
    ) {
        let Some(font) = self.current_font.and_then(|i| self.fonts.get(i)) else {
            self.fail("No font has been set");
            return;
        };
        let cw = font.cw;
        let width = if width == 0.0 {
            self.w - self.r_margin - self.x
        } else {
            width
        };
        let w_first = width - indent;
        let w_max_first = (w_first - 2.0 * self.c_margin) * 1000.0 / self.font_size;
        let w_max_other = (width - 2.0 * self.c_margin) * 1000.0 / self.font_size;

        let mut nb = s.len();
        if nb > 0 && s[nb - 1] == b'\n' {
            nb -= 1;
        }
        // `b` holds the borders of the line about to be drawn, `b2` those of the middle lines.
        let mut b = Border::NONE;
        let mut b2 = Border::NONE;
        if border.any() {
            if border.frame {
                b = Border {
                    left: true,
                    top: true,
                    right: true,
                    ..Border::NONE
                };
                b2 = Border {
                    left: true,
                    right: true,
                    ..Border::NONE
                };
            } else {
                b2 = Border {
                    left: border.left,
                    right: border.right,
                    ..Border::NONE
                };
                b = Border {
                    top: border.top,
                    ..b2
                };
            }
        }
        let want_bottom = border.frame || border.bottom;

        let mut sep: Option<usize> = None;
        let (mut i, mut j) = (1usize, 1usize);
        let mut l: u32 = 0;
        let mut ls: u32 = 0;
        let mut ns: u32 = 0;
        let mut nl = 1;
        let mut first_line = true;

        let line = LineCfg {
            height,
            w_first,
            width,
            indent,
            align,
            fill,
        };

        while i <= nb {
            let c = s[i - 1];
            if c == b'\n' {
                // Explicit line break.
                if self.ws > 0.0 {
                    self.ws = 0.0;
                    self.out("0 Tw");
                }
                self.emit_line(&line, pcopy(s, j, i - j), b, &mut first_line);
                i += 1;
                sep = None;
                j = i;
                l = 0;
                ns = 0;
                nl += 1;
                if border.any() && nl == 2 {
                    b = b2;
                }
                continue;
            }
            if c == b' ' {
                sep = Some(i);
                ls = l;
                ns += 1;
            }
            l += u32::from(cw[usize::from(c)]);
            let w_max = if first_line { w_max_first } else { w_max_other };
            if f64::from(l) > w_max {
                // Automatic line break.
                match sep {
                    None => {
                        if i == j {
                            i += 1;
                        }
                        if self.ws > 0.0 {
                            self.ws = 0.0;
                            self.out("0 Tw");
                        }
                        self.emit_line(&line, pcopy(s, j, i - j), b, &mut first_line);
                    }
                    Some(sep_pos) => {
                        if align == Align::Justify {
                            self.ws = if ns > 1 {
                                (w_max - f64::from(ls)) / 1000.0 * self.font_size
                                    / f64::from(ns - 1)
                            } else {
                                0.0
                            };
                            let tw = format!("{} Tw", fixed(self.ws * self.k, 3));
                            self.out(&tw);
                        }
                        self.emit_line(&line, pcopy(s, j, sep_pos - j), b, &mut first_line);
                        i = sep_pos + 1;
                    }
                }
                sep = None;
                j = i;
                l = 0;
                ns = 0;
                nl += 1;
                if border.any() && nl == 2 {
                    b = b2;
                }
            } else {
                i += 1;
            }
        }
        // Last chunk.
        if self.ws > 0.0 {
            self.ws = 0.0;
            self.out("0 Tw");
        }
        if want_bottom {
            b.bottom = true;
        }
        self.emit_line(&line, pcopy(s, j, i.saturating_sub(j)), b, &mut first_line);
        self.x = self.l_margin;
    }

    /// Prints flowing text: the cursor ends right after the text so the next call continues
    /// on the same line. `height` is the line height.
    pub fn write(&mut self, height: f64, text: &str, link: Option<&Link>) {
        let Some(font) = self.current_font.and_then(|i| self.fonts.get(i)) else {
            self.fail("No font has been set");
            return;
        };
        let cw = font.cw;
        let s: Vec<u8> = to_cp1252(text)
            .into_iter()
            .filter(|b| *b != b'\r')
            .collect();
        let nb = s.len();
        let mut w = self.w - self.r_margin - self.x;
        let mut w_max = (w - 2.0 * self.c_margin) * 1000.0 / self.font_size;
        let opts = |link: Option<&Link>| CellOpts {
            ln: Ln::Below,
            link: link.cloned(),
            ..CellOpts::default()
        };

        let mut sep: Option<usize> = None;
        let (mut i, mut j) = (1usize, 1usize);
        let mut l: u32 = 0;
        let mut nl = 1;
        while i <= nb {
            let c = s[i - 1];
            if c == b'\n' {
                self.cell_bytes(w, height, pcopy(&s, j, i - j), &opts(link));
                i += 1;
                sep = None;
                j = i;
                l = 0;
                if nl == 1 {
                    self.x = self.l_margin;
                    w = self.w - self.r_margin - self.x;
                    w_max = (w - 2.0 * self.c_margin) * 1000.0 / self.font_size;
                }
                nl += 1;
                continue;
            }
            if c == b' ' {
                sep = Some(i);
            }
            l += u32::from(cw[usize::from(c)]);
            if f64::from(l) > w_max {
                // Automatic line break.
                match sep {
                    None => {
                        if self.x > self.l_margin {
                            // Move to the next line and retry.
                            self.x = self.l_margin;
                            self.y += height;
                            w = self.w - self.r_margin - self.x;
                            w_max = (w - 2.0 * self.c_margin) * 1000.0 / self.font_size;
                            i += 1;
                            nl += 1;
                            continue;
                        }
                        if i == j {
                            i += 1;
                        }
                        self.cell_bytes(w, height, pcopy(&s, j, i - j), &opts(link));
                    }
                    Some(sep_pos) => {
                        self.cell_bytes(w, height, pcopy(&s, j, sep_pos - j), &opts(link));
                        i = sep_pos + 1;
                    }
                }
                sep = None;
                j = i;
                l = 0;
                if nl == 1 {
                    self.x = self.l_margin;
                    w = self.w - self.r_margin - self.x;
                    w_max = (w - 2.0 * self.c_margin) * 1000.0 / self.font_size;
                }
                nl += 1;
            } else {
                i += 1;
            }
        }
        // Last chunk.
        if i != j {
            let cell_w = f64::from(l) / 1000.0 * self.font_size;
            let cell_opts = CellOpts {
                link: link.cloned(),
                ..CellOpts::default()
            };
            self.cell_bytes(cell_w, height, pcopy(&s, j, s.len()), &cell_opts);
        }
    }

    /// Line feed: x returns to the left margin and y advances by `height` (`<= 0.0` uses the
    /// height of the last printed cell).
    pub fn ln(&mut self, height: f64) {
        self.x = self.l_margin;
        self.y += if height <= 0.0 { self.lasth } else { height };
    }

    // ----------------------------------------------------------------- images

    /// Draws an image given its encoded bytes and extension (`"png"`, `"jpg"`, `"jpeg"`).
    ///
    /// The same bytes drawn again reuse the embedded image.
    pub fn image_bytes(&mut self, data: &[u8], ext: &str, opts: &ImageOpts) {
        let key = content_key(data);
        let index = match self.images.iter().position(|img| img.name == key) {
            Some(i) => i,
            None => match parse_image(data, ext) {
                Ok(mut info) => {
                    self.register_image(&mut info, key);
                    self.images.push(info);
                    self.images.len() - 1
                }
                Err(e) => {
                    self.fail_with(e);
                    return;
                }
            },
        };
        self.place_image(index, opts);
    }

    /// Draws an image read from a file; the extension of the file name selects the format.
    pub fn image_file(&mut self, path: impl AsRef<std::path::Path>, opts: &ImageOpts) {
        let path = path.as_ref();
        let key = path.to_string_lossy().into_owned();
        let index = match self.images.iter().position(|img| img.name == key) {
            Some(i) => i,
            None => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext.is_empty() {
                    self.fail("Image File without an extension!");
                    return;
                }
                let data = match std::fs::read(path) {
                    Ok(d) => d,
                    Err(e) => {
                        self.fail_with(crate::error::FpdfError::io(
                            format!("Unable to read image {key}"),
                            &e,
                        ));
                        return;
                    }
                };
                match parse_image(&data, ext) {
                    Ok(mut info) => {
                        self.register_image(&mut info, key);
                        self.images.push(info);
                        self.images.len() - 1
                    }
                    Err(e) => {
                        self.fail_with(e);
                        return;
                    }
                }
            }
        };
        self.place_image(index, opts);
    }

    fn register_image(&mut self, info: &mut ImageInfo, key: String) {
        info.i = self.images.len() + 1;
        info.name = key;
        if info.has_alpha() {
            self.with_alpha = true;
            if self.pdf_version < 1.4 {
                self.pdf_version = 1.4;
            }
        }
    }

    fn place_image(&mut self, index: usize, opts: &ImageOpts) {
        let (img_w, img_h, img_i) = {
            let img = &self.images[index];
            (f64::from(img.w), f64::from(img.h), img.i)
        };
        let k = self.k;
        let (mut w, mut h) = (opts.width, opts.height);
        // Automatic width and height: put the image at 96 dpi.
        if w == 0.0 && h == 0.0 {
            w = -96.0;
            h = -96.0;
        }
        if w < 0.0 {
            w = -img_w * 72.0 / w / k;
        }
        if h < 0.0 {
            h = -img_h * 72.0 / h / k;
        }
        if w == 0.0 {
            w = h * img_w / img_h;
        }
        if h == 0.0 {
            h = w * img_h / img_w;
        }
        // Flowing mode.
        let y = match opts.y {
            Some(y) => y,
            None => {
                if self.y + h > self.page_break_trigger
                    && !self.in_header
                    && !self.in_footer
                    && self.accept_page_break()
                {
                    let x = self.x;
                    self.add_page_with(self.cur_orientation, self.cur_page_size, self.cur_rotation);
                    self.x = x;
                }
                let y = self.y;
                self.y += h;
                y
            }
        };
        let x = opts.x.unwrap_or(self.x);
        let op = format!(
            "q {} 0 0 {} {} {} cm /I{} Do Q",
            fixed(w * k, 2),
            fixed(h * k, 2),
            fixed(x * k, 2),
            fixed((self.h - (y + h)) * k, 2),
            img_i
        );
        self.out(&op);
        if let Some(link) = &opts.link {
            self.link(x, y, w, h, link);
        }
    }

    /// Current orientation of the page being drawn.
    pub fn orientation(&self) -> Orientation {
        self.cur_orientation
    }
}

/// Cache key of an image given by content.
fn content_key(data: &[u8]) -> String {
    // FNV-1a over the bytes plus the length: cheap and collision-safe enough to avoid
    // embedding the same logo twice.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("#{hash:016x}:{}", data.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcopy_matches_pascal_copy() {
        let s = b"abcdef";
        assert_eq!(pcopy(s, 1, 3), b"abc");
        assert_eq!(pcopy(s, 4, 10), b"def");
        assert_eq!(pcopy(s, 7, 2), b"");
        assert_eq!(pcopy(s, 0, 2), b"ab");
    }
}
