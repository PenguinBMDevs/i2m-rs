use crate::color::{Color, Palette};
use crate::image::RgbaImage;

use crate::cluster::{color_from_floats, sample_colors};

const MEAN_SHIFT_MAX_SAMPLES: usize = 10_000;
const MEAN_SHIFT_BANDWIDTH: f64 = 32.0;
const MEAN_SHIFT_MAX_ITER: usize = 7;

const DBSCAN_MAX_SAMPLES: usize = 2_000;
const DBSCAN_MIN_PTS: usize = 4;

/// Mean-shift clustering on RGB points with a Gaussian kernel.
pub fn mean_shift(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, MEAN_SHIFT_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let points: Vec<[f64; 3]> = pixels
        .iter()
        .map(|color| [f64::from(color.r), f64::from(color.g), f64::from(color.b)])
        .collect();
    let mut shifted = points.clone();

    let grid_size = (MEAN_SHIFT_BANDWIDTH / 2.0).max(1.0) as usize;
    let bandwidth_sq = MEAN_SHIFT_BANDWIDTH * MEAN_SHIFT_BANDWIDTH;

    for _ in 0..MEAN_SHIFT_MAX_ITER {
        let grid = build_grid(&points, grid_size);
        for point in &mut shifted {
            let key = grid_key(point, grid_size);
            let mut sum = [0.0; 3];
            let mut weight_sum = 0.0;
            for dx in -1..=1i32 {
                for dy in -1..=1i32 {
                    for dz in -1..=1i32 {
                        let neighbor_key = (key.0 + dx, key.1 + dy, key.2 + dz);
                        if let Some(indices) = grid.get(&neighbor_key) {
                            for &index in indices {
                                let other = points[index];
                                let dist_sq = squared_distance(point, &other);
                                if dist_sq <= bandwidth_sq {
                                    let weight = (-dist_sq / (2.0 * bandwidth_sq)).exp();
                                    for channel in 0..3 {
                                        sum[channel] += other[channel] * weight;
                                    }
                                    weight_sum += weight;
                                }
                            }
                        }
                    }
                }
            }
            if weight_sum > 0.0 {
                for channel in 0..3 {
                    point[channel] = sum[channel] / weight_sum;
                }
            }
        }
    }

    merge_centers(&points, &shifted, MEAN_SHIFT_BANDWIDTH / 2.0, color_count)
}

/// Build a grid acceleration structure mapping voxel keys to point indices.
fn build_grid(
    points: &[[f64; 3]],
    grid_size: usize,
) -> std::collections::HashMap<(i32, i32, i32), Vec<usize>> {
    let mut grid: std::collections::HashMap<(i32, i32, i32), Vec<usize>> =
        std::collections::HashMap::new();
    for (index, point) in points.iter().enumerate() {
        grid.entry(grid_key(point, grid_size))
            .or_default()
            .push(index);
    }
    grid
}

fn grid_key(point: &[f64; 3], grid_size: usize) -> (i32, i32, i32) {
    let size = grid_size as f64;
    (
        (point[0] / size).floor() as i32,
        (point[1] / size).floor() as i32,
        (point[2] / size).floor() as i32,
    )
}

fn squared_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// Merge shifted points into clusters and return the dominant `color_count` centers.
fn merge_centers(
    points: &[[f64; 3]],
    shifted: &[[f64; 3]],
    merge_radius: f64,
    color_count: usize,
) -> Palette {
    let merge_radius_sq = merge_radius * merge_radius;
    let mut cluster_sums: Vec<[f64; 4]> = Vec::new();

    for (index, point) in shifted.iter().enumerate() {
        let mut found = None;
        for (cluster_index, sum) in cluster_sums.iter().enumerate() {
            let center = [sum[0] / sum[3], sum[1] / sum[3], sum[2] / sum[3]];
            if squared_distance(point, &center) < merge_radius_sq {
                found = Some(cluster_index);
                break;
            }
        }
        if let Some(cluster_index) = found {
            let original = points[index];
            for channel in 0..3 {
                cluster_sums[cluster_index][channel] += original[channel];
            }
            cluster_sums[cluster_index][3] += 1.0;
        } else {
            let original = points[index];
            cluster_sums.push([original[0], original[1], original[2], 1.0]);
        }
    }

    let mut clusters: Vec<(Color, f64)> = cluster_sums
        .into_iter()
        .map(|sum| {
            let count = sum[3];
            (
                color_from_floats(sum[0] / count, sum[1] / count, sum[2] / count),
                count,
            )
        })
        .collect();
    clusters.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut colors: Vec<Color> = clusters
        .into_iter()
        .take(color_count)
        .map(|(color, _)| color)
        .collect();
    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
    Palette::new(colors)
}

