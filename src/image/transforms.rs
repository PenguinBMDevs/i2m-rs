use crate::color::Color;
use crate::image::RgbaImage;

pub fn apply_transforms(image: RgbaImage, rotation: i32, flip: bool, grayscale: bool) -> RgbaImage {
    let rotated = rotate_image(image, rotation);
    let flipped = if flip {
        flip_horizontal(rotated)
    } else {
        rotated
    };
    if grayscale {
        to_grayscale(flipped)
    } else {
        flipped
    }
}

fn rotate_image(image: RgbaImage, rotation: i32) -> RgbaImage {
    match rotation.rem_euclid(360) {
        90 => rotate_90(image),
        180 => rotate_180(image),
        270 => rotate_270(image),
        _ => image,
    }
}

fn rotate_90(image: RgbaImage) -> RgbaImage {
    let (old_w, old_h) = (image.width, image.height);
    let mut out = RgbaImage::new(old_h, old_w, Color::BLACK);

    for y in 0..old_h {
        for x in 0..old_w {
            let color = image.get(x, y);
            out.set(y, old_w - 1 - x, color);
        }
    }

    out
}

fn rotate_180(image: RgbaImage) -> RgbaImage {
    let (w, h) = (image.width, image.height);
    let mut out = RgbaImage::new(w, h, Color::BLACK);

    for y in 0..h {
        for x in 0..w {
            out.set(w - 1 - x, h - 1 - y, image.get(x, y));
        }
    }

    out
}

fn rotate_270(image: RgbaImage) -> RgbaImage {
    let (old_w, old_h) = (image.width, image.height);
    let mut out = RgbaImage::new(old_h, old_w, Color::BLACK);

    for y in 0..old_h {
        for x in 0..old_w {
            out.set(old_h - 1 - y, x, image.get(x, y));
        }
    }

    out
}

fn flip_horizontal(image: RgbaImage) -> RgbaImage {
    let (w, h) = (image.width, image.height);
    let mut out = RgbaImage::new(w, h, Color::BLACK);

    for y in 0..h {
        for x in 0..w {
            out.set(w - 1 - x, y, image.get(x, y));
        }
    }

    out
}

fn to_grayscale(image: RgbaImage) -> RgbaImage {
    let (w, h) = (image.width, image.height);
    let mut out = RgbaImage::new(w, h, Color::BLACK);

    for y in 0..h {
        for x in 0..w {
            let c = image.get(x, y);
            let luma = (0.299 * f64::from(c.r) + 0.587 * f64::from(c.g) + 0.114 * f64::from(c.b))
                .round() as u8;
            out.set(x, y, Color::new(luma, luma, luma, c.a));
        }
    }

    out
}
