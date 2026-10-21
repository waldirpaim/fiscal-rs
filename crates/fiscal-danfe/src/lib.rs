//! DANFE (printed representation of the NF-e) rendered to PDF from the authorised XML.
//!
//! This crate is a Rust translation of the DANFE layout of the ACBr project and is
//! therefore distributed under the **LGPL-2.1-or-later** (see `NOTICE.md`). The PDF is
//! produced with [`fiscal_pdf`].

pub mod context;
pub mod conversions;
pub mod decimal;
pub mod error;
pub mod format;
pub mod model;
mod nfce;
mod nfe;
pub mod options;
mod reader;
pub mod xml;

pub use context::DanfeContext;
pub use decimal::Decimal;
pub use error::DanfeError;
pub use model::Nfe;
pub use nfce::render_nfce;
pub use nfe::{render_danfe, render_danfe_bytes, render_nfe};
pub use options::DanfeOptions;
