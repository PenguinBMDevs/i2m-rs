use crate::color::{Color, Palette};
use crate::image::RgbaImage;

use crate::cluster::{color_from_floats, sample_colors};

const MAX_SAMPLES: usize = 20_000;
const MAX_LEVEL: usize = 7;

/// Build a palette from the most frequent opaque colors in the image.
pub fn popularity(image: &RgbaImage, color_count: usize) -> Palette {
    use std::collections::HashMap;

    let pixels = sample_colors(image, MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let mut frequencies: HashMap<u32, usize> = HashMap::new();
    for color in &pixels {
        let key = color_key(color);
        *frequencies.entry(key).or_insert(0) += 1;
    }

    let mut entries: Vec<(u32, usize)> = frequencies.into_iter().collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.1));

    let mut colors: Vec<Color> = entries
        .into_iter()
        .take(color_count)
        .map(|(key, _)| color_from_key(key))
        .collect();

    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
    Palette::new(colors)
}

/// Pack RGB channels into a single u32 key.
fn color_key(color: &Color) -> u32 {
    (u32::from(color.r) << 16) | (u32::from(color.g) << 8) | u32::from(color.b)
}

/// Unpack a u32 key back into a `Color`.
fn color_from_key(key: u32) -> Color {
    Color::new(
        ((key >> 16) & 0xFF) as u8,
        ((key >> 8) & 0xFF) as u8,
        (key & 0xFF) as u8,
        255,
    )
}

/// Build a palette using an 8-level octree quantizer.
pub fn octree(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let mut octree = Octree::new(MAX_LEVEL);
    for color in &pixels {
        octree.insert(color);
    }

    octree.reduce_to(color_count);
    octree.palette(color_count)
}

struct OctreeNode {
    children: [Option<usize>; 8],
    parent: Option<usize>,
    level: usize,
    red: u64,
    green: u64,
    blue: u64,
    count: u64,
    is_leaf: bool,
}

impl OctreeNode {
    fn new(level: usize, parent: Option<usize>) -> Self {
        Self {
            children: [None; 8],
            parent,
            level,
            red: 0,
            green: 0,
            blue: 0,
            count: 0,
            is_leaf: false,
        }
    }
}

struct Octree {
    nodes: Vec<OctreeNode>,
    leaves_by_level: Vec<Vec<usize>>,
}

impl Octree {
    fn new(max_level: usize) -> Self {
        Self {
            nodes: vec![OctreeNode::new(0, None)],
            leaves_by_level: (0..=max_level).map(|_| Vec::new()).collect(),
        }
    }

    fn insert(&mut self, color: &Color) {
        let mut node_index = 0;
        self.nodes[node_index].red += u64::from(color.r);
        self.nodes[node_index].green += u64::from(color.g);
        self.nodes[node_index].blue += u64::from(color.b);
        self.nodes[node_index].count += 1;

        for level in 0..MAX_LEVEL {
            let child_index = octree_index(color, level);
            if self.nodes[node_index].children[child_index].is_none() {
                let new_node = OctreeNode::new(level + 1, Some(node_index));
                self.nodes.push(new_node);
                let new_index = self.nodes.len() - 1;
                self.nodes[node_index].children[child_index] = Some(new_index);
            }
            node_index = self.nodes[node_index].children[child_index].unwrap();
            self.nodes[node_index].red += u64::from(color.r);
            self.nodes[node_index].green += u64::from(color.g);
            self.nodes[node_index].blue += u64::from(color.b);
            self.nodes[node_index].count += 1;
        }

        if !self.nodes[node_index].is_leaf {
            self.nodes[node_index].is_leaf = true;
            self.leaves_by_level[MAX_LEVEL].push(node_index);
        }
    }

    fn reduce_to(&mut self, target_count: usize) {
        let mut total_leaves = self
            .leaves_by_level
            .iter()
            .map(|level| level.len())
            .sum::<usize>();
        while total_leaves > target_count {
            let mut level = MAX_LEVEL;
            while level > 0 && self.leaves_by_level[level].is_empty() {
                level -= 1;
            }
            if level == 0 {
                break;
            }
            let leaf_index = self.leaves_by_level[level]
                .pop()
                // SAFETY: the loop only selects levels that are non-empty.
                .expect("non-empty leaf level");
            self.merge_leaf(leaf_index);
            total_leaves -= 1;
        }
    }

