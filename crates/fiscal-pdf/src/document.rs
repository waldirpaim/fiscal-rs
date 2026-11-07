//! The document object: construction, settings, pages, fonts, colours and links.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::encoding::to_cp1252;
use crate::error::FpdfError;
use crate::ext::Layer;
use crate::fmt::{fixed, float_to_str};
use crate::fonts::{Font, core_fonts};
use crate::image::ImageInfo;
use crate::types::{
    Color, DateTime, LayoutMode, Link, Orientation, PageFormat, PageSize, Rotation, Unit, ZoomMode,
};

/// Version of the FPDF library this engine is ported from.
pub const FPDF_VERSION: &str = "1.85";

/// Callback run when a page starts or ends (see [`Fpdf::set_header`] and [`Fpdf::set_footer`]).
pub type PageHook = Box<dyn FnMut(&mut Fpdf) + Send>;

/// Callback deciding whether an automatic page break is accepted
/// (see [`Fpdf::set_accept_page_break`]).
pub type PageBreakHook = Box<dyn FnMut(&mut Fpdf) -> bool + Send>;

#[derive(Debug, Clone, Default)]
pub(crate) struct PageInfo {
    /// Custom media box (points), when the page differs from the document default.
    pub size: Option<(f64, f64)>,
    /// Rotation in degrees, when not zero.
    pub rotation: Option<i32>,
    /// PDF object number of the page.
    pub n: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct PageLink {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub link: Link,
    pub n: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LinkDest {
    pub page: usize,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct UsedFont {
    /// Key used to look the font up (`family-StyleName`).
    pub font_name: String,
    /// PDF object number.
    pub n: usize,
}

/// An FPDF-style PDF document under construction.
///
/// The API mirrors FPDF: create the document, [`add_page`](Fpdf::add_page), select a font,
/// draw with [`cell`](Fpdf::cell), [`multi_cell`](Fpdf::multi_cell), [`line`](Fpdf::line),
/// [`rect`](Fpdf::rect), [`image_bytes`](Fpdf::image_bytes)..., and finally
/// [`output`](Fpdf::output) the bytes. Coordinates grow rightwards and downwards from the
/// top-left corner, in the unit chosen at creation.
///
/// Only the 14 PDF core fonts are available (`courier`, `helvetica`/`arial`, `times`,
/// `symbol`, `zapfdingbats`); text is encoded in code page 1252.
///
/// # Errors
///
/// Misuse (no font selected, invalid link, unsupported image...) is recorded as a sticky
/// error: later drawing calls do nothing and [`output`](Fpdf::output) returns the first error.
///
/// # Example
///
/// ```
/// use fiscal_pdf::{Fpdf, Orientation, PageFormat, Unit};
///
/// let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
/// pdf.add_page();
/// pdf.set_font("Arial", "B", 16.0);
/// pdf.cell(40.0, 10.0, "Hello World!");
/// let bytes = pdf.output().expect("valid document");
/// assert!(bytes.starts_with(b"%PDF-1.3"));
/// ```
pub struct Fpdf {
    pub(crate) error: Option<FpdfError>,
    pub(crate) page: usize,
    pub(crate) n: usize,
    pub(crate) offsets: Vec<usize>,
    pub(crate) buffer: Vec<u8>,
    pub(crate) pages: Vec<Vec<u8>>,
    /// 0 = no page yet, 1 = page closed, 2 = page open, 3 = document closed.
    pub(crate) state: u8,
    pub(crate) compress: bool,
    /// Points per user unit.
    pub(crate) k: f64,
    pub(crate) def_orientation: Orientation,
    pub(crate) cur_orientation: Orientation,
    pub(crate) def_page_size: PageSize,
    pub(crate) cur_page_size: PageSize,
    pub(crate) cur_rotation: Rotation,
    pub(crate) page_info: Vec<PageInfo>,
    pub(crate) w_pt: f64,
    pub(crate) h_pt: f64,
    /// Page width in user units.
    pub(crate) w: f64,
    /// Page height in user units.
    pub(crate) h: f64,
    pub(crate) l_margin: f64,
    pub(crate) t_margin: f64,
    pub(crate) r_margin: f64,
    pub(crate) b_margin: f64,
    pub(crate) c_margin: f64,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) lasth: f64,
    pub(crate) line_width: f64,
    pub(crate) fonts: Vec<Font>,
    pub(crate) used_fonts: Vec<UsedFont>,
    pub(crate) font_family: String,
    pub(crate) font_style: String,
    pub(crate) underline: bool,
    /// Index into `fonts` of the selected font.
    pub(crate) current_font: Option<usize>,
    /// `used_fonts` key of the selected font.
    pub(crate) current_font_key: String,
    pub(crate) font_size_pt: f64,
    pub(crate) font_size: f64,
    pub(crate) draw_color: String,
    pub(crate) fill_color: String,
    pub(crate) text_color: String,
    pub(crate) color_flag: bool,
    pub(crate) with_alpha: bool,
    /// Word spacing (user units) used to justify text.
    pub(crate) ws: f64,
    pub(crate) images: Vec<ImageInfo>,
    pub(crate) page_links: Vec<Vec<PageLink>>,
    pub(crate) links: Vec<LinkDest>,
    pub(crate) auto_page_break: bool,
    pub(crate) page_break_trigger: f64,
    pub(crate) in_header: bool,
    pub(crate) in_footer: bool,
    pub(crate) alias_nb_pages: Vec<u8>,
    pub(crate) zoom_mode: ZoomMode,
    pub(crate) zoom_factor: i32,
    pub(crate) layout_mode: LayoutMode,
    /// Document information entries in insertion order.
    pub(crate) metadata: Vec<(String, String)>,
    pub(crate) creation_date: Option<DateTime>,
    pub(crate) pdf_version: f64,
    pub(crate) time_zone: String,
    pub(crate) header: Option<PageHook>,
    pub(crate) footer: Option<PageHook>,
    pub(crate) accept_page_break_hook: Option<PageBreakHook>,
    /// Rotation angle (degrees) currently open, `0.0` when none.
    pub(crate) angle: f64,
    pub(crate) layers: Vec<Layer>,
    pub(crate) current_layer: Option<usize>,
    pub(crate) open_layer_pane: bool,
    pub(crate) html_href: String,
    pub(crate) html_style: String,
    #[cfg(feature = "protection")]
    pub(crate) encryption: Option<crate::ext::protection::Encryption>,
}

impl std::fmt::Debug for Fpdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fpdf")
            .field("page", &self.page)
            .field("state", &self.state)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl Fpdf {
    /// Creates a document with a standard page format.
    pub fn new(orientation: Orientation, unit: Unit, format: PageFormat) -> Self {
        let k = unit.scale();
        let pt = format.size_pt();
        Self::with_size(
            orientation,
            unit,
            PageSize {
                w: pt.w / k,
                h: pt.h / k,
            },
        )
    }

    /// Creates a document with a custom page size given in the chosen `unit`.
    ///
    /// The size is normalised so that `h >= w` (the orientation decides the final shape).
    pub fn with_size(orientation: Orientation, unit: Unit, size: PageSize) -> Self {
        let k = unit.scale();
        let size = if size.w > size.h {
            PageSize {
                w: size.h,
                h: size.w,
            }
        } else {
            size
        };
        let orientation = if orientation == Orientation::Default {
            Orientation::Portrait
        } else {
            orientation
        };
        let (w, h) = if orientation == Orientation::Landscape {
            (size.h, size.w)
        } else {
            (size.w, size.h)
        };
        // 1 cm page margins, 1 mm cell margin, 0.2 mm line width.
        let margin = 28.35 / k;
        let font_size_pt = 12.0;
        let mut pdf = Self {
            error: None,
            page: 0,
            n: 2,
            offsets: Vec::new(),
            buffer: Vec::new(),
            pages: Vec::new(),
            state: 0,
            compress: true,
            k,
            def_orientation: orientation,
            cur_orientation: orientation,
            def_page_size: size,
            cur_page_size: size,
            cur_rotation: Rotation::R0,
            page_info: Vec::new(),
            w_pt: w * k,
            h_pt: h * k,
            w,
            h,
            l_margin: margin,
            t_margin: margin,
            r_margin: margin,
            b_margin: 2.0 * margin,
            c_margin: margin / 10.0,
            x: 0.0,
            y: 0.0,
            lasth: 0.0,
            line_width: 0.567 / k,
            fonts: core_fonts(),
            used_fonts: Vec::new(),
            font_family: String::new(),
            font_style: String::new(),
            underline: false,
            current_font: None,
            current_font_key: String::new(),
            font_size_pt,
            font_size: font_size_pt / k,
            draw_color: "0 G".to_owned(),
            fill_color: "0 g".to_owned(),
            text_color: "0 g".to_owned(),
            color_flag: false,
            with_alpha: false,
            ws: 0.0,
            images: Vec::new(),
            page_links: Vec::new(),
            links: Vec::new(),
            auto_page_break: true,
            page_break_trigger: h - 2.0 * margin,
            in_header: false,
            in_footer: false,
            alias_nb_pages: Vec::new(),
            zoom_mode: ZoomMode::Default,
            zoom_factor: 0,
            layout_mode: LayoutMode::Default,
            metadata: Vec::new(),
            creation_date: None,
            pdf_version: 1.3,
            time_zone: "Z".to_owned(),
            header: None,
            footer: None,
            accept_page_break_hook: None,
            angle: 0.0,
            layers: Vec::new(),
            current_layer: None,
            open_layer_pane: false,
            html_href: String::new(),
            html_style: String::new(),
            #[cfg(feature = "protection")]
            encryption: None,
        };
        pdf.set_metadata(
            "Producer",
            &format!(
                "fiscal-pdf {} (FPDF {FPDF_VERSION})",
                env!("CARGO_PKG_VERSION")
            ),
        );
        pdf
    }

    // ---------------------------------------------------------------- errors

    /// Records the first error; later calls become no-ops.
    pub(crate) fn fail(&mut self, message: impl Into<String>) {
        if self.error.is_none() {
            self.error = Some(FpdfError::msg(message));
        }
    }

    pub(crate) fn fail_with(&mut self, error: FpdfError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    /// The first error recorded so far, if any.
    pub fn error(&self) -> Option<&FpdfError> {
        self.error.as_ref()
    }

    // -------------------------------------------------------------- settings

    /// Sets the left, top and right margins. `right = None` copies the left margin.
    pub fn set_margins(&mut self, left: f64, top: f64, right: Option<f64>) {
        self.l_margin = left;
        self.t_margin = top;
        self.r_margin = right.unwrap_or(left);
    }

    /// Sets the left margin (moving `x` if it is now inside the margin).
    pub fn set_left_margin(&mut self, margin: f64) {
        self.l_margin = margin;
        if self.page > 0 && self.x < margin {
            self.x = margin;
        }
    }

    /// Sets the top margin.
    pub fn set_top_margin(&mut self, margin: f64) {
        self.t_margin = margin;
    }

    /// Sets the right margin.
    pub fn set_right_margin(&mut self, margin: f64) {
        self.r_margin = margin;
    }

    /// Enables or disables automatic page breaks; `margin` is the distance from the page
    /// bottom that triggers the break.
    pub fn set_auto_page_break(&mut self, auto: bool, margin: f64) {
        self.auto_page_break = auto;
        self.b_margin = margin;
        self.page_break_trigger = self.h - margin;
    }

    /// Sets the initial zoom and page layout; `zoom_factor` (percent) applies to
    /// [`ZoomMode::Custom`].
    pub fn set_display_mode(&mut self, zoom: ZoomMode, layout: LayoutMode, zoom_factor: i32) {
        self.zoom_mode = zoom;
        self.layout_mode = layout;
        self.zoom_factor = zoom_factor;
    }

    /// Enables or disables page content compression (enabled by default).
    pub fn set_compression(&mut self, compress: bool) {
        self.compress = compress;
    }

    pub(crate) fn set_metadata(&mut self, key: &str, value: &str) {
        match self.metadata.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value.to_owned(),
            None => self.metadata.push((key.to_owned(), value.to_owned())),
        }
    }

    /// Sets the document title.
    pub fn set_title(&mut self, title: &str) {
        self.set_metadata("Title", title);
    }

    /// Sets the document author.
    pub fn set_author(&mut self, author: &str) {
        self.set_metadata("Author", author);
    }

    /// Sets the document subject.
    pub fn set_subject(&mut self, subject: &str) {
        self.set_metadata("Subject", subject);
    }

    /// Sets the document keywords.
    pub fn set_keywords(&mut self, keywords: &str) {
        self.set_metadata("Keywords", keywords);
    }

    /// Sets the creating application.
    pub fn set_creator(&mut self, creator: &str) {
        self.set_metadata("Creator", creator);
    }

    /// Sets the producer (defaults to `fiscal-pdf`).
    pub fn set_producer(&mut self, producer: &str) {
        self.set_metadata("Producer", producer);
    }

    /// Defines an alias (usually `{nb}`) replaced by the total page count in every page.
    pub fn set_alias_nb_pages(&mut self, alias: &str) {
        self.alias_nb_pages = to_cp1252(alias);
    }

    /// Sets the creation date written in the information dictionary. Without it the current
    /// UTC time is used when the document is closed.
    pub fn set_creation_date(&mut self, date: DateTime) {
        self.creation_date = Some(date);
    }

    /// Sets the time zone of the creation date: `"Z"` (default) or an offset like `"-03:00"`.
    pub fn set_time_zone(&mut self, time_zone: &str) {
        let tz = time_zone.to_ascii_uppercase();
        if tz != "Z" {
            let b = tz.as_bytes();
            let mut valid = b.len() == 6 && (b[0] == b'-' || b[0] == b'+') && b[3] == b':';
            if valid {
                let hours: i32 = tz[..3].parse().unwrap_or(-99);
                let minutes: i32 = tz[4..].parse().unwrap_or(-99);
                valid = (-11..=14).contains(&hours) && (0..=60).contains(&minutes);
            }
            if !valid {
                self.fail(format!("Invalid TimeZone: {time_zone}"));
                return;
            }
        }
        self.time_zone = tz;
    }

    /// Installs a callback run at the start of every page (after the page state is set up).
    pub fn set_header(&mut self, hook: impl FnMut(&mut Fpdf) + Send + 'static) {
        self.header = Some(Box::new(hook));
    }

    /// Installs a callback run at the end of every page.
    pub fn set_footer(&mut self, hook: impl FnMut(&mut Fpdf) + Send + 'static) {
        self.footer = Some(Box::new(hook));
    }

    /// Installs a callback that decides whether an automatic page break is accepted
    /// (by default it follows [`set_auto_page_break`](Fpdf::set_auto_page_break)).
    pub fn set_accept_page_break(&mut self, hook: impl FnMut(&mut Fpdf) -> bool + Send + 'static) {
        self.accept_page_break_hook = Some(Box::new(hook));
    }

    pub(crate) fn run_header(&mut self) {
        if let Some(mut hook) = self.header.take() {
            hook(self);
            if self.header.is_none() {
                self.header = Some(hook);
            }
        }
    }

    pub(crate) fn run_footer(&mut self) {
        if let Some(mut hook) = self.footer.take() {
            hook(self);
            if self.footer.is_none() {
                self.footer = Some(hook);
            }
        }
    }

    pub(crate) fn accept_page_break(&mut self) -> bool {
        match self.accept_page_break_hook.take() {
            Some(mut hook) => {
                let accepted = hook(self);
                if self.accept_page_break_hook.is_none() {
                    self.accept_page_break_hook = Some(hook);
                }
                accepted
            }
            None => self.auto_page_break,
        }
    }

    // ----------------------------------------------------------------- pages

    /// Current page number (0 before the first page).
    pub fn page_no(&self) -> usize {
        self.page
    }

    /// Current page width in user units.
    pub fn page_width(&self) -> f64 {
        self.w
    }

    /// Current page height in user units.
    pub fn page_height(&self) -> f64 {
        self.h
    }

    /// Adds a page with the current orientation, size and rotation.
    pub fn add_page(&mut self) {
        self.add_page_with(self.cur_orientation, self.cur_page_size, self.cur_rotation);
    }

    /// Adds a page with the given orientation.
    pub fn add_page_orientation(&mut self, orientation: Orientation) {
        self.add_page_with(orientation, self.cur_page_size, self.cur_rotation);
    }

    /// Adds a page with the given orientation and standard format.
    pub fn add_page_format(&mut self, orientation: Orientation, format: PageFormat) {
        let pt = format.size_pt();
        let size = PageSize {
            w: pt.w / self.k,
            h: pt.h / self.k,
        };
        self.add_page_with(orientation, size, self.cur_rotation);
    }

    /// Adds a page with an explicit orientation, size (user units, portrait dimensions) and
    /// rotation.
    pub fn add_page_with(&mut self, orientation: Orientation, size: PageSize, rotation: Rotation) {
        if self.state == 3 {
            self.fail("The document is closed");
            return;
        }
        let orientation = if orientation == Orientation::Default {
            self.def_orientation
        } else {
            orientation
        };
        let family = self.font_family.clone();
        let style = self.font_style.clone();
        let underline = self.underline;
        let font_size = self.font_size_pt;
        let line_width = self.line_width;
        let draw_color = self.draw_color.clone();
        let fill_color = self.fill_color.clone();
        let text_color = self.text_color.clone();
        let color_flag = self.color_flag;

        if self.page > 0 {
            self.in_footer = true;
            self.run_footer();
            self.in_footer = false;
            self.end_page();
        }
        self.begin_page(orientation, size, rotation);
        // Square line cap and line width.
        self.out("2 J");
        self.line_width = line_width;
        self.out(&format!("{} w", fixed(line_width * self.k, 2)));
        if !family.is_empty() {
            self.set_font(&family, &style, font_size);
        }
        self.draw_color = draw_color.clone();
        if draw_color != "0 G" {
            self.out(&draw_color);
        }
        self.fill_color = fill_color.clone();
        if fill_color != "0 g" {
            self.out(&fill_color);
        }
        self.text_color = text_color.clone();
        self.color_flag = color_flag;

        self.in_header = true;
        self.run_header();
        self.in_header = false;

        // Restore what the header may have changed.
        if self.line_width != line_width {
            self.line_width = line_width;
            self.out(&format!("{} w", fixed(line_width * self.k, 2)));
        }
        if !family.is_empty() {
            self.set_font(&family, &style, font_size);
        }
        if self.draw_color != draw_color {
            self.draw_color = draw_color.clone();
            self.out(&draw_color);
        }
        if self.fill_color != fill_color {
            self.fill_color = fill_color.clone();
            self.out(&fill_color);
        }
        self.text_color = text_color;
        self.color_flag = color_flag;
        self.underline = underline;
    }

    fn begin_page(&mut self, orientation: Orientation, size: PageSize, rotation: Rotation) {
        self.page += 1;
        self.pages.push(Vec::new());
        self.page_links.push(Vec::new());
        let mut info = PageInfo::default();
        self.state = 2;
        self.x = self.l_margin;
        self.y = self.t_margin;
        self.font_family.clear();

        if orientation != self.cur_orientation
            || size.w != self.cur_page_size.w
            || size.h != self.cur_page_size.h
        {
            let (w, h) = if orientation == Orientation::Landscape {
                (size.h, size.w)
            } else {
                (size.w, size.h)
            };
            self.w = w;
            self.h = h;
            self.w_pt = w * self.k;
            self.h_pt = h * self.k;
            self.page_break_trigger = self.h - self.b_margin;
            self.cur_orientation = orientation;
            self.cur_page_size = size;
        }
        if orientation != self.def_orientation
            || size.w != self.def_page_size.w
            || size.h != self.def_page_size.h
        {
            info.size = Some((self.w_pt, self.h_pt));
        }
        if rotation != Rotation::R0 {
            info.rotation = Some(rotation.degrees());
        }
        self.page_info.push(info);
        self.cur_rotation = rotation;
    }

    pub(crate) fn end_page(&mut self) {
        if self.angle != 0.0 {
            self.angle = 0.0;
            self.out("Q");
        }
        self.end_layer();
        self.state = 1;
    }

    /// Terminates the document. Called implicitly by [`output`](Fpdf::output).
    pub fn close(&mut self) {
        if self.state == 3 {
            return;
        }
        if self.page == 0 {
            self.add_page_with(self.def_orientation, self.def_page_size, Rotation::R0);
        }
        self.in_footer = true;
        self.run_footer();
        self.in_footer = false;
        self.end_page();
        self.end_doc();
    }

    // ---------------------------------------------------------------- colours

    /// Sets the colour of all stroking operations (lines, borders).
    pub fn set_draw_color(&mut self, color: impl Into<Color>) {
        self.draw_color = color_operator(color.into(), "G", "RG");
        if self.page > 0 {
            let op = self.draw_color.clone();
            self.out(&op);
        }
    }

    /// Sets the colour of fills (cell backgrounds, filled rectangles).
    pub fn set_fill_color(&mut self, color: impl Into<Color>) {
        self.fill_color = color_operator(color.into(), "g", "rg");
        self.color_flag = self.fill_color != self.text_color;
        if self.page > 0 {
            let op = self.fill_color.clone();
            self.out(&op);
        }
    }

    /// Sets the text colour.
    pub fn set_text_color(&mut self, color: impl Into<Color>) {
        self.text_color = color_operator(color.into(), "g", "rg");
        self.color_flag = self.fill_color != self.text_color;
    }

    /// Enables or disables underlined text for the current font.
    pub fn set_underline(&mut self, underline: bool) {
        self.underline = underline;
    }

    /// Sets the line width in user units.
    pub fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
        if self.page > 0 {
            self.out(&format!("{} w", fixed(width * self.k, 2)));
        }
    }

    // ------------------------------------------------------------------ fonts

    /// Selects a font. `family` is one of `courier`, `helvetica` (alias `arial`), `times`,
    /// `symbol`, `zapfdingbats` (case-insensitive; empty keeps the current family); `style`
    /// is any combination of `B`, `I` and `U`; `size` is in points (`0.0` keeps the current
    /// size).
    pub fn set_font(&mut self, family: &str, style: &str, size: f64) {
        let mut family = if family.trim().is_empty() {
            self.font_family.clone()
        } else {
            family.to_ascii_lowercase()
        };
        let mut style = style.to_ascii_uppercase();
        if style.contains('U') {
            self.underline = true;
            style = style.replace('U', "");
        } else {
            self.underline = false;
        }
        if style == "IB" {
            style = "BI".to_owned();
        }
        let size = if size == 0.0 { self.font_size_pt } else { size };
        if family == "arial" {
            family = "helvetica".to_owned();
        }
        if family == "symbol" || family == "zapfdingbats" {
            style.clear();
        }
        if self.font_family == family && self.font_style == style && self.font_size_pt == size {
            return;
        }
        let style_name = format!(
            "{}{}",
            if style.contains('B') { "Bold" } else { "" },
            if style.contains('I') { "Oblique" } else { "" }
        );
        let font_name = if style_name.is_empty() {
            family.clone()
        } else {
            format!("{family}-{style_name}")
        };
        let Some(index) = self
            .fonts
            .iter()
            .position(|f| f.font_name.eq_ignore_ascii_case(&font_name))
        else {
            self.fail(format!("Undefined font: {font_name}"));
            return;
        };
        self.font_family = family;
        self.font_style = style;
        self.font_size_pt = size;
        self.font_size = size / self.k;
        self.current_font = Some(index);
        self.current_font_key = font_name.clone();

        let font_index = match self.find_used_font(&font_name) {
            Some(i) => i,
            None => {
                self.used_fonts.push(UsedFont { font_name, n: 0 });
                self.used_fonts.len()
            }
        };
        if self.page > 0 {
            let size = fixed(self.font_size_pt, 2);
            self.out(&format!("BT /F{font_index} {size} Tf ET"));
        }
    }

    /// Changes only the font size (points).
    pub fn set_font_size(&mut self, size: f64) {
        if self.font_size_pt == size {
            return;
        }
        self.font_size_pt = size;
        self.font_size = size / self.k;
        if self.page > 0 && self.current_font.is_some() {
            let key = self.current_font_key.clone();
            let index = self.find_used_font(&key).unwrap_or(0);
            let size = fixed(self.font_size_pt, 2);
            self.out(&format!("BT /F{index} {size} Tf ET"));
        }
    }

    /// 1-based index of a used font.
    pub(crate) fn find_used_font(&self, font_name: &str) -> Option<usize> {
        self.used_fonts
            .iter()
            .position(|f| f.font_name.eq_ignore_ascii_case(font_name))
            .map(|i| i + 1)
    }

    /// Current font family (lowercase, as selected).
    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    /// Current font size in user units.
    pub fn font_size(&self) -> f64 {
        self.font_size
    }

    /// Current font size in points.
    pub fn font_size_pt(&self) -> f64 {
        self.font_size_pt
    }

    /// The colour operator currently used for text (`0.000 g`, `1.000 0.000 0.000 rg`...).
    pub fn text_color_operator(&self) -> &str {
        &self.text_color
    }

    /// Restores a text colour operator previously read with
    /// [`text_color_operator`](Fpdf::text_color_operator).
    pub fn set_text_color_operator(&mut self, operator: &str) {
        self.text_color = operator.to_owned();
        self.color_flag = self.fill_color != self.text_color;
    }

    /// Width of `text` in user units with the current font (the widest line when the text
    /// has several). Returns `0.0` when no font is selected.
    pub fn get_string_width(&self, text: &str) -> f64 {
        self.string_width(&to_cp1252(text))
    }

    pub(crate) fn string_width(&self, text: &[u8]) -> f64 {
        let Some(font) = self.current_font.and_then(|i| self.fonts.get(i)) else {
            return 0.0;
        };
        let widest = text
            .split(|b| *b == b'\n')
            .map(|line| {
                line.iter()
                    .filter(|b| **b != b'\r')
                    .map(|b| u32::from(font.cw[usize::from(*b)]))
                    .sum::<u32>()
            })
            .max()
            .unwrap_or(0);
        f64::from(widest) * self.font_size / 1000.0
    }

    // ------------------------------------------------------------------ links

    /// Creates a new internal link and returns its identifier.
    pub fn add_link(&mut self) -> usize {
        self.links.push(LinkDest::default());
        self.links.len() - 1
    }

    /// Defines the destination of an internal link: vertical position `y` (`None` = current
    /// position) on `page` (`None` = current page).
    pub fn set_link(&mut self, link: usize, y: Option<f64>, page: Option<usize>) {
        if link >= self.links.len() {
            self.fail(format!("Invalid Link Index: {link}"));
            return;
        }
        let y = y.unwrap_or(self.y);
        let page = page.unwrap_or(self.page);
        if page > self.pages.len() {
            self.fail(format!("Invalid Link Page: {page}"));
            return;
        }
        self.links[link] = LinkDest { page, y };
    }

    /// Puts a link on a rectangular area of the current page.
    pub fn link(&mut self, x: f64, y: f64, width: f64, height: f64, link: &Link) {
        if let Link::Internal(id) = link {
            if *id >= self.links.len() {
                self.fail(format!("Invalid Link Index: {id}"));
                return;
            }
        }
        let Some(page_links) = self
            .page
            .checked_sub(1)
            .and_then(|p| self.page_links.get_mut(p))
        else {
            self.fail("No page has been added yet");
            return;
        };
        page_links.push(PageLink {
            x: x * self.k,
            y: self.h_pt - y * self.k,
            width: width * self.k,
            height: height * self.k,
            link: link.clone(),
            n: 0,
        });
    }

    // --------------------------------------------------------------- position

    /// Current x position.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Current y position.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Sets x; a negative value is measured from the right edge of the page.
    pub fn set_x(&mut self, x: f64) {
        self.x = if x >= 0.0 { x } else { self.w + x };
    }

    /// Sets y (negative = from the bottom edge) and optionally resets x to the left margin.
    pub fn set_y(&mut self, y: f64, reset_x: bool) {
        self.y = if y >= 0.0 { y } else { self.h + y };
        if reset_x {
            self.x = self.l_margin;
        }
    }

    /// Sets x and y.
    pub fn set_xy(&mut self, x: f64, y: f64) {
        self.set_x(x);
        self.set_y(y, false);
    }

    // ----------------------------------------------------------------- output

    /// Closes the document and returns the PDF bytes (the first recorded error otherwise).
    pub fn output(&mut self) -> Result<Vec<u8>, FpdfError> {
        self.close();
        match &self.error {
            Some(e) => Err(e.clone()),
            None => Ok(self.buffer.clone()),
        }
    }

    /// Closes the document and writes it to `path`.
    pub fn save_to_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), FpdfError> {
        let bytes = self.output()?;
        std::fs::write(path.as_ref(), bytes).map_err(|e| {
            FpdfError::io(
                format!("Unable to create output file: {}", path.as_ref().display()),
                &e,
            )
        })
    }
}

