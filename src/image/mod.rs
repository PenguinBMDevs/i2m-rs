use crate::color::Color;

pub mod load;
pub mod transforms;

pub use load::load_image;

#[derive(Clone, Debug)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
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

    pub fn pixel_offset(&self, x: u32, y: u32) -> usize {
        y as usize * self.stride + x as usize * 4
    }

    pub fn get(&self, x: u32, y: u32) -> Color {
        let offset = self.pixel_offset(x, y);
        Color {
            b: self.pixels[offset],
            g: self.pixels[offset + 1],
            r: self.pixels[offset + 2],
            a: self.pixels[offset + 3],
        }
    }

    pub fn set(&mut self, x: u32, y: u32, color: Color) {
        let offset = self.pixel_offset(x, y);
        self.pixels[offset] = color.b;
        self.pixels[offset + 1] = color.g;
        self.pixels[offset + 2] = color.r;
        self.pixels[offset + 3] = color.a;
    }

    pub fn is_in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    pub fn iter_pixels(&self) -> impl Iterator<Item = (u32, u32, Color)> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| (x, y, self.get(x, y))))
    }
}
