//! Nearest-palette-color matching.
//!
//! After a palette has been generated, every opaque pixel of the resized image
//! must be mapped onto one palette index (the *color ID*). [`match_pixel`]
//! dispatches on [`ColorIdMethod`] to one of five distance metrics; the
//! `find_nearest_*` functions are also public for direct use.
//!
//! Transparent pixels (`a < 128`) and empty palettes yield
//! [`TRANSPARENT_ID`], which the converter turns into silence.

use crate::color::{Color, Palette, PaletteLabCache};
use crate::config::ColorIdMethod;
use crate::utils::{ciede2000, rgb_to_hsl, rgb_to_hsv};

/// Color ID meaning "no color": returned for transparent pixels and empty
/// palettes. Never a valid palette index (all valid IDs are `>= 0`).
pub const TRANSPARENT_ID: i32 = -2;

/// Match one pixel to its nearest palette entry using the chosen metric.
///
/// Returns the palette index (`0..palette.colors.len()`), or
/// [`TRANSPARENT_ID`] when the pixel is transparent (`a < 128`) or the palette
/// is empty.
///
/// `cache` must be `Some` (built from the *same* palette) when `method` is
/// [`ColorIdMethod::Lab`] or [`ColorIdMethod::Ciede2000`]; it is ignored for
/// the other methods and may be `None`.
///
/// # Panics
///
/// Panics if a Lab-based method is selected but `cache` is `None`.
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, ColorIdMethod, Palette};
/// use i2m_rs::match_color::{match_pixel, TRANSPARENT_ID};
///
/// let palette = Palette::new(vec![Color::BLACK, Color::new(255, 255, 255, 255)]);
///
/// // Dark gray is nearer to black (index 0):
/// assert_eq!(match_pixel(Color::new(40, 40, 40, 255), &palette, ColorIdMethod::Rgb, None), 0);
/// // Transparent pixels never match:
/// assert_eq!(match_pixel(Color::new(255, 0, 0, 0), &palette, ColorIdMethod::Rgb, None), TRANSPARENT_ID);
/// ```
pub fn match_pixel(
    color: Color,
    palette: &Palette,
    method: ColorIdMethod,
    cache: Option<&PaletteLabCache>,
) -> i32 {
    if palette.colors.is_empty() {
        return TRANSPARENT_ID;
    }
    if color.a < 128 {
        return TRANSPARENT_ID;
    }

    match method {
        ColorIdMethod::Rgb => find_nearest_rgb(color, palette) as i32,
        ColorIdMethod::Hsv => find_nearest_hsv(color, palette) as i32,
        ColorIdMethod::Hsl => find_nearest_hsl(color, palette) as i32,
        ColorIdMethod::Lab | ColorIdMethod::Ciede2000 => {
            let cache = cache.expect("Lab/CIEDE2000 color matching requires a PaletteLabCache");
            if method == ColorIdMethod::Lab {
                find_nearest_lab(color, palette, cache) as i32
            } else {
                find_nearest_ciede2000(color, palette, cache) as i32
            }
        }
    }
}

/// Index of the palette entry with the smallest squared Euclidean RGB
/// distance to `color`. The fastest metric; ties resolve to the lower index.
pub fn find_nearest_rgb(color: Color, palette: &Palette) -> usize {
    let (r, g, b) = (i32::from(color.r), i32::from(color.g), i32::from(color.b));
    let mut best = 0;
    let mut best_dist = i32::MAX;

    for (index, candidate) in palette.colors.iter().enumerate() {
        let dr = r - i32::from(candidate.r);
        let dg = g - i32::from(candidate.g);
        let db = b - i32::from(candidate.b);
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best = index;
        }
    }

    best
}

/// Index of the palette entry nearest in HSV space.
///
/// Low-value or low-saturation pixels are matched mostly by value/saturation;
/// otherwise hue distance (circular) is weighted highest.
pub fn find_nearest_hsv(color: Color, palette: &Palette) -> usize {
    let (h1, s1, v1) = rgb_to_hsv(color);
    let mut best = 0;
    let mut best_score = f64::MAX;

    for (index, candidate) in palette.colors.iter().enumerate() {
        let (h2, s2, v2) = rgb_to_hsv(*candidate);
        let score = hsv_score(h1, s1, v1, h2, s2, v2);
        if score < best_score {
            best_score = score;
            best = index;
        }
    }

    best
}

