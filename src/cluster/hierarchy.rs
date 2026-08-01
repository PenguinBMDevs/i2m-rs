use crate::color::{Color, Palette};
use crate::image::RgbaImage;
use std::cmp;

use crate::cluster::{color_from_floats, sample_colors};

const MAX_SAMPLES: usize = 2_000;

/// Agglomerative hierarchical clustering on RGB colors (single linkage by default).
pub fn hierarchical(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }
    if color_count >= pixels.len() {
        let mut colors: Vec<Color> = pixels;
        while colors.len() < color_count {
            colors.push(colors.first().copied().unwrap_or(Color::BLACK));
        }
        colors.truncate(color_count);
        return Palette::new(colors);
    }

    let points: Vec<[f64; 3]> = pixels
        .iter()
        .map(|color| [f64::from(color.r), f64::from(color.g), f64::from(color.b)])
        .collect();
    let sample_count = points.len();

    let mut clusters: Vec<Vec<usize>> = (0..sample_count).map(|i| vec![i]).collect();
    let mut active = vec![true; sample_count];
    let mut active_count = sample_count;

    let mut distances = vec![0.0; sample_count * (sample_count - 1) / 2];
    for i in 0..sample_count {
        for j in 0..i {
            distances[pair_index(i, j)] = euclidean_distance(&points[i], &points[j]);
        }
    }

    let mut heap = std::collections::BinaryHeap::new();
    for i in 0..sample_count {
        for j in 0..i {
            heap.push(HeapItem {
                dist: distances[pair_index(i, j)],
                i,
                j,
            });
        }
    }

    while active_count > color_count {
        let item = loop {
            let candidate = heap.pop().expect("heap contains all pairs");
            if active[candidate.i] && active[candidate.j] {
                break candidate;
            }
        };

        let (a, b) = (item.i, item.j);
        if !active[a] || !active[b] {
            continue;
        }

        let merged: Vec<usize> = clusters[a]
            .iter()
            .chain(clusters[b].iter())
            .copied()
            .collect();
        clusters[a] = merged;
        active[b] = false;
        active_count -= 1;

        for k in 0..sample_count {
            if k == a || !active[k] {
                continue;
            }
            let new_dist = linkage_distance(&points, &clusters[a], &clusters[k], Linkage::Single);
            let index = if a > k {
                pair_index(a, k)
            } else {
                pair_index(k, a)
            };
            distances[index] = new_dist;
            heap.push(HeapItem {
                dist: new_dist,
                i: a,
                j: k,
            });
        }
    }

    let mut colors: Vec<Color> = Vec::new();
    for (index, cluster) in clusters.iter().enumerate() {
        if !active[index] || cluster.is_empty() {
            continue;
        }
        let mut sum = [0.0; 3];
        for &point_index in cluster {
            for channel in 0..3 {
                sum[channel] += points[point_index][channel];
            }
        }
        let count = cluster.len() as f64;
        colors.push(color_from_floats(
            sum[0] / count,
            sum[1] / count,
            sum[2] / count,
        ));
    }

    while colors.len() < color_count {
        colors.push(colors.first().copied().unwrap_or(Color::BLACK));
    }
    colors.truncate(color_count);
    Palette::new(colors)
}

/// Linkage strategy used by hierarchical clustering.
#[allow(dead_code)]
enum Linkage {
    Single,
    Complete,
    Average,
}

fn euclidean_distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn linkage_distance(
    points: &[[f64; 3]],
    cluster_a: &[usize],
    cluster_b: &[usize],
    linkage: Linkage,
) -> f64 {
    match linkage {
        Linkage::Single => {
            let mut min_distance = f64::MAX;
            for &i in cluster_a {
                for &j in cluster_b {
                    let distance = euclidean_distance(&points[i], &points[j]);
                    if distance < min_distance {
                        min_distance = distance;
                    }
                }
            }
            min_distance
        }
        Linkage::Complete => {
            let mut max_distance = -1.0;
            for &i in cluster_a {
                for &j in cluster_b {
                    let distance = euclidean_distance(&points[i], &points[j]);
                    if distance > max_distance {
                        max_distance = distance;
                    }
                }
            }
            max_distance
        }
        Linkage::Average => {
            let mut sum = 0.0;
            let mut count = 0usize;
            for &i in cluster_a {
                for &j in cluster_b {
                    sum += euclidean_distance(&points[i], &points[j]);
                    count += 1;
                }
            }
            if count == 0 { 0.0 } else { sum / count as f64 }
        }
    }
}

fn pair_index(i: usize, j: usize) -> usize {
    let (large, small) = if i > j { (i, j) } else { (j, i) };
    large * (large - 1) / 2 + small
}

#[derive(Clone, Copy, Debug)]
struct HeapItem {
    dist: f64,
    i: usize,
    j: usize,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.dist.total_cmp(&other.dist).is_eq()
    }
}

impl Eq for HeapItem {}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other.dist.total_cmp(&self.dist)
    }
}
