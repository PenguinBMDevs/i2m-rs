use crate::color::{Color, Palette};
use crate::error::Result;
use crate::image::RgbaImage;
use crate::match_color::find_nearest_rgb;
use crate::utils::clamp;

const BAYER_4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

const BAYER_SIZE: usize = 4;
const BAYER_MAX: f64 = 15.0;

/// Floyd–Steinberg dither an image into the given palette.
///
/// Returns a new image where each opaque pixel is replaced by the nearest
/// palette color, with quantization error distributed to neighbors.
pub fn floyd_steinberg(image: &RgbaImage, palette: &Palette, strength: f64) -> Result<RgbaImage> {
    let width = image.width as usize;
    let height = image.height as usize;
    let mut result = image.clone();
    if width == 0 || height == 0 {
        return Ok(result);
    }

    let mut error_curr = vec![0.0f32; width * 3];
    let mut error_next = vec![0.0f32; width * 3];
    let strength = strength as f32;

    for y in 0..height {
        let left_to_right = y % 2 == 0;
        let dx: isize = if left_to_right { 1 } else { -1 };
        let x_start: isize = if left_to_right { 0 } else { width as isize - 1 };
        let x_end: isize = if left_to_right { width as isize } else { -1 };

        let mut x = x_start;
        while x != x_end {
            let px = x as u32;
            let py = y as u32;
            let color = result.get(px, py);
            if color.a < 128 {
                x += dx;
                continue;
            }

            let base_index = x as usize * 3;
            let r = clamp_f32(f32::from(color.r) + error_curr[base_index]);
            let g = clamp_f32(f32::from(color.g) + error_curr[base_index + 1]);
            let b = clamp_f32(f32::from(color.b) + error_curr[base_index + 2]);

            let adjusted = Color::new(r as u8, g as u8, b as u8, 255);
            let nearest = palette.colors[find_nearest_rgb(adjusted, palette)];
            result.set(px, py, Color::new(nearest.r, nearest.g, nearest.b, color.a));

            let err_r = (r - f32::from(nearest.r)) * strength;
            let err_g = (g - f32::from(nearest.g)) * strength;
            let err_b = (b - f32::from(nearest.b)) * strength;

            let right = x + dx;
            let right_index = if right >= 0 && (right as usize) < width {
                Some(right as usize * 3)
            } else {
                None
            };

            if let Some(index) = right_index {
                error_curr[index] += err_r * 7.0 / 16.0;
                error_curr[index + 1] += err_g * 7.0 / 16.0;
                error_curr[index + 2] += err_b * 7.0 / 16.0;
            }

            if y + 1 < height {
                let down_index = x as usize * 3;
                error_next[down_index] += err_r * 5.0 / 16.0;
                error_next[down_index + 1] += err_g * 5.0 / 16.0;
                error_next[down_index + 2] += err_b * 5.0 / 16.0;

                let left = x - dx;
                if left >= 0 && (left as usize) < width {
                    let left_index = left as usize * 3;
                    error_next[left_index] += err_r * 3.0 / 16.0;
                    error_next[left_index + 1] += err_g * 3.0 / 16.0;
                    error_next[left_index + 2] += err_b * 3.0 / 16.0;
                }

                if let Some(index) = right_index {
                    error_next[index] += err_r * 1.0 / 16.0;
                    error_next[index + 1] += err_g * 1.0 / 16.0;
                    error_next[index + 2] += err_b * 1.0 / 16.0;
                }
            }

            x += dx;
        }

        std::mem::swap(&mut error_curr, &mut error_next);
        error_next.fill(0.0);
    }

    Ok(result)
}

/// Ordered Bayer dither an image into the given palette.
///
/// Uses a 4x4 Bayer matrix and adds threshold-scaled noise before matching
/// each pixel to its nearest palette entry.
pub fn ordered(image: &RgbaImage, palette: &Palette, strength: f64) -> Result<RgbaImage> {
    let width = image.width as usize;
    let height = image.height as usize;
    let mut result = image.clone();
    if width == 0 || height == 0 {
        return Ok(result);
    }

    for y in 0..height {
        for x in 0..width {
            let px = x as u32;
            let py = y as u32;
            let color = result.get(px, py);
            if color.a < 128 {
                continue;
            }

            let bayer_value = BAYER_4X4[y % BAYER_SIZE][x % BAYER_SIZE];
            let threshold = (f64::from(bayer_value) + 0.5) / (BAYER_MAX + 1.0);
            let offset = (threshold - 0.5) * 255.0 * strength;

            let r = clamp_f64(f64::from(color.r) + offset);
            let g = clamp_f64(f64::from(color.g) + offset);
            let b = clamp_f64(f64::from(color.b) + offset);

            let adjusted = Color::new(r as u8, g as u8, b as u8, 255);
            let nearest = palette.colors[find_nearest_rgb(adjusted, palette)];
            result.set(px, py, Color::new(nearest.r, nearest.g, nearest.b, color.a));
        }
    }

    Ok(result)
}

fn clamp_f32(value: f32) -> f32 {
    clamp(value, 0.0, 255.0)
}

fn clamp_f64(value: f64) -> f64 {
    clamp(value, 0.0, 255.0)
}
