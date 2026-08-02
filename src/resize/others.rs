//! Non-kernel resampling filters: area-weighted averaging, nearest neighbor,
//! box average, integral-image average, and mode pooling.

use crate::color::Color;
use crate::image::RgbaImage;
use std::collections::HashMap;

/// Area resampling: each output pixel is the average of the source pixels it
/// covers, weighted by the covered fraction of each source pixel.
///
/// This is the default algorithm — it preserves overall brightness best when
/// downscaling by large factors.
pub fn area_resampling(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (src_w, src_h) = (image.width as f64, image.height as f64);
    let (dst_w, dst_h) = (new_width as f64, new_height as f64);
    let scale_x = src_w / dst_w;
    let scale_y = src_h / dst_h;

    let mut out = RgbaImage::new(new_width, new_height, Color::BLACK);

    for y in 0..new_height {
        let y0 = y as f64 * scale_y;
        let y1 = (y + 1) as f64 * scale_y;
        let y_start = y0.floor() as u32;
        let y_end = y1.ceil().min(src_h) as u32;

        for x in 0..new_width {
            let x0 = x as f64 * scale_x;
            let x1 = (x + 1) as f64 * scale_x;
            let x_start = x0.floor() as u32;
            let x_end = x1.ceil().min(src_w) as u32;

            let (mut r, mut g, mut b, mut a) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
            let mut total = 0.0_f64;

            for sy in y_start..y_end {
                let y_overlap = overlap(y0, y1, f64::from(sy), f64::from(sy + 1));
                for sx in x_start..x_end {
                    let x_overlap = overlap(x0, x1, f64::from(sx), f64::from(sx + 1));
                    let weight = x_overlap * y_overlap;
                    let c = image.get(sx, sy);
                    r += f64::from(c.r) * weight;
                    g += f64::from(c.g) * weight;
                    b += f64::from(c.b) * weight;
                    a += f64::from(c.a) * weight;
                    total += weight;
                }
            }

            if total > 0.0 {
                let color = Color::new(
                    (r / total).round() as u8,
                    (g / total).round() as u8,
                    (b / total).round() as u8,
                    (a / total).round() as u8,
                );
                out.set(x, y, color);
            }
        }
    }

    out
}

fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

/// Nearest-neighbor sampling: pick the source pixel closest to the mapped
/// coordinate. Fastest; preserves hard edges and exact palette colors.
pub fn nearest_neighbor(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (src_w, src_h) = (image.width, image.height);
    let scale_x = f64::from(src_w) / f64::from(new_width);
    let scale_y = f64::from(src_h) / f64::from(new_height);
    let mut out = RgbaImage::new(new_width, new_height, Color::BLACK);

    for y in 0..new_height {
        for x in 0..new_width {
            let sx = ((f64::from(x) + 0.5) * scale_x - 0.5)
                .round()
                .clamp(0.0, f64::from(src_w - 1)) as u32;
            let sy = ((f64::from(y) + 0.5) * scale_y - 0.5)
                .round()
                .clamp(0.0, f64::from(src_h - 1)) as u32;
            out.set(x, y, image.get(sx, sy));
        }
    }

    out
}

/// Box filter: unweighted average of all source pixels inside the covered
/// rectangle.
pub fn box_filter(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (src_w, src_h) = (image.width as f64, image.height as f64);
    let (dst_w, dst_h) = (new_width as f64, new_height as f64);
    let scale_x = src_w / dst_w;
    let scale_y = src_h / dst_h;

    let mut out = RgbaImage::new(new_width, new_height, Color::BLACK);

    for y in 0..new_height {
        let y0 = (y as f64 * scale_y).floor() as u32;
        let y1 = (((y + 1) as f64 * scale_y).ceil() as u32).min(image.height);

        for x in 0..new_width {
            let x0 = (x as f64 * scale_x).floor() as u32;
            let x1 = (((x + 1) as f64 * scale_x).ceil() as u32).min(image.width);

            let (mut r, mut g, mut b, mut a) = (0_u64, 0_u64, 0_u64, 0_u64);
            let mut count = 0_u64;

            for sy in y0..y1 {
                for sx in x0..x1 {
                    let c = image.get(sx, sy);
                    r += u64::from(c.r);
                    g += u64::from(c.g);
                    b += u64::from(c.b);
                    a += u64::from(c.a);
                    count += 1;
                }
            }

            let color = Color::new(
                r.checked_div(count).unwrap_or(0) as u8,
                g.checked_div(count).unwrap_or(0) as u8,
                b.checked_div(count).unwrap_or(0) as u8,
                a.checked_div(count).unwrap_or(0) as u8,
            );
            out.set(x, y, color);
        }
    }

    out
}