/// Builds the colour operator string (`0.500 g`, `1.000 0.000 0.000 rg`, ...).
fn color_operator(color: Color, gray_op: &str, rgb_op: &str) -> String {
    let (r, g, b) = color.components();
    if color.is_gray() {
        format!("{} {gray_op}", fixed(f64::from(r) / 255.0, 3))
    } else {
        format!(
            "{} {} {} {rgb_op}",
            fixed(f64::from(r) / 255.0, 3),
            fixed(f64::from(g) / 255.0, 3),
            fixed(f64::from(b) / 255.0, 3)
        )
    }
}

/// Current UTC date and time.
pub(crate) fn now_utc() -> DateTime {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
        .unwrap_or(0);
    from_unix(secs)
}

/// Converts Unix seconds to a civil UTC date (Howard Hinnant's algorithm).
pub(crate) fn from_unix(secs: i64) -> DateTime {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    DateTime {
        year,
        month,
        day,
        hour: (rem / 3600) as u32,
        minute: (rem % 3600 / 60) as u32,
        second: (rem % 60) as u32,
    }
}

pub(crate) fn pdf_version_string(version: f64) -> String {
    float_to_str(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_unix_time() {
        let d = from_unix(0);
        assert_eq!(
            (d.year, d.month, d.day, d.hour, d.minute, d.second),
            (1970, 1, 1, 0, 0, 0)
        );
        let d = from_unix(1_700_000_000);
        assert_eq!(
            (d.year, d.month, d.day, d.hour, d.minute, d.second),
            (2023, 11, 14, 22, 13, 20)
        );
        let d = from_unix(951_782_400); // 2000-02-29
        assert_eq!((d.year, d.month, d.day), (2000, 2, 29));
    }

    #[test]
    fn colour_operators() {
        assert_eq!(color_operator(Color::BLACK, "g", "rg"), "0.000 g");
        assert_eq!(color_operator(Color::Rgb(0, 0, 0), "g", "rg"), "0.000 g");
        assert_eq!(color_operator(Color::Gray(128), "G", "RG"), "0.502 G");
        assert_eq!(
            color_operator(Color::Rgb(255, 0, 128), "g", "rg"),
            "1.000 0.000 0.502 rg"
        );
    }
}
