//! Errors of the DANFE renderer.

use thiserror::Error;

/// Errors reading an NF-e or rendering its DANFE.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DanfeError {
    /// The XML is malformed.
    #[error("invalid XML: {0}")]
    Xml(String),
    /// The XML is well formed but is not an NF-e (or misses a mandatory block).
    #[error("invalid NF-e: {0}")]
    InvalidNfe(String),
    /// The PDF engine failed.
    #[error(transparent)]
    Pdf(#[from] fiscal_pdf::FpdfError),
}
