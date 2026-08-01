use crate::color::{Color, Palette};
use crate::image::RgbaImage;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::cluster::{color_from_floats, sample_colors};

const KMEANS_MAX_SAMPLES: usize = 20_000;
const KMEANS_THRESHOLD: f64 = 1.0;
const KMEANS_MAX_ITERATIONS: usize = 100;
const NATIVE_KMEANS_ITERATIONS: usize = 10;
const NATIVE_KMEANS_RATE: f64 = 0.3;
const MAX_MIN_ITERATIONS: usize = 3;

/// Run Lloyd-style K-Means in RGB space.
///
/// If `plus_plus` is true, centers are initialized with K-Means++.
/// Otherwise the first `color_count` sampled pixels are used as seeds.
pub fn kmeans(image: &RgbaImage, color_count: usize, plus_plus: bool) -> Palette {
    let pixels = sample_colors(image, KMEANS_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let mut rng = StdRng::seed_from_u64(0);
    let mut centers = if plus_plus {
        kmeans_plus_plus_init(&pixels, color_count, &mut rng)
    } else {
        initial_centers(&pixels, color_count, &mut rng)
    };

    lloyd(
        &pixels,
        &mut centers,
        KMEANS_THRESHOLD,
        KMEANS_MAX_ITERATIONS,
    );
    Palette::new(
        centers
            .iter()
            .map(|c| color_from_floats(c[0], c[1], c[2]))
            .collect(),
    )
}

/// Return the K-Means++ initialized centers without further refinement.
pub fn kmeans_plus_plus(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, KMEANS_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }
    let mut rng = StdRng::seed_from_u64(0);
    let centers = kmeans_plus_plus_init(&pixels, color_count, &mut rng);
    Palette::new(
        centers
            .iter()
            .map(|c| color_from_floats(c[0], c[1], c[2]))
            .collect(),
    )
}

/// Native-style K-Means that updates centers incrementally with a fixed rate.
pub fn native_kmeans(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, KMEANS_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let mut rng = StdRng::seed_from_u64(0);
    let mut positions = kmeans_plus_plus_init(&pixels, color_count, &mut rng);

    let mut counts = vec![0usize; color_count];
    let mut sums = vec![[0.0; 3]; color_count];

    for _ in 0..NATIVE_KMEANS_ITERATIONS {
        sums.fill([0.0; 3]);
        counts.fill(0);

        for color in &pixels {
            let point = [f64::from(color.r), f64::from(color.g), f64::from(color.b)];
            let cluster = nearest_cluster(&point, &positions);
            counts[cluster] += 1;
            let weight = counts[cluster] as f64;
            sums[cluster][0] = (sums[cluster][0] * (weight - 1.0) + point[0]) / weight;
            sums[cluster][1] = (sums[cluster][1] * (weight - 1.0) + point[1]) / weight;
            sums[cluster][2] = (sums[cluster][2] * (weight - 1.0) + point[2]) / weight;
            positions[cluster][0] = positions[cluster][0] * (1.0 - NATIVE_KMEANS_RATE)
                + sums[cluster][0] * NATIVE_KMEANS_RATE;
            positions[cluster][1] = positions[cluster][1] * (1.0 - NATIVE_KMEANS_RATE)
                + sums[cluster][1] * NATIVE_KMEANS_RATE;
            positions[cluster][2] = positions[cluster][2] * (1.0 - NATIVE_KMEANS_RATE)
                + sums[cluster][2] * NATIVE_KMEANS_RATE;
        }
    }

    Palette::new(
        positions
            .iter()
            .map(|c| color_from_floats(c[0], c[1], c[2]))
            .collect(),
    )
}

/// Weighted Max-Min K-Means: spread centers by frequency-weighted distance,
/// then refine with a few K-Means iterations.
pub fn max_min(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, KMEANS_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }
    if color_count == 1 {
        return Palette::new(vec![mean_color(&pixels)]);
    }

    let mut rng = StdRng::seed_from_u64(0);
    let mut centers = max_min_init(&pixels, color_count, &mut rng);
    lloyd(&pixels, &mut centers, KMEANS_THRESHOLD, MAX_MIN_ITERATIONS);
    Palette::new(
        centers
            .iter()
            .map(|c| color_from_floats(c[0], c[1], c[2]))
            .collect(),
    )
}

/// K-Means++ initialization seeded by a `StdRng`.
fn kmeans_plus_plus_init(
    pixels: &[Color],
    cluster_count: usize,
    rng: &mut StdRng,
) -> Vec<[f64; 3]> {
    let mut centers = Vec::with_capacity(cluster_count);
    if pixels.is_empty() || cluster_count == 0 {
        return centers;
    }

    let first = pixels[rng.gen_range(0..pixels.len())];
    centers.push([f64::from(first.r), f64::from(first.g), f64::from(first.b)]);

    let mut distances = vec![f64::MAX; pixels.len()];
    update_distances(pixels, &centers[0], &mut distances);

    while centers.len() < cluster_count {
        let total: f64 = distances.iter().sum();
        let target = if total > 0.0 {
            rng.gen_range(0.0..1.0) * total
        } else {
            0.0
        };
        let mut accumulator = 0.0;
        let mut chosen = 0;
        for (index, distance) in distances.iter().enumerate() {
            accumulator += distance;
            if accumulator >= target {
                chosen = index;
                break;
            }
        }

        let point = pixels[chosen];
        let new_center = [f64::from(point.r), f64::from(point.g), f64::from(point.b)];
        update_distances(pixels, &new_center, &mut distances);
        centers.push(new_center);
    }

    centers
}

