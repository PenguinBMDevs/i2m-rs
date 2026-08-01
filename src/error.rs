use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image decode error: {0}")]
    ImageDecode(String),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("palette generation failed: {0}")]
    PaletteGeneration(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("external command failed: {0}")]
    ExternalCommand(String),

    #[error("resize error: {0}")]
    Resize(String),

    #[error("MIDI error: {0}")]
    Midi(String),

    #[error("operation cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;
