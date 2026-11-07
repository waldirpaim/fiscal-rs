//! Band-based report engine (port of FPDF-Pascal's `ACBr_fpdf_report`, by Arimateia Jr —
//! Nuvem Fiscal).
//!
//! A [`Report`] holds [`Page`]s; every page holds [`Band`]s (page header/footer, margin
//! bands, watermark, overlay and *data* bands that flow across pages). The [`Engine`] lays
//! the bands out, breaking pages when a data band needs more room than is left, and — with
//! the double-pass option — knows the total number of pages while drawing the first one.
//!
//! Bands are implemented with the [`BandHandler`] trait: `on_init` sizes the band (and may
//! hide it) before each page layout, `on_draw` draws it through a [`ReportPdf`] whose
//! coordinates are relative to the band's top-left corner.

mod engine;
mod image_util;
mod wrapper;

pub use engine::Engine;
pub use image_util::{ImgType, image_size, image_type};
pub use wrapper::ReportPdf;

use crate::error::FpdfError;
use crate::types::{Orientation, PageFormat, PageSize, Unit};

/// Kind of band; decides where and when the engine draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandType {
    /// Reserves space on the left of every page.
    LeftMargin,
    /// Reserves space at the top of every page.
    TopMargin,
    /// Reserves space on the right of every page.
    RightMargin,
    /// Reserves space at the bottom of every page.
    BottomMargin,
    /// Flowing content: drawn in order, breaking to a new page when it does not fit.
    Data,
    /// Drawn once, at the top of the first page.
    ReportHeader,
    /// Drawn at the top of every page.
    PageHeader,
    /// Drawn after the data bands.
    ReportFooter,
    /// Drawn at the bottom of every page.
    PageFooter,
    /// Drawn behind everything, covering the page body.
    Watermark,
    /// Drawn on top of everything, covering the page body.
    Overlay,
}

pub(crate) const MARGIN_BANDS: [BandType; 4] = [
    BandType::LeftMargin,
    BandType::TopMargin,
    BandType::RightMargin,
    BandType::BottomMargin,
];
pub(crate) const FIRST_BANDS: [BandType; 2] = [BandType::ReportHeader, BandType::PageHeader];
pub(crate) const MIDDLE_BANDS: [BandType; 1] = [BandType::Data];
pub(crate) const LAST_BANDS: [BandType; 1] = [BandType::PageFooter];

/// Mutable properties of a band (what the handler may change).
#[derive(Debug, Clone)]
pub struct BandState {
    /// Kind of band.
    pub band_type: BandType,
    /// Name used in error messages.
    pub name: String,
    /// When set, the space used is what the band actually drew instead of [`height`].
    ///
    /// [`height`]: BandState::height
    pub auto_height: bool,
    /// Height in user units.
    pub height: f64,
    /// Width in user units (set by the engine from the page).
    pub width: f64,
    /// Hidden bands are skipped.
    pub visible: bool,
}

/// Information given to [`BandHandler::on_init`].
#[derive(Debug, Clone, Copy)]
pub struct InitArgs {
    /// Orientation of the page.
    pub orientation: Orientation,
    /// Page width.
    pub page_width: f64,
    /// Page height.
    pub page_height: f64,
    /// Current left margin (including margin bands).
    pub left_margin: f64,
    /// Current top margin.
    pub top_margin: f64,
    /// Current right margin.
    pub right_margin: f64,
    /// Current bottom margin.
    pub bottom_margin: f64,
    /// PDF page being produced (0 before the first page).
    pub current_page: usize,
    /// `true` during the final pass.
    pub final_pass: bool,
    /// Total number of pages (known only in the final pass of a double-pass report).
    pub total_pages: usize,
}

/// Everything a band needs to draw itself.
pub struct DrawArgs<'a> {
    /// Drawing surface; coordinates are relative to the top-left corner of the band.
    pub pdf: ReportPdf<'a>,
    /// Orientation of the page.
    pub orientation: Orientation,
    /// Page width.
    pub page_width: f64,
    /// Page height.
    pub page_height: f64,
    /// Current left margin (including margin bands).
    pub left_margin: f64,
    /// Current top margin.
    pub top_margin: f64,
    /// Current right margin.
    pub right_margin: f64,
    /// Current bottom margin.
    pub bottom_margin: f64,
    /// PDF page being produced.
    pub current_page: usize,
    /// `true` during the final pass.
    pub final_pass: bool,
    /// Total number of pages (known only in the final pass of a double-pass report).
    pub total_pages: usize,
    /// Vertical space left on the page for data bands.
    pub free_space: f64,
    /// Space the engine wants to keep for the data bands that follow this one.
    pub reserved_space: f64,
    /// Set to `true` to be drawn again on the next page.
    pub draw_again: bool,
}

/// Behaviour of a band.
pub trait BandHandler: Send {
    /// Called before every page layout: set the height, visibility...
    fn on_init(&mut self, band: &mut BandState, args: &InitArgs) {
        let _ = (band, args);
    }

