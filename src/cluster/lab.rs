use crate::color::{Color, Lab, Palette};
use crate::image::RgbaImage;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::cluster::{color_from_floats, sample_colors};

const MAX_SAMPLES: usize = 20_000;
const THRESHOLD: f64 = 1.0;
const MAX_ITERATIONS: usize = 100;

/// K-Means clustering in CIE Lab space, returning the centers converted to RGB.
pub fn lab_kmeans(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let labs: Vec<Lab> = pixels
        .iter()
        .map(|c| crate::utils::rgb_to_lab(*c))
        .collect();
    let mut rng = StdRng::seed_from_u64(0);
    let mut centers = kmeans_plus_plus_lab(&labs, color_count, &mut rng);
    lloyd_lab(&labs, &mut centers, THRESHOLD, MAX_ITERATIONS);

    let colors: Vec<Color> = centers
        .iter()
        .map(|lab| lab_to_rgb(lab.l, lab.a, lab.b))
        .collect();
    Palette::new(colors)
}

/// K-Means++ initialization in Lab space.
fn kmeans_plus_plus_lab(labs: &[Lab], cluster_count: usize, rng: &mut StdRng) -> Vec<Lab> {
    let mut centers = Vec::with_capacity(cluster_count);
    if labs.is_empty() || cluster_count == 0 {
        return centers;
    }

    centers.push(labs[rng.gen_range(0..labs.len())]);
    let mut distances = vec![f64::MAX; labs.len()];
    update_lab_distances(labs, &centers[0], &mut distances);

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
        let new_center = labs[chosen];
        update_lab_distances(labs, &new_center, &mut distances);
        centers.push(new_center);
    }

    centers
}

fn update_lab_distances(labs: &[Lab], center: &Lab, distances: &mut [f64]) {
    for (index, lab) in labs.iter().enumerate() {
        let distance = lab_distance_sq(lab, center);
        if distance < distances[index] {
            distances[index] = distance;
        }
    }
}

fn lab_distance_sq(a: &Lab, b: &Lab) -> f64 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl * dl + da * da + db * db
}

/// Lloyd iterations in Lab space.
fn lloyd_lab(labs: &[Lab], centers: &mut [Lab], threshold: f64, max_iterations: usize) {
    let cluster_count = centers.len();
    if cluster_count == 0 || labs.is_empty() {
        return;
    }

    let mut sums = vec![[0.0; 3]; cluster_count];
    let mut counts = vec![0usize; cluster_count];

    for _ in 0..max_iterations {
        sums.fill([0.0; 3]);
        counts.fill(0);

        for lab in labs {
            let cluster = nearest_lab_center(lab, centers);
            sums[cluster][0] += lab.l;
            sums[cluster][1] += lab.a;
            sums[cluster][2] += lab.b;
            counts[cluster] += 1;
        }

        let mut max_change = 0.0;
        for cluster in 0..cluster_count {
            if counts[cluster] == 0 {
                relocate_empty_lab_cluster(labs, centers, cluster);
                continue;
            }
            let count = counts[cluster] as f64;
            let new_center = Lab {
                l: sums[cluster][0] / count,
                a: sums[cluster][1] / count,
                b: sums[cluster][2] / count,
            };
            let change = lab_distance_sq(&new_center, &centers[cluster]).sqrt();
            if change > max_change {
                max_change = change;
            }
            centers[cluster] = new_center;
        }

        if max_change < threshold {
            break;
        }
    }
}

fn nearest_lab_center(lab: &Lab, centers: &[Lab]) -> usize {
    let mut best = 0;
    let mut best_distance = f64::MAX;
    for (index, center) in centers.iter().enumerate() {
        let distance = lab_distance_sq(lab, center);
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
    }
    best
}

fn relocate_empty_lab_cluster(labs: &[Lab], centers: &mut [Lab], cluster: usize) {
    let mut best_index = 0;
    let mut best_distance = -1.0;
    for (index, lab) in labs.iter().enumerate() {
        let mut min_distance = f64::MAX;
        for (other, center) in centers.iter().enumerate() {
            if other == cluster {
                continue;
            }
            let distance = lab_distance_sq(lab, center);
            if distance < min_distance {
                min_distance = distance;
            }
        }
        if min_distance > best_distance {
            best_distance = min_distance;
            best_index = index;
        }
    }
    centers[cluster] = labs[best_index];
}

/// Convert a Lab color back to sRGB.
fn lab_to_rgb(l: f64, a: f64, b: f64) -> Color {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;

    let delta: f64 = 6.0 / 29.0;
    let f_inv = |t: f64| {
        if t > delta {
            t * t * t
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };

    let xr = f_inv(fx);
    let yr = if l > 903.3 * 0.008856 {
        ((l + 16.0) / 116.0).powi(3)
    } else {
        l / 903.3
    };
    let zr = f_inv(fz);

    let x = xr * 0.95047;
    let y = yr * 1.00000;
    let z = zr * 1.08883;

    let r = x * 3.2406 + y * -1.5372 + z * -0.4986;
    let g = x * -0.9689 + y * 1.8758 + z * 0.0415;
    let blue = x * 0.0557 + y * -0.2040 + z * 1.0570;

    let to_srgb = |channel: f64| {
        if channel > 0.0031308 {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * channel
        }
    };

    color_from_floats(
        to_srgb(r) * 255.0,
        to_srgb(g) * 255.0,
        to_srgb(blue) * 255.0,
    )
}
