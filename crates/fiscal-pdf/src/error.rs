//! Error type of the PDF engine.

use thiserror::Error;

/// Fatal error raised by the PDF engine.
///
/// FPDF reports misuse (no font selected, invalid link, unsupported image, ...) as fatal
/// errors. To keep the drawing API ergonomic, [`Fpdf`](crate::Fpdf) records the **first**
/// error and turns every later drawing call into a no-op; the error is returned by
/// [`Fpdf::output`](crate::Fpdf::output) (or inspected with
/// [`Fpdf::error`](crate::Fpdf::error)).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FpdfError {
    /// A usage or format error, with the FPDF message.
    #[error("FPDF error: {0}")]
    Message(String),
    /// An I/O failure while reading an image or writing the output file.
    #[error("FPDF error: {context}: {message}")]
    Io {
        /// What the engine was doing.
        context: String,
        /// The underlying I/O error message.
        message: String,
    },
}

impl FpdfError {
    pub(crate) fn msg(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }

    pub(crate) fn io(context: impl Into<String>, source: &std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            message: source.to_string(),
        }
    }
}