/// Index of the palette entry nearest in HSL space.
///
/// Dark or desaturated pixels are matched by lightness alone; otherwise hue
/// distance (circular) is weighted highest.
pub fn find_nearest_hsl(color: Color, palette: &Palette) -> usize {
    let (h1, s1, l1) = rgb_to_hsl(color);
    let mut best = 0;
    let mut best_score = f64::MAX;

    for (index, candidate) in palette.colors.iter().enumerate() {
        let (h2, s2, l2) = rgb_to_hsl(*candidate);
        let score = hsl_score(h1, s1, l1, h2, s2, l2);
        if score < best_score {
            best_score = score;
            best = index;
        }
    }

    best
}

/// Index of the palette entry nearest in Lab space, using the pre-computed
/// `cache` for the palette side.
pub fn find_nearest_lab(color: Color, _palette: &Palette, cache: &PaletteLabCache) -> usize {
    let lab = crate::utils::rgb_to_lab(color);
    let mut best = 0;
    let mut best_dist = f64::MAX;

    for (index, candidate) in cache.lab.iter().enumerate() {
        let dl = lab.l - candidate.l;
        let da = lab.a - candidate.a;
        let db = lab.b - candidate.b;
        let dist = dl * dl + da * da + db * db;
        if dist < best_dist {
            best_dist = dist;
            best = index;
        }
    }

    best
}

/// Index of the palette entry with the smallest CIEDE2000 ΔE, using the
/// pre-computed `cache` for the palette side. Slowest but most perceptual.
pub fn find_nearest_ciede2000(color: Color, _palette: &Palette, cache: &PaletteLabCache) -> usize {
    let lab = crate::utils::rgb_to_lab(color);
    let mut best = 0;
    let mut best_dist = f64::MAX;

    for (index, candidate) in cache.lab.iter().enumerate() {
        let dist = ciede2000(&lab, candidate);
        if dist < best_dist {
            best_dist = dist;
            best = index;
        }
    }

    best
}

fn hue_distance(h1: f64, h2: f64) -> f64 {
    let diff = (h1 - h2).abs();
    if diff > 180.0 { 360.0 - diff } else { diff }
}

fn hsv_score(h1: f64, s1: f64, v1: f64, h2: f64, s2: f64, v2: f64) -> f64 {
    if v1 < 0.22 || s1 < 0.22 {
        (v1 - v2).abs() * 2.0 + (s1 - s2).abs()
    } else {
        let dh = hue_distance(h1, h2) / 180.0;
        let ds = (s1 - s2).abs();
        let dv = (v1 - v2).abs();
        dh * 2.0 + ds * 0.5 + dv * 0.5
    }
}

fn hsl_score(h1: f64, s1: f64, l1: f64, h2: f64, s2: f64, l2: f64) -> f64 {
    if l1 < 0.22 || s1 < 0.22 {
        (l1 - l2).abs() * 10.0
    } else {
        let dh = hue_distance(h1, h2) / 180.0;
        let ds = (s1 - s2).abs();
        let dl = (l1 - l2).abs();
        dh * 2.0 + ds * 0.5 + dl * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_pixel_is_ignored() {
        let palette = Palette::new(vec![Color::new(255, 0, 0, 255), Color::new(0, 255, 0, 255)]);
        let id = match_pixel(Color::new(255, 0, 0, 0), &palette, ColorIdMethod::Rgb, None);
        assert_eq!(id, TRANSPARENT_ID);
    }

    #[test]
    fn nearest_rgb_selects_correct_index() {
        let palette = Palette::new(vec![Color::new(255, 0, 0, 255), Color::new(0, 255, 0, 255)]);
        let id = match_pixel(
            Color::new(200, 0, 0, 255),
            &palette,
            ColorIdMethod::Rgb,
            None,
        );
        assert_eq!(id, 0);
    }
}
