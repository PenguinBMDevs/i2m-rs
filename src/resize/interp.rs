//! Kernel-based interpolation filters (bilinear, bicubic, Lanczos, Gaussian,
//! Mitchell, Hermite).
//!
//! All filters share the same skeleton: [`generic_sample`] maps each output
//! pixel to a fractional source coordinate and hands it to a per-filter
//! sampler that blends a fixed neighborhood. Out-of-range taps are clamped to
//! the image edge.

use crate::color::Color;
use crate::image::RgbaImage;

/// Bilinear interpolation over the nearest 2×2 pixels.
pub fn bilinear(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    generic_sample(image, new_width, new_height, sample_bilinear)
}

/// Cubic-convolution interpolation over the nearest 4×4 pixels.
pub fn bicubic(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    generic_sample(image, new_width, new_height, sample_bicubic)
}

/// Lanczos filter with window size a = 3 (6×6 taps).
pub fn lanczos(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    generic_sample(image, new_width, new_height, |img, x, y| {
        sample_lanczos(img, x, y, 3)
    })
}

/// Gaussian filter with σ = 1.0 and radius 2 (5×5 taps).
pub fn gaussian(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    generic_sample(image, new_width, new_height, |img, x, y| {
        sample_gaussian(img, x, y, 1.0, 2)
    })
}

/// Mitchell–Netravali filter with B = C = 1/3 (4×4 taps).
pub fn mitchell(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    generic_sample(image, new_width, new_height, sample_mitchell)
}

/// Hermite (smoothstep) interpolation over the nearest 2×2 pixels.
pub fn hermite(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    generic_sample(image, new_width, new_height, sample_hermite)
}

fn generic_sample<F>(image: &RgbaImage, new_width: u32, new_height: u32, sampler: F) -> RgbaImage
where
    F: Fn(&RgbaImage, f64, f64) -> Color,
{
    let scale_x = f64::from(image.width) / f64::from(new_width);
    let scale_y = f64::from(image.height) / f64::from(new_height);
    let mut out = RgbaImage::new(new_width, new_height, Color::BLACK);

    for y in 0..new_height {
        let sy = (f64::from(y) + 0.5) * scale_y - 0.5;
        for x in 0..new_width {
            let sx = (f64::from(x) + 0.5) * scale_x - 0.5;
            out.set(x, y, sampler(image, sx, sy));
        }
    }

    out
}

fn sample_bilinear(image: &RgbaImage, x: f64, y: f64) -> Color {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let c00 = get_clamped(image, x0, y0);
    let c10 = get_clamped(image, x0 + 1, y0);
    let c01 = get_clamped(image, x0, y0 + 1);
    let c11 = get_clamped(image, x0 + 1, y0 + 1);

    let w00 = (1.0 - fx) * (1.0 - fy);
    let w10 = fx * (1.0 - fy);
    let w01 = (1.0 - fx) * fy;
    let w11 = fx * fy;

    blend(&[(c00, w00), (c10, w10), (c01, w01), (c11, w11)])
}

fn sample_hermite(image: &RgbaImage, x: f64, y: f64) -> Color {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    let wx = hermite_weight(fx);
    let wy = hermite_weight(fy);

    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut a = 0.0;
    let mut total = 0.0;

    for dy in 0..2_i32 {
        for dx in 0..2_i32 {
            let weight = wx[dx as usize] * wy[dy as usize];
            let c = get_clamped(image, x0 + dx, y0 + dy);
            r += f64::from(c.r) * weight;
            g += f64::from(c.g) * weight;
            b += f64::from(c.b) * weight;
            a += f64::from(c.a) * weight;
            total += weight;
        }
    }

    if total > 0.0 {
        Color::new(
            (r / total).round() as u8,
            (g / total).round() as u8,
            (b / total).round() as u8,
            (a / total).round() as u8,
        )
    } else {
        Color::BLACK
    }
}

fn hermite_weight(t: f64) -> [f64; 2] {
    let h0 = t * t * (2.0 * t - 3.0) + 1.0;
    let h1 = t * t * (3.0 - 2.0 * t);
    [h0, h1]
}

fn sample_bicubic(image: &RgbaImage, x: f64, y: f64) -> Color {
    let x0 = x.floor() as i32 - 1;
    let y0 = y.floor() as i32 - 1;
    let fx = x - (x0 + 1) as f64;
    let fy = y - (y0 + 1) as f64;

    let cx = cubic_weights(fx);
    let cy = cubic_weights(fy);

    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut a = 0.0;
    let mut total = 0.0;

    for dy in 0..4_i32 {
        for dx in 0..4_i32 {
            let weight = cx[dx as usize] * cy[dy as usize];
            let c = get_clamped(image, x0 + dx, y0 + dy);
            r += f64::from(c.r) * weight;
            g += f64::from(c.g) * weight;
            b += f64::from(c.b) * weight;
            a += f64::from(c.a) * weight;
            total += weight;
        }
    }

    if total > 0.0 {
        Color::new(
            (r / total).round() as u8,
            (g / total).round() as u8,
            (b / total).round() as u8,
            (a / total).round() as u8,
        )
    } else {
        Color::BLACK
    }
}

fn cubic_weights(t: f64) -> [f64; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ]
}

