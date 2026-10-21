//! The drawing surface handed to bands (`IFPDF` in the original).
//!
//! Coordinates are relative to the top-left corner of the band; the wrapper adds the band
//! origin and keeps track of the lowest point drawn, which is how the engine measures
//! auto-height bands.

use super::image_util::{ImgType, image_size, image_type};
use crate::document::Fpdf;
use crate::draw::ImageOpts;
use crate::ext::{HAlign, QrEcc, TextBoxOpts, VAlign};
use crate::fmt::round_to;
use crate::types::{Color, Orientation};

/// Drawing surface of a band.
pub struct ReportPdf<'a> {
    pdf: &'a mut Fpdf,
    offset_x: f64,
    offset_y: f64,
    pub(crate) modified: bool,
    pub(crate) highest_x: f64,
    pub(crate) highest_y: f64,
}

impl<'a> ReportPdf<'a> {
    pub(crate) fn new(pdf: &'a mut Fpdf, offset_xy: bool) -> Self {
        let (offset_x, offset_y) = if offset_xy {
            (pdf.x(), pdf.y())
        } else {
            (0.0, 0.0)
        };
        Self {
            pdf,
            offset_x,
            offset_y,
            modified: false,
            highest_x: 0.0,
            highest_y: 0.0,
        }
    }

    /// `true` once anything has been drawn.
    pub fn modified(&self) -> bool {
        self.modified
    }

    fn apply_highest_x(&mut self, x: f64) {
        if x > self.highest_x {
            self.highest_x = x;
            self.modified = true;
        }
    }

    fn apply_highest_y(&mut self, y: f64) {
        if y > self.highest_y {
            self.highest_y = y;
            self.modified = true;
        }
    }

    fn apply_highest_xy(&mut self, x: f64, y: f64) {
        self.apply_highest_x(x);
        self.apply_highest_y(y);
    }

    /// Translates band-relative coordinates to page coordinates and records them.
    fn apply_offset(&mut self, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = (self.offset_x + x, self.offset_y + y);
        self.apply_highest_x(x);
        self.apply_highest_y(y);
        (x, y)
    }

    // ---- state ------------------------------------------------------------------------

    /// Orientation of the page being drawn.
    pub fn orientation(&self) -> Orientation {
        self.pdf.orientation()
    }

    /// PDF page number being drawn.
    pub fn current_page(&self) -> usize {
        self.pdf.page_no()
    }

    /// Current font size in user units.
    pub fn current_font_size(&self) -> f64 {
        self.pdf.font_size()
    }

    /// Current font size in points.
    pub fn current_font_size_pt(&self) -> f64 {
        self.pdf.font_size_pt()
    }

    /// Current font family.
    pub fn current_font_family(&self) -> String {
        self.pdf.font_family().to_owned()
    }

    /// The text colour operator in use.
    pub fn text_color(&self) -> String {
        self.pdf.text_color_operator().to_owned()
    }

    /// Restores a text colour operator read with [`text_color`](ReportPdf::text_color).
    pub fn set_text_color_operator(&mut self, operator: &str) {
        self.pdf.set_text_color_operator(operator);
    }

    /// Selects a font by family, style and size (points; `0.0` keeps the current size).
    pub fn set_font(&mut self, family: &str, style: &str, size: f64) {
        self.pdf.set_font(family, style, size);
    }

    /// Changes size and style keeping the current family.
    pub fn set_font_size_style(&mut self, size: f64, style: &str) {
        let family = self.pdf.font_family().to_owned();
        self.pdf.set_font(&family, style, size);
    }

    /// Sets the stroking colour.
    pub fn set_draw_color(&mut self, color: impl Into<Color>) {
        self.pdf.set_draw_color(color);
    }

    /// Sets the fill colour.
    pub fn set_fill_color(&mut self, color: impl Into<Color>) {
        self.pdf.set_fill_color(color);
    }

    /// Sets the text colour.
    pub fn set_text_color(&mut self, color: impl Into<Color>) {
        self.pdf.set_text_color(color);
    }

    /// Enables or disables underline.
    pub fn set_underline(&mut self, underline: bool) {
        self.pdf.set_underline(underline);
    }

    /// Dash pattern (see [`Fpdf::set_dash`]).
    pub fn set_dash(&mut self, black: f64, white: f64) {
        self.pdf.set_dash(black, white);
    }

    /// Equal dash and gap (see [`Fpdf::set_dash_width`]).
    pub fn set_dash_width(&mut self, width: f64) {
        self.pdf.set_dash_width(width);
    }

