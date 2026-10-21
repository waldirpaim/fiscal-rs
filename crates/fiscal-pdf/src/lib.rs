//! A pure-Rust PDF engine with the FPDF programming model, used to render fiscal auxiliary
//! documents (DANFE, DANFCE...).
//!
//! `fiscal-pdf` is a port of [FPDF-Pascal](https://github.com/Projeto-ACBr-Oficial/FPDF-Pascal)
//! (which is itself a translation of Olivier Plathey's [FPDF](http://www.fpdf.org/) for PHP).
//! The MIT notices of those works are reproduced in `THIRD_PARTY_LICENSES.md`.
//!
//! See [`Fpdf`] for the entry point.

mod document;
mod draw;
mod encoding;
mod error;
mod ext;
mod fmt;
mod fonts;
mod fonts_data;
mod image;
mod output;
pub mod report;
mod types;

pub use document::{FPDF_VERSION, Fpdf, PageBreakHook, PageHook};
pub use draw::{ImageOpts, RectStyle};
pub use error::FpdfError;
#[cfg(feature = "protection")]
pub use ext::Permissions;
pub use ext::{HAlign, QrEcc, TextBoxOpts, VAlign};
pub use fmt::round_to;
pub use types::{
    Align, Border, CellOpts, Color, DateTime, LayoutMode, Link, Ln, NamedColor, Orientation,
    PageFormat, PageSize, Rotation, Unit, ZoomMode,
};