    /// Draws the band.
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let _ = (band, args);
    }
}

struct DrawFn<F>(F);

impl<F> BandHandler for DrawFn<F>
where
    F: FnMut(&mut BandState, &mut DrawArgs<'_>) + Send,
{
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        (self.0)(band, args);
    }
}

/// A band: its state plus the handler that draws it.
pub struct Band {
    /// Mutable properties.
    pub state: BandState,
    pub(crate) handler: Box<dyn BandHandler>,
}

impl Band {
    /// Creates a band of `band_type` driven by `handler`. The band is visible and has no
    /// height until the handler's `on_init` sets one.
    pub fn new(band_type: BandType, handler: impl BandHandler + 'static) -> Self {
        Self {
            state: BandState {
                band_type,
                name: std::any::type_name_of_val(&handler).to_owned(),
                auto_height: false,
                height: 0.0,
                width: 0.0,
                visible: true,
            },
            handler: Box::new(handler),
        }
    }

    /// Creates a fixed-height band drawn by a closure.
    pub fn anonymous(
        band_type: BandType,
        height: f64,
        draw: impl FnMut(&mut BandState, &mut DrawArgs<'_>) + Send + 'static,
    ) -> Self {
        let mut band = Self::new(band_type, DrawFn(draw));
        band.state.height = height;
        band.state.name = "AnonymousBand".to_owned();
        band
    }
}

/// A page of a report.
pub struct Page {
    /// Orientation.
    pub orientation: Orientation,
    /// Unit of every dimension of the page and its bands.
    pub page_unit: Unit,
    /// Height in `page_unit`.
    pub page_height: f64,
    /// Width in `page_unit`.
    pub page_width: f64,
    /// A page as tall as its content (single page, e.g. a receipt).
    pub endless_height: bool,
    /// Hidden pages are kept in the report but have no effect.
    pub visible: bool,
    /// Current top margin (margins plus margin bands).
    pub top_margin: f64,
    /// Current bottom margin.
    pub bottom_margin: f64,
    /// Current left margin.
    pub left_margin: f64,
    /// Current right margin.
    pub right_margin: f64,
    pub(crate) def_top_margin: f64,
    pub(crate) def_bottom_margin: f64,
    pub(crate) def_left_margin: f64,
    pub(crate) def_right_margin: f64,
    pub(crate) bands: Vec<Band>,
}

impl Page {
    /// A page with explicit width and height.
    pub fn with_dimensions(orientation: Orientation, unit: Unit, width: f64, height: f64) -> Self {
        Self {
            orientation,
            page_unit: unit,
            page_height: height,
            page_width: width,
            endless_height: false,
            visible: true,
            top_margin: 0.0,
            bottom_margin: 0.0,
            left_margin: 0.0,
            right_margin: 0.0,
            def_top_margin: 0.0,
            def_bottom_margin: 0.0,
            def_left_margin: 0.0,
            def_right_margin: 0.0,
            bands: Vec::new(),
        }
    }

    /// A page of the given size (swapped when landscape is requested for a portrait size).
    pub fn with_size(orientation: Orientation, unit: Unit, size: PageSize) -> Self {
        if orientation == Orientation::Landscape && size.h > size.w {
            Self::with_dimensions(orientation, unit, size.h, size.w)
        } else {
            Self::with_dimensions(orientation, unit, size.w, size.h)
        }
    }

    /// A page of a standard format.
    pub fn new(orientation: Orientation, unit: Unit, format: PageFormat) -> Self {
        let scale = unit.scale();
        let pt = format.size_pt();
        Self::with_size(
            orientation,
            unit,
            PageSize {
                w: pt.w / scale,
                h: pt.h / scale,
            },
        )
    }

    /// Adds a band.
    pub fn add_band(&mut self, band: Band) -> &mut Band {
        self.bands.push(band);
        let last = self.bands.len() - 1;
        &mut self.bands[last]
    }

