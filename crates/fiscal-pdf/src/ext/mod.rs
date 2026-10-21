//! Extensions of the base engine: barcodes, QR codes, rotation, rounded rectangles, dashed
//! lines, optional content layers, text boxes with word wrap, a tiny HTML writer and
//! (behind the `protection` feature) RC4 encryption.
//!
//! These correspond to the FPDF scripts bundled in FPDF-Pascal's `ACBr_fpdf_ext` unit.

mod barcode;
mod barcode_tables;
mod html;
mod layers;
mod shapes;
mod textbox;

#[cfg(feature = "protection")]
pub(crate) mod protection;

pub use barcode::QrEcc;
#[cfg(feature = "protection")]
pub use protection::Permissions;
pub use textbox::{HAlign, TextBoxOpts, VAlign};

pub(crate) use layers::Layer;
