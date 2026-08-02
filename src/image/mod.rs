//! In-memory RGBA image type and image loading.
//!
//! [`RgbaImage`] is the crate's own minimal image buffer (BGRA byte layout in
//! memory, RGBA semantics at the API level). [`load_image`] decodes PNG, JPEG,
//! BMP, GIF and WebP via the `image` crate, and rasterizes SVG via
//! `resvg`/`tiny-skia`. The [`transforms`] submodule offers rotation, flipping
//! and grayscale conversion.

use crate::color::Color;

pub mod load;
pub mod transforms;

pub use load::load_image;

/// An owned RGBA image buffer.
///
/// Pixels are stored row-major, 4 bytes per pixel. The on-disk byte order is
/// **B, G, R, A** (matching the original C# tool's buffer layout); all
/// accessors (`get`, `set`, `new`) convert to/from [`Color`] so you never
/// touch the raw order directly. `stride` is always `width * 4`.
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, RgbaImage};
///
/// let mut img = RgbaImage::new(2, 2, Color::BLACK);
/// img.set(1, 0, Color::new(255, 0, 0, 255));
/// assert_eq!(img.get(1, 0).r, 255);
/// assert!(img.is_in_bounds(1, 1));
/// assert!(!img.is_in_bounds(2, 0));
///
/// // Iterate every pixel with its coordinates:
/// for (x, y, color) in img.iter_pixels() {
///     let _ = (x, y, color);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct RgbaImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bytes per row (`width * 4`).
    pub stride: usize,
    /// Raw pixel data in B, G, R, A byte order, `height * stride` bytes long.
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// Create an image of `width` × `height` filled with `fill`.
    pub fn new(width: u32, height: u32, fill: Color) -> Self {
        let stride = width as usize * 4;
        let mut pixels = Vec::with_capacity(stride * height as usize);
        for _ in 0..width as usize * height as usize {
            pixels.extend_from_slice(&[fill.b, fill.g, fill.r, fill.a]);
        }
        Self {
            width,
            height,
            stride,
            pixels,
        }
    }

    /// Byte offset of pixel `(x, y)` inside [`pixels`](Self::pixels).
    pub fn pixel_offset(&self, x: u32, y: u32) -> usize {
        y as usize * self.stride + x as usize * 4
    }

    /// Read the color of pixel `(x, y)`.
    ///
    /// # Panics
    ///
    /// Panics on out-of-bounds coordinates (index out of range).
    pub fn get(&self, x: u32, y: u32) -> Color {
        let offset = self.pixel_offset(x, y);
        Color {
            b: self.pixels[offset],
            g: self.pixels[offset + 1],
            r: self.pixels[offset + 2],
            a: self.pixels[offset + 3],
        }
    }

    /// Overwrite pixel `(x, y)` with `color`.
    ///
    /// # Panics
    ///
    /// Panics on out-of-bounds coordinates.
    pub fn set(&mut self, x: u32, y: u32, color: Color) {
        let offset = self.pixel_offset(x, y);
        self.pixels[offset] = color.b;
        self.pixels[offset + 1] = color.g;
        self.pixels[offset + 2] = color.r;
        self.pixels[offset + 3] = color.a;
    }

    /// `true` if `(x, y)` lies inside the image.
    pub fn is_in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    /// Iterate all pixels as `(x, y, color)` triples, row by row.
    pub fn iter_pixels(&self) -> impl Iterator<Item = (u32, u32, Color)> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| (x, y, self.get(x, y))))
    }
}
