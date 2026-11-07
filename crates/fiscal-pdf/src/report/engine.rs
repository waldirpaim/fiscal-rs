//! The layout engine of the report (`TFPDFEngine`).

use super::wrapper::ReportPdf;
use super::{
    BandType, DrawArgs, EngineEvent, FIRST_BANDS, InitArgs, LAST_BANDS, MARGIN_BANDS, MIDDLE_BANDS,
    Report,
};
use crate::document::Fpdf;
use crate::error::FpdfError;
use crate::types::{Orientation, PageFormat, PageSize, Rotation, Unit};

/// Safety net against layouts that never converge (a band taller than any page).
const MAX_PAGES: usize = 2_000;

/// Lays a [`Report`] out and renders it to PDF.
pub struct Engine<'a> {
    report: &'a mut Report,
    pdf: Fpdf,
    compressed: bool,
    double_pass: bool,
    final_pass: bool,
    free_space: f64,
    /// Indices (in the current page) of the data bands still to be drawn.
    active_middle: Vec<usize>,
    total_pages: usize,
    endless_height: f64,
    break_page: bool,
    in_margins: bool,
    error: Option<FpdfError>,
}

fn is_margin(band_type: BandType) -> bool {
    MARGIN_BANDS.contains(&band_type)
}

fn is_middle_or_report_footer(band_type: BandType) -> bool {
    MIDDLE_BANDS.contains(&band_type) || band_type == BandType::ReportFooter
}