    fn merge_leaf(&mut self, leaf_index: usize) {
        let (red, green, blue, count, parent_index) = {
            let leaf = &self.nodes[leaf_index];
            (
                leaf.red,
                leaf.green,
                leaf.blue,
                leaf.count,
                // SAFETY: only the deepest merged leaf is popped, never the root.
                leaf.parent.expect("root is never merged"),
            )
        };
        let parent_level = self.nodes[parent_index].level;
        self.nodes[parent_index].red += red;
        self.nodes[parent_index].green += green;
        self.nodes[parent_index].blue += blue;
        self.nodes[parent_index].count += count;
        self.nodes[leaf_index].is_leaf = false;

        let has_leaf_child = self.nodes[parent_index]
            .children
            .iter()
            .any(|child| child.is_some_and(|index| self.nodes[index].is_leaf));
        if !has_leaf_child {
            self.nodes[parent_index].is_leaf = true;
            self.leaves_by_level[parent_level].push(parent_index);
        }
    }

    fn palette(&self, color_count: usize) -> Palette {
        let mut colors = Vec::new();
        for level in &self.leaves_by_level {
            for &index in level {
                let node = &self.nodes[index];
                if node.count > 0 {
                    colors.push(node.average_color());
                }
            }
        }

        if colors.is_empty() {
            colors.push(Color::BLACK);
        }
        while colors.len() < color_count {
            colors.push(colors[0]);
        }
        colors.truncate(color_count);
        Palette::new(colors)
    }
}

fn octree_index(color: &Color, level: usize) -> usize {
    let shift = 7 - level;
    let index = (((u16::from(color.r) >> shift) & 1) << 2)
        | (((u16::from(color.g) >> shift) & 1) << 1)
        | ((u16::from(color.b) >> shift) & 1);
    usize::from(index)
}

impl OctreeNode {
    fn average_color(&self) -> Color {
        if self.count == 0 {
            return Color::BLACK;
        }
        color_from_floats(
            (self.red / self.count) as f64,
            (self.green / self.count) as f64,
            (self.blue / self.count) as f64,
        )
    }
}

/// Build a palette by iteratively splitting the RGB box with the largest variance.
pub fn variance_split(image: &RgbaImage, color_count: usize) -> Palette {
    let pixels = sample_colors(image, MAX_SAMPLES);
    if pixels.is_empty() {
        return Palette::new(vec![Color::BLACK; color_count.max(1)]);
    }

    let mut boxes: Vec<Vec<Color>> = vec![pixels];
    while boxes.len() < color_count {
        let mut max_variance = -1.0;
        let mut max_index = 0;
        let mut max_axis = 0;

        for (index, box_colors) in boxes.iter().enumerate() {
            if box_colors.len() < 2 {
                continue;
            }
            let (variance, axis) = box_variance_and_axis(box_colors);
            if variance > max_variance {
                max_variance = variance;
                max_index = index;
                max_axis = axis;
            }
        }

        if max_variance < 0.0 {
            break;
        }

        let mut target = boxes.remove(max_index);
        target.sort_by_key(|color| channel_value(color, max_axis));
        let mid = target.len() / 2;
        let second = target.split_off(mid);
        boxes.push(target);
        boxes.push(second);
    }

    let mut colors: Vec<Color> = boxes
        .into_iter()
        .map(|box_colors| mean_color(&box_colors))
        .collect();

    while colors.len() < color_count {
        colors.push(colors[0]);
    }
    colors.truncate(color_count);
    Palette::new(colors)
}

/// Compute the variance of a box and the channel with the largest variance.
fn box_variance_and_axis(box_colors: &[Color]) -> (f64, usize) {
    let mean = mean_color(box_colors);
    let mean_values = [f64::from(mean.r), f64::from(mean.g), f64::from(mean.b)];
    let mut variances = [0.0; 3];

    for color in box_colors {
        let values = [f64::from(color.r), f64::from(color.g), f64::from(color.b)];
        for axis in 0..3 {
            let diff = values[axis] - mean_values[axis];
            variances[axis] += diff * diff;
        }
    }

    let mut max_axis = 0;
    for axis in 1..3 {
        if variances[axis] > variances[max_axis] {
            max_axis = axis;
        }
    }
    (variances[max_axis], max_axis)
}

/// Get the value of a color along the requested axis (0=R, 1=G, 2=B).
fn channel_value(color: &Color, axis: usize) -> u8 {
    match axis {
        0 => color.r,
        1 => color.g,
        2 => color.b,
        _ => color.r,
    }
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