/// Choose `cluster_count` initial centers deterministically from the first pixels.
fn initial_centers(pixels: &[Color], cluster_count: usize, rng: &mut StdRng) -> Vec<[f64; 3]> {
    let mut shuffled: Vec<Color> = pixels.to_vec();
    shuffled.shuffle(rng);
    shuffled
        .into_iter()
        .take(cluster_count)
        .map(|color| [f64::from(color.r), f64::from(color.g), f64::from(color.b)])
        .collect()
}

/// Update the minimum squared distance from each pixel to a newly added center.
fn update_distances(pixels: &[Color], center: &[f64; 3], distances: &mut [f64]) {
    for (index, color) in pixels.iter().enumerate() {
        let point = [f64::from(color.r), f64::from(color.g), f64::from(color.b)];
        let distance = squared_distance(&point, center);
        if distance < distances[index] {
            distances[index] = distance;
        }
    }
}

/// Run Lloyd iterations until convergence or the iteration limit.
fn lloyd(pixels: &[Color], centers: &mut [[f64; 3]], threshold: f64, max_iterations: usize) {
    let cluster_count = centers.len();
    if cluster_count == 0 || pixels.is_empty() {
        return;
    }

    let mut sums = vec![[0.0; 3]; cluster_count];
    let mut counts = vec![0usize; cluster_count];

    for _ in 0..max_iterations {
        sums.fill([0.0; 3]);
        counts.fill(0);

        for color in pixels {
            let point = [f64::from(color.r), f64::from(color.g), f64::from(color.b)];
            let cluster = nearest_cluster(&point, centers);
            sums[cluster][0] += point[0];
            sums[cluster][1] += point[1];
            sums[cluster][2] += point[2];
            counts[cluster] += 1;
        }

        let mut max_change = 0.0;
        for cluster in 0..cluster_count {
            if counts[cluster] == 0 {
                relocate_empty_cluster(pixels, centers, cluster);
                continue;
            }
            let count = counts[cluster] as f64;
            let new_center = [
                sums[cluster][0] / count,
                sums[cluster][1] / count,
                sums[cluster][2] / count,
            ];
            for channel in 0..3 {
                let change = (new_center[channel] - centers[cluster][channel]).abs();
                if change > max_change {
                    max_change = change;
                }
            }
            centers[cluster] = new_center;
        }

        if max_change < threshold {
            break;
        }
    }
}

/// Find the index of the nearest center to a point.
fn nearest_cluster(point: &[f64; 3], centers: &[[f64; 3]]) -> usize {
    let mut best = 0;
    let mut best_distance = f64::MAX;
    for (index, center) in centers.iter().enumerate() {
        let distance = squared_distance(point, center);
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
    }
    best
}

/// Squared Euclidean distance between two 3D points.
fn squared_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// Move an empty cluster center to the pixel farthest from other centers.
fn relocate_empty_cluster(pixels: &[Color], centers: &mut [[f64; 3]], cluster: usize) {
    let mut best_index = 0;
    let mut best_distance = -1.0;
    for (index, color) in pixels.iter().enumerate() {
        let point = [f64::from(color.r), f64::from(color.g), f64::from(color.b)];
        let mut min_distance = f64::MAX;
        for (other, center) in centers.iter().enumerate() {
            if other == cluster {
                continue;
            }
            let distance = squared_distance(&point, center);
            if distance < min_distance {
                min_distance = distance;
            }
        }
        if min_distance > best_distance {
            best_distance = min_distance;
            best_index = index;
        }
    }
    let chosen = pixels[best_index];
    centers[cluster] = [
        f64::from(chosen.r),
        f64::from(chosen.g),
        f64::from(chosen.b),
    ];
}

/// Weighted Max-Min initialization favoring high-frequency colors.
fn max_min_init(pixels: &[Color], cluster_count: usize, _rng: &mut StdRng) -> Vec<[f64; 3]> {
    use std::collections::HashMap;

    let mut frequencies: HashMap<Color, usize> = HashMap::new();
    for color in pixels {
        *frequencies.entry(*color).or_insert(0) += 1;
    }

    let first = *frequencies
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .unwrap_or(&pixels[0]);

    let mut centers = vec![[f64::from(first.r), f64::from(first.g), f64::from(first.b)]];
    let mut distances = vec![f64::MAX; pixels.len()];
    update_distances(pixels, &centers[0], &mut distances);

    while centers.len() < cluster_count {
        let mut best_distance = -1.0;
        let mut candidates = Vec::new();
        for (index, distance) in distances.iter().enumerate() {
            if (*distance - best_distance).abs() < 1e-3 {
                candidates.push(index);
            } else if *distance > best_distance {
                best_distance = *distance;
                candidates.clear();
                candidates.push(index);
            }
        }

        let best_index = *candidates
            .iter()
            .max_by_key(|idx| frequencies.get(&pixels[**idx]).unwrap_or(&0))
            .unwrap_or(&candidates[0]);

        let point = pixels[best_index];
        let new_center = [f64::from(point.r), f64::from(point.g), f64::from(point.b)];
        update_distances(pixels, &new_center, &mut distances);
        centers.push(new_center);
    }

    centers
}

/// Compute the mean color of a set of pixels.
fn mean_color(pixels: &[Color]) -> Color {
    let mut sum = [0.0; 3];
    for color in pixels {
        sum[0] += f64::from(color.r);
        sum[1] += f64::from(color.g);
        sum[2] += f64::from(color.b);
    }
    let count = pixels.len() as f64;
    color_from_floats(sum[0] / count, sum[1] / count, sum[2] / count)
}