impl<'a> Engine<'a> {
    /// Creates an engine for `report` (compressed output, no extra pass unless the report
    /// asks for it).
    pub fn new(report: &'a mut Report) -> Self {
        let double_pass = report.engine_options.double_pass || report.has_endless_page();
        let mut pdf = match report.pages.first() {
            Some(page) => Fpdf::with_size(
                page.orientation,
                page.page_unit,
                PageSize {
                    w: page.page_width,
                    h: page.page_height,
                },
            ),
            None => Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4),
        };
        if !report.default_font_family().is_empty() {
            let family = report.default_font_family().to_owned();
            pdf.set_font(&family, "", 12.0);
        }
        Self {
            report,
            pdf,
            compressed: true,
            double_pass,
            final_pass: false,
            free_space: 0.0,
            active_middle: Vec::new(),
            total_pages: 0,
            endless_height: 0.0,
            break_page: false,
            in_margins: false,
            error: None,
        }
    }

    /// Enables or disables content compression (enabled by default).
    pub fn set_compressed(&mut self, compressed: bool) {
        self.compressed = compressed;
    }

    /// Gives access to the underlying document, e.g. to set metadata.
    pub fn pdf_mut(&mut self) -> &mut Fpdf {
        &mut self.pdf
    }

    /// Runs the layout (a first pass first when double pass is on) and returns the PDF.
    pub fn render(&mut self) -> Result<Vec<u8>, FpdfError> {
        self.run_first_pass();
        self.pdf.set_compression(self.compressed);
        self.draw_report();
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        self.pdf.output()
    }

    fn fail(&mut self, message: impl Into<String>) {
        if self.error.is_none() {
            self.error = Some(FpdfError::msg(format!(
                "FPDFReport error: {}",
                message.into()
            )));
        }
    }

    fn event(&self) -> EngineEvent {
        EngineEvent {
            current_page: self.pdf.page_no(),
            final_pass: self.final_pass,
            total_pages: self.total_pages,
        }
    }

    fn run_first_pass(&mut self) {
        self.total_pages = 0;
        self.endless_height = 0.0;
        if self.double_pass {
            let mut first = Engine::new(&mut *self.report);
            first.double_pass = false;
            first.draw_report();
            let (pages, height, error) = (
                first.pdf.page_no(),
                first.endless_height,
                first.error.take(),
            );
            drop(first);
            self.total_pages = pages;
            self.endless_height = height;
            if let Some(e) = error {
                self.error = Some(e);
            }
        }
        self.final_pass = true;
    }

    // ---- bands lookup -----------------------------------------------------------------

    /// Indices of the bands of the page whose type is in `types`, in page order.
    fn get_bands(&self, page: usize, types: &[BandType]) -> Vec<usize> {
        self.report.pages[page]
            .bands
            .iter()
            .enumerate()
            .filter(|(_, b)| types.contains(&b.state.band_type))
            .map(|(i, _)| i)
            .collect()
    }

    // ---- layout -----------------------------------------------------------------------

    fn draw_report(&mut self) {
        let event = self.event();
        self.report.notify_start(&event);
        for page in 0..self.report.pages.len() {
            if self.error.is_some() {
                return;
            }
            self.init_page_bands(page);
            self.draw_page(page);
        }
        let event = self.event();
        self.report.notify_end(&event);
    }

    fn init_page_bands(&mut self, page: usize) {
        self.init_bands(page, &MARGIN_BANDS);
        self.calculate_page_margins(page);
        self.active_middle.clear();
        let merged: Vec<BandType> = FIRST_BANDS
            .iter()
            .chain(&MIDDLE_BANDS)
            .chain(&LAST_BANDS)
            .copied()
            .collect();
        self.init_bands(page, &merged);
        self.init_bands(page, &[BandType::ReportFooter]);
        if self.report.has_endless_page() {
            self.endless_height += self.report.pages[page].top_margin;
        }
    }

    fn init_bands(&mut self, page: usize, types: &[BandType]) {
        for index in self.get_bands(page, types) {
            let args = {
                let p = &self.report.pages[page];
                InitArgs {
                    orientation: p.orientation,
                    page_width: p.page_width,
                    page_height: p.page_height,
                    left_margin: p.left_margin,
                    top_margin: p.top_margin,
                    right_margin: p.right_margin,
                    bottom_margin: p.bottom_margin,
                    current_page: self.pdf.page_no(),
                    final_pass: self.final_pass,
                    total_pages: self.total_pages,
                }
            };
            let band = &mut self.report.pages[page].bands[index];
            band.handler.on_init(&mut band.state, &args);
            let (visible, height, band_type) =
                (band.state.visible, band.state.height, band.state.band_type);
            if visible {
                if !is_margin(band_type) && height <= 0.0 {
                    let name = band.state.name.clone();
                    self.fail(format!("Field {name}.Height must be greater than 0"));
                    return;
                }
                if is_middle_or_report_footer(band_type) {
                    self.active_middle.push(index);
                }
            }
        }
    }

    fn calculate_page_margins(&mut self, page: usize) {
        let (mut left, mut top, mut right, mut bottom) = {
            let p = &self.report.pages[page];
            (
                p.def_left_margin,
                p.def_top_margin,
                p.def_right_margin,
                p.def_bottom_margin,
            )
        };
        for index in self.get_bands(page, &MARGIN_BANDS) {
            let state = &self.report.pages[page].bands[index].state;
            if !state.visible {
                continue;
            }
            match state.band_type {
                BandType::LeftMargin => left += state.width,
                BandType::TopMargin => top += state.height,
                BandType::RightMargin => right += state.width,
                BandType::BottomMargin => bottom += state.height,
                _ => {}
            }
        }
        let p = &mut self.report.pages[page];
        p.left_margin = left;
        p.top_margin = top;
        p.right_margin = right;
        p.bottom_margin = bottom;
        self.calculate_band_dimensions(page);
    }

    fn calculate_band_dimensions(&mut self, page: usize) {
        let p = &mut self.report.pages[page];
        let (page_w, page_h) = (p.page_width, p.page_height);
        let (dl, dt, dr, db) = (
            p.def_left_margin,
            p.def_top_margin,
            p.def_right_margin,
            p.def_bottom_margin,
        );
        let (l, t, r, b) = (p.left_margin, p.top_margin, p.right_margin, p.bottom_margin);
        for band in &mut p.bands {
            let state = &mut band.state;
            if is_margin(state.band_type) {
                match state.band_type {
                    BandType::LeftMargin | BandType::RightMargin => state.height = page_h - dt - db,
                    BandType::TopMargin | BandType::BottomMargin => state.width = page_w - dl - dr,
                    _ => {}
                }
            } else {
                state.width = page_w - l - r;
            }
            if matches!(state.band_type, BandType::Watermark | BandType::Overlay) {
                state.height = page_h - t - b;
            }
        }
    }

    fn draw_page(&mut self, page: usize) {
        loop {
            if self.error.is_some() {
                return;
            }
            if self.pdf.page_no() >= MAX_PAGES {
                self.fail("the layout does not converge (a band never fits a page)");
                return;
            }
            self.start_new_page(page);
            self.draw_first_bands(page);
            self.draw_middle_bands(page);
            self.draw_last_bands(page);
            self.draw_bands(page, &[BandType::Overlay]);
            if self.active_middle.is_empty() {
                break;
            }
        }
    }

    fn start_new_page(&mut self, page: usize) {
        self.calculate_page_margins(page);
        let (left, top, right, bottom) = {
            let p = &self.report.pages[page];
            (p.left_margin, p.top_margin, p.right_margin, p.bottom_margin)
        };
        self.pdf.set_margins(left, top, Some(right));
        self.pdf.set_auto_page_break(false, bottom);

        if self.final_pass && self.report.has_endless_page() {
            self.report.pages[page].page_height = self.endless_height;
        }
        let p = &self.report.pages[page];
        let size = if p.orientation == Orientation::Landscape {
            PageSize {
                w: p.page_height,
                h: p.page_width,
            }
        } else {
            PageSize {
                w: p.page_width,
                h: p.page_height,
            }
        };
        let orientation = p.orientation;
        self.pdf.add_page_with(orientation, size, Rotation::R0);

        self.draw_bands(page, &[BandType::Watermark]);
        self.in_margins = true;
        for band_type in MARGIN_BANDS {
            self.draw_bands(page, &[band_type]);
        }
        self.in_margins = false;

        let p = &self.report.pages[page];
        self.pdf.set_xy(p.left_margin, p.top_margin);
        self.free_space = if self.report.has_endless_page() {
            1e6
        } else {
            p.page_height - p.top_margin - p.bottom_margin
        };
    }

    fn draw_first_bands(&mut self, page: usize) {
        for band_type in FIRST_BANDS {
            if band_type == BandType::ReportHeader && self.pdf.page_no() > 1 {
                continue;
            }
            self.draw_bands(page, &[band_type]);
        }
    }

    fn draw_middle_bands(&mut self, page: usize) {
        // Space the page footers take is not available to data bands.
        for index in self.get_bands(page, &LAST_BANDS) {
            let state = &self.report.pages[page].bands[index].state;
            if state.visible {
                self.free_space -= state.height;
            }
        }
        while let Some(&index) = self.active_middle.first() {
            if self.error.is_some() {
                return;
            }
            let visible = self.report.pages[page].bands[index].state.visible;
            self.draw_band(page, index);
            if self.break_page {
                self.break_page = false;
                break;
            }
            // A band that turned invisible can never leave the active list on its own.
            if !visible && self.active_middle.first() == Some(&index) {
                self.active_middle.remove(0);
            }
        }
    }

    fn draw_last_bands(&mut self, page: usize) {
        if !self.report.has_endless_page() {
            let total_height: f64 = self
                .get_bands(page, &LAST_BANDS)
                .into_iter()
                .map(|i| &self.report.pages[page].bands[i].state)
                .filter(|s| s.visible)
                .map(|s| s.height)
                .sum();
            // Print on the bottom of the page.
            if total_height > 0.0 {
                let p = &self.report.pages[page];
                self.pdf.set_xy(
                    p.left_margin,
                    p.page_height - p.bottom_margin - total_height,
                );
            }
        }
        for band_type in LAST_BANDS {
            self.draw_bands(page, &[band_type]);
        }
    }

    fn draw_bands(&mut self, page: usize, types: &[BandType]) {
        for index in self.get_bands(page, types) {
            self.draw_band(page, index);
            if self.break_page {
                self.break_page = false;
                break;
            }
        }
    }

    fn draw_band(&mut self, page: usize, index: usize) {
        let (band_type, height, visible) = {
            let s = &self.report.pages[page].bands[index].state;
            (s.band_type, s.height, s.visible)
        };
        if band_type == BandType::ReportHeader && self.pdf.page_no() > 1 {
            return;
        }
        let endless = self.report.has_endless_page();
        let middle = is_middle_or_report_footer(band_type);
        if middle && self.free_space < height && !endless {
            self.break_page = true;
            return;
        }
        if !visible {
            return;
        }

        let (prev_x, prev_y) = (self.pdf.x(), self.pdf.y());
        {
            let p = &self.report.pages[page];
            match band_type {
                BandType::TopMargin | BandType::LeftMargin => {
                    self.pdf.set_xy(p.def_left_margin, p.def_top_margin);
                }
                BandType::RightMargin => {
                    self.pdf
                        .set_xy(p.page_width - p.right_margin, p.def_top_margin);
                }
                BandType::BottomMargin => {
                    self.pdf
                        .set_xy(p.def_left_margin, p.page_height - p.bottom_margin);
                }
                BandType::Watermark | BandType::Overlay => {
                    self.pdf.set_xy(p.left_margin, p.top_margin);
                }
                _ => {}
            }
        }

        // Space to keep for the data bands that still follow this one.
        let mut reserved = 0.0;
        if middle && self.active_middle.len() > 1 {
            let start = self
                .active_middle
                .iter()
                .position(|b| *b == index)
                .map_or(0, |p| p + 1);
            for &next in &self.active_middle[start..] {
                let next_height = self.report.pages[page].bands[next].state.height;
                if height > self.free_space - reserved - next_height {
                    break;
                }
                reserved += next_height;
            }
        }

        let offset_enabled = !matches!(band_type, BandType::Watermark | BandType::Overlay);
        let (draw_again, modified, highest_y, used_height, auto_height) = {
            let Self {
                report,
                pdf,
                final_pass,
                total_pages,
                free_space,
                ..
            } = self;
            let p = &mut report.pages[page];
            let snapshot = (
                p.orientation,
                p.page_width,
                p.page_height,
                p.left_margin,
                p.top_margin,
                p.right_margin,
                p.bottom_margin,
            );
            let current_page = pdf.page_no();
            let mut args = DrawArgs {
                pdf: ReportPdf::new(pdf, offset_enabled),
                orientation: snapshot.0,
                page_width: snapshot.1,
                page_height: snapshot.2,
                left_margin: snapshot.3,
                top_margin: snapshot.4,
                right_margin: snapshot.5,
                bottom_margin: snapshot.6,
                current_page,
                final_pass: *final_pass,
                total_pages: *total_pages,
                free_space: *free_space,
                reserved_space: reserved,
                draw_again: false,
            };
            let band = &mut p.bands[index];
            band.handler.on_draw(&mut band.state, &mut args);
            (
                args.draw_again,
                args.pdf.modified,
                args.pdf.highest_y,
                band.state.height,
                band.state.auto_height,
            )
        };

        if !self.in_margins {
            if modified {
                let used = if auto_height {
                    highest_y - prev_y
                } else {
                    used_height
                };
                self.free_space -= used;
                if endless {
                    self.endless_height += used;
                }
                self.pdf.set_xy(prev_x, prev_y + used);
            }
            if !draw_again {
                if let Some(position) = self.active_middle.iter().position(|b| *b == index) {
                    self.active_middle.remove(position);
                }
            }
        }
    }
}