    /// Line width in user units.
    pub fn set_line_width(&mut self, width: f64) {
        self.pdf.set_line_width(width);
    }

    // ---- measuring --------------------------------------------------------------------

    /// See [`Fpdf::word_wrap`].
    pub fn word_wrap(&self, text: &mut String, max_width: f64, indent: f64) -> usize {
        self.pdf.word_wrap(text, max_width, indent)
    }

    /// See [`Fpdf::get_num_lines`].
    pub fn get_num_lines(&self, text: &str, width: f64, indent: f64) -> usize {
        self.pdf.get_num_lines(text, width, indent)
    }

    /// See [`Fpdf::get_string_width`].
    pub fn get_string_width(&self, text: &str) -> f64 {
        self.pdf.get_string_width(text)
    }

    /// See [`Fpdf::get_string_height`].
    pub fn get_string_height(&self, text: &str, width: f64, line_spacing: f64, indent: f64) -> f64 {
        self.pdf
            .get_string_height(text, width, line_spacing, indent)
    }

    // ---- text -------------------------------------------------------------------------

    /// Prints a string with its baseline origin at `(x, y)`.
    pub fn text(&mut self, x: f64, y: f64, text: &str) {
        let (x, y) = self.apply_offset(x, y);
        let width = self.pdf.get_string_width(text);
        self.apply_highest_x(x + width);
        self.pdf.text(x, y, text);
    }

    /// An empty box with a border; returns the height used.
    pub fn text_box_frame(&mut self, x: f64, y: f64, w: f64, h: f64) -> f64 {
        let (x, y) = self.apply_offset(x, y);
        let used = self.pdf.text_box(x, y, w, h, "", &TextBoxOpts::default());
        self.apply_highest_xy(x + w, y + h.max(used));
        used
    }

    /// Text inside a box. `valign` is `'T'`, `'C'` or `'B'`; `halign` is `'L'`, `'C'` or
    /// `'R'`; a positive `border` draws the border; `force` selects the single-line mode that
    /// shrinks the text to fit (otherwise the text is word-wrapped). Returns the height used.
    #[allow(clippy::too_many_arguments)]
    pub fn text_box_force(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        text: &str,
        valign: char,
        halign: char,
        border: f64,
        force: bool,
    ) -> f64 {
        let opts = TextBoxOpts {
            valign: VAlign::from(valign),
            halign: HAlign::from(halign),
            border: border > 0.0,
            word_wrap: !force,
            scale: force,
            line_spacing: 0.0,
        };
        self.text_box_with(x, y, w, h, text, &opts)
    }

    /// [`text_box_force`](ReportPdf::text_box_force) with the default `force = true`.
    #[allow(clippy::too_many_arguments)]
    pub fn text_box(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        text: &str,
        valign: char,
        halign: char,
        border: f64,
    ) -> f64 {
        self.text_box_force(x, y, w, h, text, valign, halign, border, true)
    }

    /// Text inside a box with the explicit boolean options of the extension API
    /// (`border`, `word_wrap`, `scale`).
    #[allow(clippy::too_many_arguments)]
    pub fn text_box_flags(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        text: &str,
        valign: char,
        halign: char,
        border: bool,
        word_wrap: bool,
        scale: bool,
    ) -> f64 {
        let opts = TextBoxOpts {
            valign: VAlign::from(valign),
            halign: HAlign::from(halign),
            border,
            word_wrap,
            scale,
            line_spacing: 0.0,
        };
        self.text_box_with(x, y, w, h, text, &opts)
    }

    /// Text inside a box with explicit [`TextBoxOpts`].
    pub fn text_box_with(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        text: &str,
        opts: &TextBoxOpts,
    ) -> f64 {
        let (x, y) = self.apply_offset(x, y);
        let used = self.pdf.text_box(x, y, w, h, text, opts);
        self.apply_highest_xy(x + w, y + h.max(used));
        used
    }

    // ---- graphics ---------------------------------------------------------------------

    /// Rotates the following drawing around `(x, y)`; `rotate(0.0, x, y)` restores it.
    pub fn rotate(&mut self, angle: f64, x: f64, y: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.pdf.rotate(angle, Some(x), Some(y));
    }

