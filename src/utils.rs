use crate::color::{Color, Lab};

pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub fn is_white_key(key: u8) -> bool {
    matches!(key % 12, 0 | 2 | 4 | 5 | 7 | 9 | 11)
}

fn normalize_channel(value: u8) -> f64 {
    let v = value as f64 / 255.0;
    if v > 0.04045 {
        ((v + 0.055) / 1.055).powf(2.4)
    } else {
        v / 12.92
    }
}

fn lab_f(t: f64) -> f64 {
    let delta: f64 = 6.0 / 29.0;
    if t > delta.powi(3) {
        t.cbrt()
    } else {
        t / (3.0 * delta * delta) + 4.0 / 29.0
    }
}

pub fn rgb_to_lab(color: Color) -> Lab {
    let r = normalize_channel(color.r);
    let g = normalize_channel(color.g);
    let b = normalize_channel(color.b);

    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

    let xn = 0.95047;
    let yn = 1.00000;
    let zn = 1.08883;

    Lab {
        l: 116.0 * lab_f(y / yn) - 16.0,
        a: 500.0 * (lab_f(x / xn) - lab_f(y / yn)),
        b: 200.0 * (lab_f(y / yn) - lab_f(z / zn)),
    }
}

pub fn rgb_to_hsv(color: Color) -> (f64, f64, f64) {
    let r = color.r as f64 / 255.0;
    let g = color.g as f64 / 255.0;
    let b = color.b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let mut h = 0.0;

    if delta != 0.0 {
        if max == r {
            h = 60.0 * (((g - b) / delta) % 6.0);
        } else if max == g {
            h = 60.0 * ((b - r) / delta + 2.0);
        } else {
            h = 60.0 * ((r - g) / delta + 4.0);
        }
    }

    if h < 0.0 {
        h += 360.0;
    }

    (h, s, v)
}

pub fn rgb_to_hsl(color: Color) -> (f64, f64, f64) {
    let r = color.r as f64 / 255.0;
    let g = color.g as f64 / 255.0;
    let b = color.b as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if max == min {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let mut h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h /= 6.0;

    (h * 360.0, s, l)
}

pub fn rgb_to_hsl_key(color: Color) -> f64 {
    let (h, s, l) = rgb_to_hsl(color);
    // C# RgbToHslKey normalizes hue to [0, 1).
    (h / 360.0) * 10_000.0 + s * 100.0 + l
}

pub fn sample_pixels(pixels: &[Color], max_samples: usize) -> Vec<Color> {
    let opaque: Vec<Color> = pixels.iter().filter(|c| c.a >= 128).copied().collect();

    if opaque.len() <= max_samples {
        return opaque;
    }

    let step = opaque.len() / max_samples;
    if step == 0 {
        return opaque;
    }

    opaque.into_iter().step_by(step).take(max_samples).collect()
}

pub fn ciede2000(lab1: &Lab, lab2: &Lab) -> f64 {
    const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

    let c1 = (lab1.a * lab1.a + lab1.b * lab1.b).sqrt();
    let c2 = (lab2.a * lab2.a + lab2.b * lab2.b).sqrt();

    let c_bar = (c1 + c2) / 2.0;
    let c_bar_7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar_7 / (c_bar_7 + 25_f64.powi(7))).sqrt());

    let a1_prime = lab1.a * (1.0 + g);
    let a2_prime = lab2.a * (1.0 + g);

    let c1_prime = (a1_prime * a1_prime + lab1.b * lab1.b).sqrt();
    let c2_prime = (a2_prime * a2_prime + lab2.b * lab2.b).sqrt();

    let h1_prime = hue_angle(lab1.b, a1_prime);
    let h2_prime = hue_angle(lab2.b, a2_prime);

    let delta_l_prime = lab2.l - lab1.l;
    let delta_c_prime = c2_prime - c1_prime;
    let delta_h_rad = hue_diff(c1_prime, c2_prime, h1_prime, h2_prime).to_radians();
    let delta_h_prime = 2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_rad / 2.0).sin();

    let l_bar_prime = (lab1.l + lab2.l) / 2.0;
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;
    let h_bar_prime = mean_hue(c1_prime, c2_prime, h1_prime, h2_prime);

    let t = 1.0 - 0.17 * ((h_bar_prime - 30.0) * DEG_TO_RAD).cos()
        + 0.24 * ((2.0 * h_bar_prime) * DEG_TO_RAD).cos()
        + 0.32 * ((3.0 * h_bar_prime + 6.0) * DEG_TO_RAD).cos()
        - 0.20 * ((4.0 * h_bar_prime - 63.0) * DEG_TO_RAD).cos();

    let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let r_c = 2.0 * (c_bar_prime.powi(7) / (c_bar_prime.powi(7) + 25_f64.powi(7))).sqrt();

    let s_l =
        1.0 + (0.015 * (l_bar_prime - 50.0).powi(2)) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;

    let r_t = -((2.0 * delta_theta) * DEG_TO_RAD).sin() * r_c;

    let term_l = delta_l_prime / s_l;
    let term_c = delta_c_prime / s_c;
    let term_h = delta_h_prime / s_h;

    (term_l * term_l + term_c * term_c + term_h * term_h + r_t * term_c * term_h).sqrt()
}

fn hue_angle(b: f64, a_prime: f64) -> f64 {
    let h = b.atan2(a_prime).to_degrees();
    if h < 0.0 { h + 360.0 } else { h }
}

fn hue_diff(c1: f64, c2: f64, h1: f64, h2: f64) -> f64 {
    if c1 == 0.0 || c2 == 0.0 {
        return 0.0;
    }

    let delta = h2 - h1;
    if delta.abs() <= 180.0 {
        delta
    } else if h2 <= h1 {
        delta + 360.0
    } else {
        delta - 360.0
    }
}

fn mean_hue(c1: f64, c2: f64, h1: f64, h2: f64) -> f64 {
    if c1 == 0.0 || c2 == 0.0 {
        return h1 + h2;
    }

    let sum = h1 + h2;
    let delta = (h1 - h2).abs();

    if delta <= 180.0 {
        sum / 2.0
    } else if sum < 360.0 {
        (sum + 360.0) / 2.0
    } else {
        (sum - 360.0) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_white_key_detects_white_keys() {
        assert!(is_white_key(60)); // C4
        assert!(!is_white_key(61)); // C#4
    }

    #[test]
    fn rgb_to_lab_white_is_high_l() {
        let lab = rgb_to_lab(Color::new(255, 255, 255, 255));
        assert!(lab.l > 95.0, "expected white L to be high, got {}", lab.l);
        assert!(lab.a.abs() < 5.0);
        assert!(lab.b.abs() < 5.0);
    }

    #[test]
    fn rgb_to_hsv_red() {
        let (h, s, v) = rgb_to_hsv(Color::new(255, 0, 0, 255));
        assert!((h - 0.0).abs() < 1.0 || (h - 360.0).abs() < 1.0);
        assert!((s - 1.0).abs() < 1e-6);
        assert!((v - 1.0).abs() < 1e-6);
    }
}
