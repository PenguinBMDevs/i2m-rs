use crate::color::{Color, Palette};
use crate::image::RgbaImage;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::cluster::{color_from_floats, sample_colors};

const GMM_MAX_SAMPLES: usize = 2_000;
const GMM_MAX_ITER: usize = 30;
const GMM_TOL: f64 = 1.0;

/// Gaussian Mixture Model with diagonal covariance and K-Means++ initialization.
#[allow(clippy::needless_range_loop)]
pub fn gmm(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, GMM_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }
    let points: Vec<[f64; 3]> = pixels
        .iter()
        .map(|color| [f64::from(color.r), f64::from(color.g), f64::from(color.b)])
        .collect();

    let mut rng = StdRng::seed_from_u64(0);
    let mut means = kmeans_plus_plus_init(&points, color_count, &mut rng);
    let mut variances = vec![[400.0f64; 3]; color_count];
    let mut weights = vec![1.0 / color_count as f64; color_count];

    let mut responsibilities = vec![vec![0.0; color_count]; points.len()];
    let mut prev_log_likelihood = f64::MIN;

    for _ in 0..GMM_MAX_ITER {
        // E-step: compute cluster responsibilities for each point.
        for (point_index, point) in points.iter().enumerate() {
            let mut total = 0.0;
            for cluster in 0..color_count {
                let prob =
                    weights[cluster] * gaussian_diag(point, &means[cluster], &variances[cluster]);
                responsibilities[point_index][cluster] = prob;
                total += prob;
            }
            let total = total.max(1e-20);
            for cluster in 0..color_count {
                responsibilities[point_index][cluster] /= total;
            }
        }

        // M-step: update means, variances and weights.
        for cluster in 0..color_count {
            let mut sum_resp = 0.0;
            let mut mean = [0.0; 3];
            for (point_index, point) in points.iter().enumerate() {
                let resp = responsibilities[point_index][cluster];
                sum_resp += resp;
                for channel in 0..3 {
                    mean[channel] += point[channel] * resp;
                }
            }
            let sum_resp = sum_resp.max(1e-8);
            for channel in 0..3 {
                mean[channel] /= sum_resp;
            }

            let mut variance = [0.0; 3];
            for (point_index, point) in points.iter().enumerate() {
                let resp = responsibilities[point_index][cluster];
                for channel in 0..3 {
                    let diff = point[channel] - mean[channel];
                    variance[channel] += resp * diff * diff;
                }
            }
            for channel in 0..3 {
                variances[cluster][channel] = (variance[channel] / sum_resp).max(16.0);
            }

            means[cluster] = mean;
            weights[cluster] = sum_resp / points.len() as f64;
        }

        // Check log-likelihood convergence.
        let mut log_likelihood = 0.0;
        for point in &points {
            let mut sum = 0.0;
            for cluster in 0..color_count {
                sum +=
                    weights[cluster] * gaussian_diag(point, &means[cluster], &variances[cluster]);
            }
            log_likelihood += sum.max(1e-20).ln();
        }
        if (log_likelihood - prev_log_likelihood).abs() < GMM_TOL {
            break;
        }
        prev_log_likelihood = log_likelihood;
    }

    let mut colors: Vec<Color> = means
        .iter()
        .map(|mean| color_from_floats(mean[0], mean[1], mean[2]))
        .collect();
    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
    Palette::new(colors)
}

fn kmeans_plus_plus_init(
    points: &[[f64; 3]],
    cluster_count: usize,
    rng: &mut StdRng,
) -> Vec<[f64; 3]> {
    let mut centers = Vec::with_capacity(cluster_count);
    if points.is_empty() || cluster_count == 0 {
        return centers;
    }
    centers.push(points[rng.gen_range(0..points.len())]);

    let mut distances = vec![f64::MAX; points.len()];
    update_distances(points, &centers[0], &mut distances);

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
        let new_center = points[chosen];
        update_distances(points, &new_center, &mut distances);
        centers.push(new_center);
    }
    centers
}

fn update_distances(points: &[[f64; 3]], center: &[f64; 3], distances: &mut [f64]) {
    for (index, point) in points.iter().enumerate() {
        let distance = squared_distance(point, center);
        if distance < distances[index] {
            distances[index] = distance;
        }
    }
}

fn squared_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn gaussian_diag(x: &[f64; 3], mean: &[f64; 3], variance: &[f64; 3]) -> f64 {
    let det = variance[0] * variance[1] * variance[2];
    let norm = 1.0 / ((2.0 * std::f64::consts::PI).powf(1.5) * det.sqrt());
    let mut sum = 0.0;
    for channel in 0..3 {
        let diff = x[channel] - mean[channel];
        sum += diff * diff / variance[channel];
    }
    norm * (-0.5 * sum).exp()
}