    /// Draws a line.
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let (x1, y1) = self.apply_offset(x1, y1);
        let (x2, y2) = self.apply_offset(x2, y2);
        self.pdf.line(x1, y1, x2, y2);
    }

    /// Draws a rectangle (`style`: `""` outline, `"F"` fill, `"FD"` both).
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, style: &str) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + w, y + h);
        self.pdf.rect(x, y, w, h, style);
    }

    /// Draws a dashed line.
    pub fn dashed_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, dash_width: f64) {
        let (x1, y1) = self.apply_offset(x1, y1);
        let (x2, y2) = self.apply_offset(x2, y2);
        self.pdf.dashed_line(x1, y1, x2, y2, dash_width);
    }

    /// Draws a rectangle with a dashed outline.
    pub fn dashed_rect(&mut self, x: f64, y: f64, w: f64, h: f64, style: &str, dash_width: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + w, y + h);
        self.pdf.dashed_rect(x, y, w, h, style, dash_width);
    }

    /// Draws an image (PNG or JPEG bytes) in the box `(x, y, w, h)`.
    ///
    /// Unless `stretched`, the image keeps its proportions, fitting the box width; when it is
    /// wider than tall it is positioned vertically by `valign` (`'T'`, `'C'`, `'B'`), when taller
    /// than wide horizontally by `halign` (`'L'`, `'C'`, `'R'`).
    #[allow(clippy::too_many_arguments)]
    pub fn image(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        data: &[u8],
        valign: char,
        halign: char,
        stretched: bool,
    ) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + w, y + h);
        if data.is_empty() {
            return;
        }
        let Some((px_w, px_h)) = image_size(data) else {
            return;
        };
        let (mut x_img, mut y_img) = (x, y);
        let logo_w_mm = f64::from(px_w) / 72.0 * 25.4;
        let logo_h_mm = f64::from(px_h) / 72.0 * 25.4;
        let (img_w, img_h);
        if stretched {
            img_w = w;
            img_h = h;
        } else {
            let mut iw = w;
            let mut ih = round_to(logo_h_mm * (iw / logo_w_mm), 0);
            if ih > w {
                ih = w;
                iw = round_to(logo_w_mm * (ih / logo_h_mm), 0);
            }
            if iw > ih {
                match valign {
                    'C' => y_img += (h - ih) / 2.0,
                    'B' => y_img += h - ih,
                    _ => {}
                }
            } else if ih > iw {
                match halign {
                    'C' => x_img += (w - iw) / 2.0,
                    'R' => x_img += w - iw,
                    _ => {}
                }
            }
            img_w = iw;
            img_h = ih;
        }
        let ext = match image_type(data) {
            ImgType::Png => "PNG",
            ImgType::Jpg => "JPG",
            ImgType::Unknown => return,
        };
        let opts = ImageOpts {
            x: Some(x_img),
            y: Some(y_img),
            width: img_w,
            height: img_h,
            link: None,
        };
        self.pdf.image_bytes(data, ext, &opts);
    }

    // ---- barcodes ---------------------------------------------------------------------

    /// Code 128 barcode (`bar_width` is the total width).
    pub fn code128(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + bar_width, y + bar_height);
        self.pdf.code128(code, x, y, bar_height, bar_width);
    }

    /// Code 39 barcode.
    pub fn code39(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + bar_width, y + bar_height);
        self.pdf.code39(code, x, y, bar_height, bar_width);
    }

    /// Interleaved 2 of 5 barcode.
    pub fn code_i25(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + bar_width, y + bar_height);
        self.pdf.code_i25(code, x, y, bar_height, bar_width);
    }

    /// EAN-13 barcode.
    pub fn code_ean13(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + bar_width, y + bar_height);
        self.pdf.code_ean13(code, x, y, bar_height, bar_width);
    }

    /// EAN-8 barcode.
    pub fn code_ean8(&mut self, code: &str, x: f64, y: f64, bar_height: f64, bar_width: f64) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + bar_width, y + bar_height);
        self.pdf.code_ean8(code, x, y, bar_height, bar_width);
    }

    /// QR Code with modules `dot_size` wide; returns the side length drawn.
    pub fn qr_code(&mut self, x: f64, y: f64, data: &str, dot_size: f64, ecc: QrEcc) -> f64 {
        let (x, y) = self.apply_offset(x, y);
        let side = self.pdf.qr_code(x, y, data, dot_size, ecc);
        self.apply_highest_xy(x + side, y + side);
        side
    }

    /// QR Code fitted to a square of `size` units.
    pub fn qr_code_sized(&mut self, x: f64, y: f64, size: f64, data: &str, ecc: QrEcc) {
        let (x, y) = self.apply_offset(x, y);
        self.apply_highest_xy(x + size, y + size);
        self.pdf.qr_code_sized(x, y, size, data, ecc);
    }
}