    /// Adds a fixed-height band drawn by a closure.
    pub fn add_anonymous_band(
        &mut self,
        band_type: BandType,
        height: f64,
        draw: impl FnMut(&mut BandState, &mut DrawArgs<'_>) + Send + 'static,
    ) -> &mut Band {
        self.add_band(Band::anonymous(band_type, height, draw))
    }
}

/// Report-level options.
#[derive(Debug, Clone, Default)]
pub struct ReportOptions {
    /// Document author.
    pub author: String,
}

/// Engine options.
#[derive(Debug, Clone, Copy, Default)]
pub struct EngineOptions {
    /// Run a first pass to learn the total number of pages (and the height of endless
    /// pages) before drawing.
    pub double_pass: bool,
}

/// What the report hooks learn about the engine.
#[derive(Debug, Clone, Copy)]
pub struct EngineEvent {
    /// PDF page being produced.
    pub current_page: usize,
    /// `true` during the final pass.
    pub final_pass: bool,
    /// Total number of pages (known only in the final pass of a double-pass report).
    pub total_pages: usize,
}

/// Callback run when a report starts or ends.
pub type ReportHook = Box<dyn FnMut(&mut Report, &EngineEvent) + Send>;

/// Margins applied to pages added with [`Report::add_page`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Margins {
    /// Left.
    pub left: f64,
    /// Top.
    pub top: f64,
    /// Right.
    pub right: f64,
    /// Bottom.
    pub bottom: f64,
}

/// A report: pages with bands.
#[derive(Default)]
pub struct Report {
    pub(crate) pages: Vec<Page>,
    /// Report options.
    pub options: ReportOptions,
    /// Engine options.
    pub engine_options: EngineOptions,
    default_margins: Margins,
    default_font_family: String,
    on_start: Option<ReportHook>,
    on_end: Option<ReportHook>,
}

impl Report {
    /// An empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Default margins of the pages added afterwards; `None` right/bottom copy left/top.
    pub fn set_margins(&mut self, left: f64, top: f64, right: Option<f64>, bottom: Option<f64>) {
        self.default_margins = Margins {
            left,
            top,
            right: right.unwrap_or(left),
            bottom: bottom.unwrap_or(top),
        };
    }

    /// Font family selected before the first page is drawn.
    pub fn set_font(&mut self, family: &str) {
        self.default_font_family = family.to_owned();
    }

    pub(crate) fn default_font_family(&self) -> &str {
        &self.default_font_family
    }

    /// Installs a hook run when the engine starts drawing the report (in every pass); the
    /// usual place to add the pages and bands lazily.
    pub fn set_on_start(&mut self, hook: impl FnMut(&mut Report, &EngineEvent) + Send + 'static) {
        self.on_start = Some(Box::new(hook));
    }

    /// Installs a hook run when the engine finishes drawing the report (in every pass).
    pub fn set_on_end(&mut self, hook: impl FnMut(&mut Report, &EngineEvent) + Send + 'static) {
        self.on_end = Some(Box::new(hook));
    }

    pub(crate) fn notify_start(&mut self, event: &EngineEvent) {
        if let Some(mut hook) = self.on_start.take() {
            hook(self, event);
            if self.on_start.is_none() {
                self.on_start = Some(hook);
            }
        }
    }

    pub(crate) fn notify_end(&mut self, event: &EngineEvent) {
        if let Some(mut hook) = self.on_end.take() {
            hook(self, event);
            if self.on_end.is_none() {
                self.on_end = Some(hook);
            }
        }
    }

    /// Adds a page of a standard format with the default margins.
    pub fn add_page(
        &mut self,
        orientation: Orientation,
        unit: Unit,
        format: PageFormat,
    ) -> &mut Page {
        self.add_page_object(Page::new(orientation, unit, format), true)
    }

    /// Adds a page with an explicit size and the default margins.
    pub fn add_page_with_size(
        &mut self,
        orientation: Orientation,
        unit: Unit,
        size: PageSize,
    ) -> &mut Page {
        self.add_page_object(Page::with_size(orientation, unit, size), true)
    }

    /// Adds a page with explicit width and height and the default margins.
    pub fn add_page_with_dimensions(
        &mut self,
        orientation: Orientation,
        unit: Unit,
        width: f64,
        height: f64,
    ) -> &mut Page {
        self.add_page_object(
            Page::with_dimensions(orientation, unit, width, height),
            true,
        )
    }

    /// Adds a ready-made page, optionally applying the default margins.
    pub fn add_page_object(&mut self, mut page: Page, default_margins: bool) -> &mut Page {
        if default_margins {
            let m = self.default_margins;
            page.def_left_margin = m.left;
            page.def_top_margin = m.top;
            page.def_right_margin = m.right;
            page.def_bottom_margin = m.bottom;
            page.left_margin = m.left;
            page.top_margin = m.top;
            page.right_margin = m.right;
            page.bottom_margin = m.bottom;
        }
        self.pages.push(page);
        let last = self.pages.len() - 1;
        &mut self.pages[last]
    }

    /// Adds a band to the last page.
    pub fn add_band(&mut self, band: Band) -> Result<&mut Band, FpdfError> {
        let page = self.last_page()?;
        Ok(page.add_band(band))
    }

    /// Adds a fixed-height band drawn by a closure to the last page.
    pub fn add_anonymous_band(
        &mut self,
        band_type: BandType,
        height: f64,
        draw: impl FnMut(&mut BandState, &mut DrawArgs<'_>) + Send + 'static,
    ) -> Result<&mut Band, FpdfError> {
        let page = self.last_page()?;
        Ok(page.add_anonymous_band(band_type, height, draw))
    }

    fn last_page(&mut self) -> Result<&mut Page, FpdfError> {
        self.pages
            .last_mut()
            .ok_or_else(|| FpdfError::msg("FPDFReport error: No page has been added yet"))
    }

    /// The pages of the report.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub(crate) fn has_endless_page(&self) -> bool {
        self.pages.iter().any(|p| p.endless_height)
    }
}
