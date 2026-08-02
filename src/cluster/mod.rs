//! Palette generation: reduce an image to `color_count` representative colors.
//!
//! [`generate_palette`] dispatches on [`PaletteSource`](crate::config::PaletteSource)
//! to one of the algorithm submodules:
//!
//! | Submodule | Algorithms | Idea |
//! |-----------|------------|------|
//! | [`kmeans`] | K-Means, K-Means++, native K-Means, Max-Min | iterative center refinement in RGB |
//! | [`lab`] | Lab K-Means | K-Means in perceptual Lab space |
//! | [`spatial`] | Popularity, Octree, VarianceSplit | histogram / tree / box-split methods |
//! | [`density`] | MeanShift, DBSCAN, OPTICS | density-based clustering |
//! | [`gmm`] | GMM | EM with diagonal covariance |
//! | [`hierarchy`] | Hierarchical | agglomerative single-linkage |
//! | [`spectral`] | Spectral | KNN graph + eigen-embedding |
//! | [`dither`] | Floyd–Steinberg, ordered Bayer | quantize *and* dither the image |
//! | [`fixed`] | Fixed-bit palettes | classic 2/4/16-color or bit-allocated palettes |
//!
//! All algorithms operate on a deterministic sample of at most ~20 000 opaque
//! pixels ([`sample_colors`]), so very large images stay fast and results are
//! reproducible. Returned palettes are sorted by hue
//! ([`sort_palette_by_hsl`]) and padded with black / truncated to exactly
//! `color_count` entries ([`pad_or_truncate_palette`]).

use crate::color::{Color, Palette};
use crate::config::PaletteSource;
use crate::error::{Error, Result};
use crate::image::RgbaImage;
use crate::utils::clamp;

pub mod density;
pub mod dither;
pub mod fixed;
pub mod gmm;
pub mod hierarchy;
pub mod kmeans;
pub mod lab;
pub mod spatial;
pub mod spectral;

use density::{dbscan, mean_shift, optics};
use dither::{floyd_steinberg, ordered};
use fixed::fixed_bit_palette;
use gmm::gmm;
use hierarchy::hierarchical;
use kmeans::{kmeans, kmeans_plus_plus, max_min, native_kmeans};
use lab::lab_kmeans;
use spatial::{octree, popularity, variance_split};
use spectral::spectral;

/// Maximum number of opaque pixels used by most clustering algorithms.
const DEFAULT_MAX_SAMPLES: usize = 20_000;

/// Generate a palette from an image using the requested [`PaletteSource`].
///
/// Returns the palette and, for the two dithering methods
/// ([`PaletteSource::FloydSteinbergDither`], [`PaletteSource::OrderedDither`]),
/// additionally the dithered image (`Ok((palette, Some(dithered)))`); all
/// other methods return `None` as the second element.
///
/// The palette is sorted by hue and adjusted to exactly `color_count` entries.
/// Note that [`PaletteSource::Manual`] simply echoes the given colors back
/// (then still sorted/padded); skip this function entirely and use the manual
/// palette directly if that is not desired.
///
/// # Errors
///
/// Returns [`Error::PaletteGeneration`] if `color_count` is zero.
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, RgbaImage, PaletteSource};
/// use i2m_rs::cluster::generate_palette;
///
/// // Two half-images: red left, blue right.
/// let mut img = RgbaImage::new(8, 8, Color::new(255, 0, 0, 255));
/// for y in 0..8 {
///     for x in 4..8 {
///         img.set(x, y, Color::new(0, 0, 255, 255));
///     }
/// }
///
/// let (palette, dithered) = generate_palette(&img, &PaletteSource::Popularity, 2).unwrap();
/// assert!(dithered.is_none());
/// assert_eq!(palette.colors.len(), 2); // red and blue
/// ```
pub fn generate_palette(
    image: &RgbaImage,
    source: &PaletteSource,
    color_count: usize,
) -> Result<(Palette, Option<RgbaImage>)> {
    if color_count == 0 {
        return Err(Error::PaletteGeneration("color_count must be > 0".into()));
    }

    let mut colors = match source {
        PaletteSource::Manual(colors) => colors.clone(),
        PaletteSource::OnlyWpfMedianCut => {
            // Fallback to KMeans++ since WPF median cut is not available.
            kmeans_plus_plus(image, color_count).colors
        }
        PaletteSource::OnlyKMeansPlusPlus => kmeans_plus_plus(image, color_count).colors,
        PaletteSource::KMeans => kmeans(image, color_count, false).colors,
        PaletteSource::KMeansPlusPlus => kmeans(image, color_count, true).colors,
        PaletteSource::Popularity => popularity(image, color_count).colors,
        PaletteSource::Octree => octree(image, color_count).colors,
        PaletteSource::VarianceSplit => variance_split(image, color_count).colors,
        PaletteSource::Pca => pca(image, color_count).colors,
        PaletteSource::MaxMin => max_min(image, color_count).colors,
        PaletteSource::NativeKMeans => native_kmeans(image, color_count).colors,
        PaletteSource::MeanShift => mean_shift(image, color_count).colors,
        PaletteSource::Dbscan => dbscan(image, color_count).colors,
        PaletteSource::Optics => optics(image, color_count).colors,
        PaletteSource::Gmm => gmm(image, color_count).colors,
        PaletteSource::Hierarchical => hierarchical(image, color_count).colors,
        PaletteSource::Spectral => spectral(image, color_count).colors,
        PaletteSource::LabKMeans => lab_kmeans(image, color_count).colors,
        PaletteSource::FloydSteinbergDither => {
            let base = kmeans_plus_plus(image, color_count);
            let dithered = floyd_steinberg(image, &base, 1.0)?;
            let mut colors = base.colors;
            sort_palette_by_hsl(&mut colors);
            pad_or_truncate_palette(&mut colors, color_count);
            return Ok((Palette::new(colors), Some(dithered)));
        }
        PaletteSource::OrderedDither => {
            let base = kmeans_plus_plus(image, color_count);
            let dithered = ordered(image, &base, 1.0)?;
            let mut colors = base.colors;
            sort_palette_by_hsl(&mut colors);
            pad_or_truncate_palette(&mut colors, color_count);
            return Ok((Palette::new(colors), Some(dithered)));
        }
        PaletteSource::FixedBitPalette => fixed_bit_palette(color_count, false).colors,
    };

    sort_palette_by_hsl(&mut colors);
    pad_or_truncate_palette(&mut colors, color_count);
    Ok((Palette::new(colors), None))
}