/// Box average computed from a summed-area (integral) table — O(1) per output
/// pixel regardless of the covered rectangle size.
pub fn integral_image(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (src_w, src_h) = (image.width, image.height);
    let scale_x = f64::from(src_w) / f64::from(new_width);
    let scale_y = f64::from(src_h) / f64::from(new_height);

    let mut sum_r = vec![vec![0_u64; src_w as usize + 1]; src_h as usize + 1];
    let mut sum_g = vec![vec![0_u64; src_w as usize + 1]; src_h as usize + 1];
    let mut sum_b = vec![vec![0_u64; src_w as usize + 1]; src_h as usize + 1];
    let mut sum_a = vec![vec![0_u64; src_w as usize + 1]; src_h as usize + 1];

    for y in 0..src_h {
        for x in 0..src_w {
            let c = image.get(x, y);
            let yy = y as usize + 1;
            let xx = x as usize + 1;
            sum_r[yy][xx] =
                sum_r[yy - 1][xx] + sum_r[yy][xx - 1] - sum_r[yy - 1][xx - 1] + u64::from(c.r);
            sum_g[yy][xx] =
                sum_g[yy - 1][xx] + sum_g[yy][xx - 1] - sum_g[yy - 1][xx - 1] + u64::from(c.g);
            sum_b[yy][xx] =
                sum_b[yy - 1][xx] + sum_b[yy][xx - 1] - sum_b[yy - 1][xx - 1] + u64::from(c.b);
            sum_a[yy][xx] =
                sum_a[yy - 1][xx] + sum_a[yy][xx - 1] - sum_a[yy - 1][xx - 1] + u64::from(c.a);
        }
    }

    let mut out = RgbaImage::new(new_width, new_height, Color::BLACK);

    for y in 0..new_height {
        let y0 = (y as f64 * scale_y).floor() as u32;
        let y1 = (((y + 1) as f64 * scale_y).ceil() as u32).min(src_h);

        for x in 0..new_width {
            let x0 = (x as f64 * scale_x).floor() as u32;
            let x1 = (((x + 1) as f64 * scale_x).ceil() as u32).min(src_w);

            let count = u64::from((y1 - y0) * (x1 - x0));

            let rect = |sum: &[Vec<u64>]| {
                sum[y1 as usize][x1 as usize] + sum[y0 as usize][x0 as usize]
                    - sum[y1 as usize][x0 as usize]
                    - sum[y0 as usize][x1 as usize]
            };

            let sum_r_value = rect(&sum_r);
            let sum_g_value = rect(&sum_g);
            let sum_b_value = rect(&sum_b);
            let sum_a_value = rect(&sum_a);

            let color = Color::new(
                sum_r_value.checked_div(count).unwrap_or(0) as u8,
                sum_g_value.checked_div(count).unwrap_or(0) as u8,
                sum_b_value.checked_div(count).unwrap_or(0) as u8,
                sum_a_value.checked_div(count).unwrap_or(0) as u8,
            );
            out.set(x, y, color);
        }
    }

    out
}

/// Mode pooling: the most frequent exact RGBA value inside the covered
/// rectangle wins. Keeps results on the original color grid (no new blended
/// colors), which is useful for already-quantized images.
pub fn mode_pooling(image: &RgbaImage, new_width: u32, new_height: u32) -> RgbaImage {
    let (src_w, src_h) = (image.width as f64, image.height as f64);
    let (dst_w, dst_h) = (new_width as f64, new_height as f64);
    let scale_x = src_w / dst_w;
    let scale_y = src_h / dst_h;

    let mut out = RgbaImage::new(new_width, new_height, Color::BLACK);

    for y in 0..new_height {
        let y0 = (y as f64 * scale_y).floor() as u32;
        let y1 = (((y + 1) as f64 * scale_y).ceil() as u32).min(image.height);

        for x in 0..new_width {
            let x0 = (x as f64 * scale_x).floor() as u32;
            let x1 = (((x + 1) as f64 * scale_x).ceil() as u32).min(image.width);

            let mut counts: HashMap<(u8, u8, u8, u8), usize> = HashMap::new();
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let c = image.get(sx, sy);
                    *counts.entry((c.r, c.g, c.b, c.a)).or_insert(0) += 1;
                }
            }

            if let Some((best, _)) = counts.into_iter().max_by_key(|(_, count)| *count) {
                out.set(x, y, Color::new(best.0, best.1, best.2, best.3));
            }
        }
    }

    out
}
