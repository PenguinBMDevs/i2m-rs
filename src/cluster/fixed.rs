use crate::color::{Color, Palette};

/// Build a fixed-bit palette.
///
/// * `color_count` — requested palette size.
/// * `use_gray` — if true, produce an evenly spaced grayscale ramp.
///
/// Special cases: 2 colors are black/white, 4 colors are the Game Boy ramp,
/// and 16 colors are the Windows standard 16-color palette. Other sizes
/// use round-robin bit allocation across RGB and are padded/truncated to
/// the requested size.
pub fn fixed_bit_palette(color_count: usize, use_gray: bool) -> Palette {
    let mut colors = if use_gray {
        gray_palette(color_count)
    } else if color_count == 2 {
        vec![Color::BLACK, Color::new(255, 255, 255, 255)]
    } else if color_count == 4 {
        vec![
            Color::BLACK,
            Color::new(85, 85, 85, 255),
            Color::new(170, 170, 170, 255),
            Color::new(255, 255, 255, 255),
        ]
    } else if color_count == 16 {
        windows_16_palette()
    } else {
        let bit_depth = color_count.next_power_of_two().trailing_zeros() as usize;
        let bits = allocate_bits(bit_depth, 3);
        let r_levels = 1usize << bits[0];
        let g_levels = 1usize << bits[1];
        let b_levels = 1usize << bits[2];
        let mut palette = Vec::with_capacity(r_levels * g_levels * b_levels);
        for r in 0..r_levels {
            for g in 0..g_levels {
                for b in 0..b_levels {
                    palette.push(Color::new(
                        quantize(r, r_levels),
                        quantize(g, g_levels),
                        quantize(b, b_levels),
                        255,
                    ));
                }
            }
        }
        palette
    };

    while colors.len() < color_count {
        colors.push(Color::BLACK);
    }
    colors.truncate(color_count);
    Palette::new(colors)
}

fn gray_palette(color_count: usize) -> Vec<Color> {
    if color_count == 0 {
        return Vec::new();
    }
    let max = (color_count - 1).max(1);
    (0..color_count)
        .map(|i| {
            let gray = (i * 255 / max) as u8;
            Color::new(gray, gray, gray, 255)
        })
        .collect()
}

fn quantize(level: usize, levels: usize) -> u8 {
    if levels <= 1 {
        return 0;
    }
    ((level * 255) / (levels - 1)) as u8
}

/// Allocate `total_bits` across `channels` so that later channels receive any
/// remainder, mimicking the C# round-robin bit allocation reversed.
fn allocate_bits(total_bits: usize, channels: usize) -> Vec<usize> {
    let mut bits = vec![0; channels];
    for i in 0..total_bits {
        bits[i % channels] += 1;
    }
    bits.reverse();
    bits
}

fn windows_16_palette() -> Vec<Color> {
    vec![
        Color::new(0, 0, 0, 255),
        Color::new(128, 0, 0, 255),
        Color::new(0, 128, 0, 255),
        Color::new(128, 128, 0, 255),
        Color::new(0, 0, 128, 255),
        Color::new(128, 0, 128, 255),
        Color::new(0, 128, 128, 255),
        Color::new(192, 192, 192, 255),
        Color::new(128, 128, 128, 255),
        Color::new(255, 0, 0, 255),
        Color::new(0, 255, 0, 255),
        Color::new(255, 255, 0, 255),
        Color::new(0, 0, 255, 255),
        Color::new(255, 0, 255, 255),
        Color::new(0, 255, 255, 255),
        Color::new(255, 255, 255, 255),
    ]
}