/// Sort palette colors by a HSL-derived key (hue, then saturation, then lightness).
///
/// Gives every generated palette a stable, pleasing track order independent of
/// the clustering algorithm's output order.
pub fn sort_palette_by_hsl(colors: &mut [Color]) {
    colors.sort_by(|a, b| {
        let key_a = crate::utils::rgb_to_hsl_key(*a);
        let key_b = crate::utils::rgb_to_hsl_key(*b);
        key_a
            .partial_cmp(&key_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Pad a palette with black or truncate it to exactly `color_count` colors.
///
/// Clustering algorithms may return fewer colors than requested (e.g. an image
/// with fewer distinct colors); the converter expects the palette size to
/// match the track count, so it is normalized here.
pub fn pad_or_truncate_palette(colors: &mut Vec<Color>, color_count: usize) {
    colors.truncate(color_count);
    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
}

/// Collect all opaque pixels from an image.
pub(crate) fn collect_opaque(image: &RgbaImage) -> Vec<Color> {
    image
        .iter_pixels()
        .filter(|(_, _, color)| color.a >= 128)
        .map(|(_, _, color)| color)
        .collect()
}

/// Sample up to `max_samples` opaque pixels deterministically.
pub(crate) fn sample_colors(image: &RgbaImage, max_samples: usize) -> Vec<Color> {
    let pixels = collect_opaque(image);
    crate::utils::sample_pixels(&pixels, max_samples)
}

/// Convert float RGB components (already in [0, 255]) to a `Color`.
pub(crate) fn color_from_floats(r: f64, g: f64, b: f64) -> Color {
    Color::new(
        clamp(r.round() as i32, 0, 255) as u8,
        clamp(g.round() as i32, 0, 255) as u8,
        clamp(b.round() as i32, 0, 255) as u8,
        255,
    )
}

/// Compute a PCA palette by sampling the principal component of RGB covariance.
fn pca(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, DEFAULT_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }
    if color_count == 1 || pixels.len() <= color_count {
        let mut colors: Vec<Color> = pixels;
        while colors.len() < color_count {
            colors.push(Color::BLACK);
        }
        colors.truncate(color_count);
        return Palette::new(colors);
    }

    let mean = mean_rgb(&pixels);
    let cov = covariance_rgb(&pixels, &mean);
    let eigenvector = power_iteration(&cov, 20);

    let mut projections: Vec<(f64, Color)> = pixels
        .iter()
        .map(|color| {
            let centered = center_rgb(color, &mean);
            let projection = centered[0] * eigenvector[0]
                + centered[1] * eigenvector[1]
                + centered[2] * eigenvector[2];
            (projection, *color)
        })
        .collect();
    projections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut palette = Vec::with_capacity(color_count);
    let max_index = projections.len() - 1;
    for index in 0..color_count {
        let t = index as f64 / (color_count - 1) as f64;
        let position = (t * max_index as f64).round() as usize;
        palette.push(projections[position.min(max_index)].1);
    }
    Palette::new(palette)
}

/// Compute the mean RGB vector of a set of colors.
fn mean_rgb(pixels: &[Color]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for color in pixels {
        sum[0] += f64::from(color.r);
        sum[1] += f64::from(color.g);
        sum[2] += f64::from(color.b);
    }
    let count = pixels.len() as f64;
    [sum[0] / count, sum[1] / count, sum[2] / count]
}

/// Subtract the mean from a color, returning a centered RGB vector.
fn center_rgb(color: &Color, mean: &[f64; 3]) -> [f64; 3] {
    [
        f64::from(color.r) - mean[0],
        f64::from(color.g) - mean[1],
        f64::from(color.b) - mean[2],
    ]
}

/// Compute the RGB covariance matrix of a set of pixels.
fn covariance_rgb(pixels: &[Color], mean: &[f64; 3]) -> [[f64; 3]; 3] {
    let mut cov = [[0.0; 3]; 3];
    for color in pixels {
        let centered = center_rgb(color, mean);
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += centered[i] * centered[j];
            }
        }
    }
    let count = pixels.len() as f64;
    for row in &mut cov {
        for value in row {
            *value /= count;
        }
    }
    cov
}

/// Approximate the dominant eigenvector of a symmetric 3x3 matrix using power iteration.
fn power_iteration(matrix: &[[f64; 3]; 3], iterations: usize) -> [f64; 3] {
    let mut vector = [1.0; 3];
    for _ in 0..iterations {
        let mut next = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                next[i] += matrix[i][j] * vector[j];
            }
        }
        let norm = (next[0] * next[0] + next[1] * next[1] + next[2] * next[2]).sqrt();
        if norm < 1e-12 {
            break;
        }
        vector = [next[0] / norm, next[1] / norm, next[2] / norm];
    }
    vector
}
