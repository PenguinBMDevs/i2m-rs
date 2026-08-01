pub mod batch;
pub mod cluster;
pub mod color;
pub mod config;
pub mod convert;
pub mod error;
pub mod image;
pub mod match_color;
pub mod midi;
pub mod progress;
pub mod resize;
pub mod utils;

pub use batch::{BatchItem, convert_batch};
pub use color::{Color, Palette, PaletteLabCache};
pub use config::{
    ColorIdMethod, ConverterConfig, KeyMode, NoteLengthMode, PaletteSource, ResizeAlgorithm,
};
pub use convert::{ConversionResult, convert};
pub use error::{Error, Result};
pub use image::{RgbaImage, load_image};
pub use midi::{
    TimedMidiEvent,
    events::MidiEvent,
    writer::{color_event_payload, write_midi},
};
pub use progress::{CancellationToken, Progress, Stage};
pub use resize::resize;
