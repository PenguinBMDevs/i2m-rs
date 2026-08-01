use crate::color::Palette;
use crate::config::ConverterConfig;
use crate::convert::{ConversionResult, convert};
use crate::error::Result;
use crate::image::load_image;
use crate::progress::{Progress, Stage};
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub struct BatchItem<'a> {
    pub input_path: &'a Path,
    pub config: &'a ConverterConfig,
}

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