/// Density-based spatial clustering with grid acceleration.
pub fn dbscan(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, DBSCAN_MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let points: Vec<[f64; 3]> = pixels
        .iter()
        .map(|color| [f64::from(color.r), f64::from(color.g), f64::from(color.b)])
        .collect();
    let epsilon = estimate_epsilon(&points, DBSCAN_MIN_PTS);
    let epsilon_sq = epsilon * epsilon;
    let grid_size = (epsilon / 2.0).max(1.0) as usize;
    let grid = build_grid(&points, grid_size);

    let mut labels = vec![-1i32; points.len()];
    let mut cluster_id = 0;

    for index in 0..points.len() {
        if labels[index] != -1 {
            continue;
        }
        let neighbors = range_query(&points, index, &grid, grid_size, epsilon_sq);
        if neighbors.len() < DBSCAN_MIN_PTS {
            labels[index] = -2;
            continue;
        }
        expand_cluster(
            &points,
            &grid,
            grid_size,
            epsilon_sq,
            &mut labels,
            index,
            cluster_id,
            &neighbors,
        );
        cluster_id += 1;
    }

    let mut cluster_sums: std::collections::HashMap<i32, [f64; 4]> =
        std::collections::HashMap::new();
    for (index, label) in labels.iter().enumerate() {
        if *label < 0 {
            continue;
        }
        let entry = cluster_sums.entry(*label).or_insert([0.0; 4]);
        for channel in 0..3 {
            entry[channel] += points[index][channel];
        }
        entry[3] += 1.0;
    }

    let mut clusters: Vec<(Color, f64)> = cluster_sums
        .into_values()
        .map(|sum| {
            let count = sum[3];
            (
                color_from_floats(sum[0] / count, sum[1] / count, sum[2] / count),
                count,
            )
        })
        .collect();
    clusters.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut colors: Vec<Color> = clusters
        .into_iter()
        .take(color_count)
        .map(|(color, _)| color)
        .collect();
    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
    Palette::new(colors)
}

fn estimate_epsilon(points: &[[f64; 3]], min_pts: usize) -> f64 {
    let mut distances: Vec<f64> = Vec::with_capacity(points.len());
    for (index, point) in points.iter().enumerate() {
        let mut neighbors: Vec<f64> = Vec::with_capacity(points.len() - 1);
        for (other_index, other) in points.iter().enumerate() {
            if index == other_index {
                continue;
            }
            neighbors.push(squared_distance(point, other).sqrt());
        }
        neighbors.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let position = (min_pts - 1).min(neighbors.len().saturating_sub(1));
        distances.push(neighbors[position]);
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    distances[distances.len() / 2]
}

fn range_query(
    points: &[[f64; 3]],
    index: usize,
    grid: &std::collections::HashMap<(i32, i32, i32), Vec<usize>>,
    grid_size: usize,
    epsilon_sq: f64,
) -> Vec<usize> {
    let point = &points[index];
    let key = grid_key(point, grid_size);
    let mut neighbors = Vec::new();
    for dx in -1..=1i32 {
        for dy in -1..=1i32 {
            for dz in -1..=1i32 {
                let neighbor_key = (key.0 + dx, key.1 + dy, key.2 + dz);
                if let Some(indices) = grid.get(&neighbor_key) {
                    for &other_index in indices {
                        if other_index == index {
                            continue;
                        }
                        if squared_distance(point, &points[other_index]) <= epsilon_sq {
                            neighbors.push(other_index);
                        }
                    }
                }
            }
        }
    }
    neighbors
}

#[allow(clippy::too_many_arguments)]
fn expand_cluster(
    points: &[[f64; 3]],
    grid: &std::collections::HashMap<(i32, i32, i32), Vec<usize>>,
    grid_size: usize,
    epsilon_sq: f64,
    labels: &mut [i32],
    core_index: usize,
    cluster_id: i32,
    seeds: &[usize],
) {
    labels[core_index] = cluster_id;
    let mut queue: Vec<usize> = seeds.to_vec();
    for &neighbor in seeds {
        labels[neighbor] = cluster_id;
    }

    let mut position = 0;
    while position < queue.len() {
        let current = queue[position];
        position += 1;
        let neighbors = range_query(points, current, grid, grid_size, epsilon_sq);
        if neighbors.len() >= DBSCAN_MIN_PTS {
            for neighbor in neighbors {
                if labels[neighbor] == -1 {
                    labels[neighbor] = cluster_id;
                    queue.push(neighbor);
                }
            }
        }
    }
}

/// OPTICS fallback: delegate to DBSCAN since the configuration does not expose OPTICS.
pub fn optics(image: &RgbaImage, color_count: usize) -> Palette {
    dbscan(image, color_count)
}
