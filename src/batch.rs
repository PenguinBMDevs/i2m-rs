//! Batch conversion of several images in one call.
//!
//! [`convert_batch`] runs the full pipeline (load → palette → convert) for a
//! list of [`BatchItem`]s, each with its own [`ConverterConfig`], and returns
//! one [`ConversionResult`] per input. The individual results can afterwards
//! be chained into a single `.mid` file by passing them all to
//! [`write_midi`](crate::midi::writer::write_midi), which appends each result
//! after the previous one (see its docs).
//!
//! # Examples
//!
//! ```no_run
//! use i2m_rs::{BatchItem, ConverterConfig, convert_batch};
//! use i2m_rs::midi::writer::write_midi;
//! use std::path::Path;
//! use std::sync::atomic::AtomicBool;
//!
//! let config_a = ConverterConfig::default();
//! let config_b = ConverterConfig { color_count: 4, ..Default::default() };
//!
//! let items = [
//!     BatchItem { input_path: Path::new("verse.png"), config: &config_a },
//!     BatchItem { input_path: Path::new("chorus.png"), config: &config_b },
//! ];
//!
//! let cancel = AtomicBool::new(false);
//! let results = convert_batch(&items, None, &cancel)?;
//!
//! // Chain both parts into one song:
//! let refs: Vec<_> = results.iter().collect();
//! write_midi(Path::new("song.mid"), &refs, &config_a)?;
//! # Ok::<(), i2m_rs::Error>(())
//! ```

use crate::color::Palette;
use crate::config::ConverterConfig;
use crate::convert::{ConversionResult, convert};
use crate::error::Result;
use crate::image::load_image;
use crate::progress::{Progress, Stage};
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// One image to convert, paired with the config to use for it.
///
/// Borrows both the path and the config, so a batch can mix freely shared
/// configs without cloning.
pub struct BatchItem<'a> {
    /// Path to the input image (raster formats or SVG).
    pub input_path: &'a Path,
    /// Conversion settings for this particular image.
    pub config: &'a ConverterConfig,
}

/// Convert every [`BatchItem`] in order.
///
/// For each item this will:
///
/// 1. load the image with [`load_image`](crate::image::load_image);
/// 2. build the palette — [`PaletteSource::Manual`](crate::config::PaletteSource::Manual)
///    is used as-is, everything else runs [`crate::cluster::generate_palette`]
///    (any dithered image produced by dithering methods is discarded here);
/// 3. run [`convert`].
///
/// Progress: after each finished item, [`Stage::WritingMidi`] is reported with
/// `fraction = (index + 1) / total`.
///
/// # Errors
///
/// * [`Error::Cancelled`](crate::Error::Cancelled) — `cancel` was set before
///   starting an item.
/// * Any error from loading, palette generation or conversion aborts the whole
///   batch; already finished results are dropped.
pub fn convert_batch(
    items: &[BatchItem<'_>],
    progress: Option<&dyn Progress>,
    cancel: &AtomicBool,
) -> Result<Vec<ConversionResult>> {
    let total = items.len();
    let mut results = Vec::with_capacity(total);

    for (index, item) in items.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(crate::error::Error::Cancelled);
        }

        let image = load_image(item.input_path)?;
        let palette = match &item.config.palette {
            crate::config::PaletteSource::Manual(colors) => Palette::new(colors.clone()),
            _ => {
                let (palette, _dithered) = crate::cluster::generate_palette(
                    &image,
                    &item.config.palette,
                    item.config.color_count,
                )?;
                palette
            }
        };

        let result = convert(&image, &palette, item.config, progress, cancel)?;
        results.push(result);

        let fraction = (index + 1) as f64 / total as f64;
        if let Some(p) = progress {
            p.report(Stage::WritingMidi, fraction);
        }
    }

    Ok(results)
}
