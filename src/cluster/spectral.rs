#![allow(clippy::type_complexity, clippy::needless_range_loop)]

use crate::color::{Color, Lab, Palette};
use crate::image::RgbaImage;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::cluster::{color_from_floats, sample_colors};

const MAX_SAMPLES: usize = 2_000;
const KNN: usize = 10;
const SIGMA: f64 = 32.0;
const KMEANS_ITERATIONS: usize = 10;
const EIGEN_POWER_ITERATIONS: usize = 10;

/// Spectral clustering in Lab space with a KNN similarity graph.
#[allow(clippy::type_complexity, clippy::needless_range_loop)]
pub fn spectral(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let points: Vec<Lab> = pixels
        .iter()
        .map(|c| crate::utils::rgb_to_lab(*c))
        .collect();
    let n = points.len();
    if n <= color_count {
        let mut colors = pixels;
        while colors.len() < color_count {
            colors.push(colors.first().copied().unwrap_or(Color::BLACK));
        }
        colors.truncate(color_count);
        return Palette::new(colors);
    }

    let (edges, weights) = build_knn_graph(&points, KNN, SIGMA);
    let mut degrees = vec![0.0; n];
    for (i, j, w) in edges {
        degrees[i] += w;
        degrees[j] += w;
    }

    let eigen_count = (3usize).min(color_count);
    let mut eigenvectors = vec![vec![0.0; n]; eigen_count];
    let mut rng = StdRng::seed_from_u64(0);

    for k in 0..eigen_count {
        let mut vector: Vec<f64> = (0..n).map(|_| rng.gen_range(0.0..1.0)).collect();

        for prev in &eigenvectors[..k] {
            let dot = vector
                .iter()
                .zip(prev.iter())
                .map(|(a, b)| a * b)
                .sum::<f64>();
            for i in 0..n {
                vector[i] -= dot * prev[i];
            }
        }

        for _ in 0..EIGEN_POWER_ITERATIONS {
            let mut next = vec![0.0; n];
            for i in 0..n {
                let mut sum = degrees[i] * vector[i];
                for (j, w) in &weights[i] {
                    sum -= w * vector[*j];
                }
                next[i] = sum;
            }

            for prev in &eigenvectors[..k] {
                let dot = next
                    .iter()
                    .zip(prev.iter())
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                for i in 0..n {
                    next[i] -= dot * prev[i];
                }
            }

            let norm = next.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm < 1e-12 {
                break;
            }
            for i in 0..n {
                vector[i] = next[i] / norm;
            }
        }
        eigenvectors[k] = vector;
    }

    let features: Vec<Vec<f64>> = (0..n)
        .map(|i| eigenvectors.iter().map(|v| v[i]).collect())
        .collect();

    let labels = kmeans_plus_plus_in_embedding(&features, color_count, KMEANS_ITERATIONS, &mut rng);

    let mut cluster_sums = vec![[0.0; 4]; color_count];
    for (i, label) in labels.iter().enumerate() {
        let color = pixels[i];
        cluster_sums[*label][0] += f64::from(color.r);
        cluster_sums[*label][1] += f64::from(color.g);
        cluster_sums[*label][2] += f64::from(color.b);
        cluster_sums[*label][3] += 1.0;
    }

    let mut colors: Vec<Color> = cluster_sums
        .into_iter()
        .map(|sum| {
            let count = sum[3];
            if count > 0.0 {
                color_from_floats(sum[0] / count, sum[1] / count, sum[2] / count)
            } else {
                Color::BLACK
            }
        })
        .collect();
    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
    Palette::new(colors)
}

/// Build a KNN graph and return the edges plus adjacency lists with weights.
fn build_knn_graph(
    points: &[Lab],
    knn: usize,
    sigma: f64,
) -> (Vec<(usize, usize, f64)>, Vec<Vec<(usize, f64)>>) {
    let sigma_sq = sigma * sigma;
    let n = points.len();
    let mut edges = Vec::new();
    let mut weights: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];

    for i in 0..n {
        let mut distances: Vec<(f64, usize)> = Vec::with_capacity(n - 1);
        for j in 0..n {
            if i == j {
                continue;
            }
            let dist_sq = lab_distance_sq(&points[i], &points[j]);
            distances.push((dist_sq, j));
        }
        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (dist_sq, j) in distances.into_iter().take(knn) {
            let weight = (-dist_sq / (2.0 * sigma_sq)).exp();
            edges.push((i, j, weight));
            weights[i].push((j, weight));
            weights[j].push((i, weight));
        }
    }

    (edges, weights)
}

fn lab_distance_sq(a: &Lab, b: &Lab) -> f64 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl * dl + da * da + db * db
}

/// K-Means++ initialization followed by a few Lloyd iterations in the embedding.
fn kmeans_plus_plus_in_embedding(
    features: &[Vec<f64>],
    cluster_count: usize,
    iterations: usize,
    rng: &mut StdRng,
) -> Vec<usize> {
    let n = features.len();
    let dim = features[0].len();
    let mut centers: Vec<Vec<f64>> = Vec::with_capacity(cluster_count);
    centers.push(features[rng.gen_range(0..n)].clone());

    let mut distances = vec![f64::MAX; n];
    update_embedding_distances(features, &centers[0], &mut distances);

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
        let new_center = features[chosen].clone();
        update_embedding_distances(features, &new_center, &mut distances);
        centers.push(new_center);
    }

    let mut labels = vec![0usize; n];
    for _ in 0..iterations {
        for (index, feature) in features.iter().enumerate() {
            let mut best = 0;
            let mut best_distance = f64::MAX;
            for (cluster, center) in centers.iter().enumerate() {
                let mut distance = 0.0;
                for d in 0..dim {
                    let diff = feature[d] - center[d];
                    distance += diff * diff;
                }
                if distance < best_distance {
                    best_distance = distance;
                    best = cluster;
                }
            }
            labels[index] = best;
        }

        let mut new_centers = vec![vec![0.0; dim]; cluster_count];
        let mut counts = vec![0usize; cluster_count];
        for (index, label) in labels.iter().enumerate() {
            for d in 0..dim {
                new_centers[*label][d] += features[index][d];
            }
            counts[*label] += 1;
        }
        for cluster in 0..cluster_count {
            if counts[cluster] > 0 {
                for d in 0..dim {
                    new_centers[cluster][d] /= counts[cluster] as f64;
                }
                centers[cluster] = new_centers[cluster].clone();
            }
        }
    }

    labels
}

fn update_embedding_distances(features: &[Vec<f64>], center: &[f64], distances: &mut [f64]) {
    for (index, feature) in features.iter().enumerate() {
        let mut distance = 0.0;
        for d in 0..feature.len() {
            let diff = feature[d] - center[d];
            distance += diff * diff;
        }
        if distance < distances[index] {
            distances[index] = distance;
        }
    }
}