fn sample_lanczos(image: &RgbaImage, x: f64, y: f64, a: i32) -> Color {
    let x0 = x.floor() as i32 - a + 1;
    let y0 = y.floor() as i32 - a + 1;
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let width = 2 * a;

    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut a_sum = 0.0;
    let mut total = 0.0;

    for dy in 0..width {
        let wy = lanczos_kernel(fy - f64::from(dy), a);
        for dx in 0..width {
            let wx = lanczos_kernel(fx - f64::from(dx), a);
            let weight = wx * wy;
            let c = get_clamped(image, x0 + dx, y0 + dy);
            r += f64::from(c.r) * weight;
            g += f64::from(c.g) * weight;
            b += f64::from(c.b) * weight;
            a_sum += f64::from(c.a) * weight;
            total += weight;
        }
    }

    if total > 0.0 {
        Color::new(
            (r / total).round() as u8,
            (g / total).round() as u8,
            (b / total).round() as u8,
            (a_sum / total).round() as u8,
        )
    } else {
        Color::BLACK
    }
}

fn lanczos_kernel(x: f64, a: i32) -> f64 {
    let ax = x.abs();
    if ax < 1e-10 {
        return 1.0;
    }
    if ax > f64::from(a) {
        return 0.0;
    }
    let pi_x = std::f64::consts::PI * x;
    let pi_x_a = pi_x / f64::from(a);
    (pi_x.sin() / pi_x) * (pi_x_a.sin() / pi_x_a)
}

fn sample_gaussian(image: &RgbaImage, x: f64, y: f64, sigma: f64, radius: i32) -> Color {
    let x0 = x.floor() as i32 - radius;
    let y0 = y.floor() as i32 - radius;
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut a_sum = 0.0;
    let mut total = 0.0;

    for dy in 0..(2 * radius + 1) {
        let wy = gaussian_kernel(fy - f64::from(dy), sigma);
        for dx in 0..(2 * radius + 1) {
            let wx = gaussian_kernel(fx - f64::from(dx), sigma);
            let weight = wx * wy;
            let c = get_clamped(image, x0 + dx, y0 + dy);
            r += f64::from(c.r) * weight;
            g += f64::from(c.g) * weight;
            b += f64::from(c.b) * weight;
            a_sum += f64::from(c.a) * weight;
            total += weight;
        }
    }

    if total > 0.0 {
        Color::new(
            (r / total).round() as u8,
            (g / total).round() as u8,
            (b / total).round() as u8,
            (a_sum / total).round() as u8,
        )
    } else {
        Color::BLACK
    }
}

fn gaussian_kernel(x: f64, sigma: f64) -> f64 {
    (-0.5 * (x / sigma).powi(2)).exp()
}

fn sample_mitchell(image: &RgbaImage, x: f64, y: f64) -> Color {
    let x0 = x.floor() as i32 - 1;
    let y0 = y.floor() as i32 - 1;
    let fx = x - (x0 + 1) as f64;
    let fy = y - (y0 + 1) as f64;

    let cx = mitchell_weights(fx);
    let cy = mitchell_weights(fy);

    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut a_sum = 0.0;
    let mut total = 0.0;

    for dy in 0..4_i32 {
        for dx in 0..4_i32 {
            let weight = cx[dx as usize] * cy[dy as usize];
            let c = get_clamped(image, x0 + dx, y0 + dy);
            r += f64::from(c.r) * weight;
            g += f64::from(c.g) * weight;
            b += f64::from(c.b) * weight;
            a_sum += f64::from(c.a) * weight;
            total += weight;
        }
    }

    if total > 0.0 {
        Color::new(
            (r / total).round() as u8,
            (g / total).round() as u8,
            (b / total).round() as u8,
            (a_sum / total).round() as u8,
        )
    } else {
        Color::BLACK
    }
}

fn mitchell_weights(t: f64) -> [f64; 4] {
    const B: f64 = 1.0 / 3.0;
    const C: f64 = 1.0 / 3.0;

    let kernel = |x: f64| {
        let ax = x.abs();
        if ax < 1.0 {
            ((12.0 - 9.0 * B - 6.0 * C) * ax.powi(3)
                + (-18.0 + 12.0 * B + 6.0 * C) * ax.powi(2)
                + (6.0 - 2.0 * B))
                / 6.0
        } else if ax < 2.0 {
            ((-B - 6.0 * C) * ax.powi(3)
                + (6.0 * B + 30.0 * C) * ax.powi(2)
                + (-12.0 * B - 48.0 * C) * ax
                + (8.0 * B + 24.0 * C))
                / 6.0
        } else {
            0.0
        }
    };

    [kernel(t + 1.0), kernel(t), kernel(t - 1.0), kernel(t - 2.0)]
}

fn get_clamped(image: &RgbaImage, x: i32, y: i32) -> Color {
    let x = x.clamp(0, i32::try_from(image.width).unwrap_or(i32::MAX) - 1) as u32;
    let y = y.clamp(0, i32::try_from(image.height).unwrap_or(i32::MAX) - 1) as u32;
    image.get(x, y)
}

fn blend(colors: &[(Color, f64)]) -> Color {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut a = 0.0;
    let mut total = 0.0;

    for (color, weight) in colors {
        r += f64::from(color.r) * weight;
        g += f64::from(color.g) * weight;
        b += f64::from(color.b) * weight;
        a += f64::from(color.a) * weight;
        total += weight;
    }

    if total > 0.0 {
        Color::new(
            (r / total).round() as u8,
            (g / total).round() as u8,
            (b / total).round() as u8,
            (a / total).round() as u8,
        )
    } else {
        Color::BLACK
    }
}
