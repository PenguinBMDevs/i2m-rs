//! Error type shared by the whole crate.
//!
//! Every fallible public API returns [`Result`]. The variants map 1:1 onto
//! pipeline stages, so you can match on the stage that failed.
//!
//! # Examples
//!
//! ```
//! use i2m_rs::{Error, load_image};
//! use std::path::Path;
//!
//! match load_image(Path::new("missing.png")) {
//!     Err(Error::Io(e)) => eprintln!("cannot read file: {e}"),
//!     Err(Error::ImageDecode(e)) => eprintln!("corrupt image: {e}"),
//!     other => { let _ = other; }
//! }
//! ```

use thiserror::Error;

/// All errors that can occur while converting images to MIDI.
#[derive(Debug, Error)]
pub enum Error {
    /// A filesystem operation failed (e.g. image or MIDI file could not be
    /// opened/created). Wraps [`std::io::Error`] via `#[from]`.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The input file could not be decoded as an image (unsupported codec or
    /// corrupt data for raster files, parse failure for SVG).
    #[error("image decode error: {0}")]
    ImageDecode(String),

    /// A [`crate::config::ConverterConfig`] or palette is invalid (empty
    /// palette, `start_key > end_key`, unusable key range, …).
    #[error("invalid configuration: {0}")]
    Config(String),

    /// Palette generation failed (e.g. `color_count == 0`).
    #[error("palette generation failed: {0}")]
    PaletteGeneration(String),

    /// The input/output format is not supported.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// An external helper command failed. Reserved for future integrations.
    #[error("external command failed: {0}")]
    ExternalCommand(String),

    /// Resizing failed (zero target dimensions, zero-sized result).
    #[error("resize error: {0}")]
    Resize(String),

    /// MIDI serialization or writing failed (also: nothing to write).
    #[error("MIDI error: {0}")]
    Midi(String),

    /// The operation was aborted via the shared cancellation flag
    /// (`AtomicBool`) or [`crate::progress::CancellationToken`].
    #[error("operation cancelled")]
    Cancelled,
}

/// Convenience alias for `std::result::Result<T, i2m_rs::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
