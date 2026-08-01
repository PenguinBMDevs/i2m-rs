use crate::error::{Error, Result};
use crate::image::RgbaImage;
use std::path::Path;

pub fn load_image(path: &Path) -> Result<RgbaImage> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "svg" {
        load_svg(path)
    } else {
        load_raster(path)
    }
}

fn load_raster(path: &Path) -> Result<RgbaImage> {
    let img = image::open(path).map_err(|e| Error::ImageDecode(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for chunk in rgba.chunks_exact(4) {
        pixels.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
    }

    Ok(RgbaImage {
        width,
        height,
        stride: width as usize * 4,
        pixels,
    })
}

fn load_svg(path: &Path) -> Result<RgbaImage> {
    let data = std::fs::read(path)?;
    let opt = resvg::usvg::Options::default();
    let tree =
        resvg::usvg::Tree::from_data(&data, &opt).map_err(|e| Error::ImageDecode(e.to_string()))?;

    let size = tree.size().to_int_size();
    let (width, height) = (size.width(), size.height());

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| Error::ImageDecode("failed to allocate SVG pixmap".into()))?;

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    let rgba = pixmap.data();
    let mut pixels = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks_exact(4) {
        pixels.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
    }

    Ok(RgbaImage {
        width,
        height,
        stride: width as usize * 4,
        pixels,
    })
}
