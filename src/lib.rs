//! # i2m-rs
//!
//! `i2m-rs` is an image-to-MIDI converter library, ported from the C# project
//! *ImageToMidi_Modded*.
//!
//! The core idea: an image is treated like a piano roll. Each **column** of the
//! (resized) image maps to one MIDI key, each **palette color** maps to one MIDI
//! track, and every vertical run of pixels sharing the same color becomes a
//! note-on / note-off pair. The result is written as a standard `.mid` file
//! that "plays" the picture.
//!
//! ## Quick start
//!
//! ```no_run
//! use i2m_rs::{ConverterConfig, convert, load_image};
//! use i2m_rs::cluster::generate_palette;
//! use i2m_rs::midi::writer::write_midi;
//! use std::path::Path;
//! use std::sync::atomic::AtomicBool;
//!
//! // 1. Configure the conversion (defaults mimic the original C# tool).
//! let config = ConverterConfig::default();
//!
//! // 2. Load a raster (PNG/JPEG/BMP/GIF/WebP) or SVG image.
//! let image = load_image(Path::new("input.png"))?;
//!
//! // 3. Generate a palette from the image (or use `PaletteSource::Manual`).
//! let (palette, _dithered) =
//!     generate_palette(&image, &config.palette, config.color_count)?;
//!
//! // 4. Convert the image into timed MIDI events, one track per color.
//! let cancel = AtomicBool::new(false);
//! let result = convert(&image, &palette, &config, None, &cancel)?;
//! println!("generated {} notes", result.note_count);
//!
//! // 5. Write the standard MIDI file.
//! write_midi(Path::new("output.mid"), &[&result], &config)?;
//! # Ok::<(), i2m_rs::Error>(())
//! ```
//!
//! ## Conversion pipeline
//!
//! The full pipeline, and the module responsible for each step:
//!
//! 1. **Loading** — [`image::load_image`] decodes raster formats via the
//!    `image` crate, and rasterizes SVG via `resvg`/`tiny-skia`.
//! 2. **Palette generation** — [`cluster::generate_palette`] reduces the image
//!    to `color_count` colors using one of 20+ algorithms (K-Means, octree,
//!    median-cut-style splits, GMM, DBSCAN, dithering, …), selected with
//!    [`PaletteSource`].
//! 3. **Resizing** — [`resize::resize`] scales the image so its width equals
//!    the number of usable keys (`start_key..=end_key`), with 11 interpolation
//!    algorithms selectable via [`ResizeAlgorithm`].
//! 4. **Note generation** — [`convert`] scans the resized image bottom-up and
//!    emits [`TimedMidiEvent`]s per color track.
//! 5. **Writing** — [`midi::writer::write_midi`] assembles the tracks into a
//!    Format-1 SMF file, inserting tempo and optional color meta events.
//!
//! Steps 1–5 are wrapped for you by [`convert_batch`] when converting several
//! files with different configs.
//!
//! ## Progress reporting and cancellation
//!
//! Long conversions can report progress through the [`Progress`] trait (any
//! `Fn(Stage, f64) + Send + Sync` closure works out of the box) and can be
//! aborted by setting a shared [`AtomicBool`](std::sync::atomic::AtomicBool)
//! to `true`, which makes the conversion return [`Error::Cancelled`].
//!
//! ```
//! use i2m_rs::Progress;
//!
//! let progress = |stage: i2m_rs::Stage, fraction: f64| {
//!     eprintln!("{stage:?}: {:.0}%", fraction * 100.0);
//! };
//! let _: &dyn Progress = &progress; // pass as `Some(&progress)`
//! ```

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
