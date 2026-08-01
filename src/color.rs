#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Palette {
    pub colors: Vec<Color>,
}

impl Palette {
    pub fn new(colors: Vec<Color>) -> Self {
        Self { colors }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

#[derive(Clone, Debug)]
pub struct PaletteLabCache {
    pub colors: Vec<Color>,
    pub lab: Vec<Lab>,
}

impl PaletteLabCache {
    pub fn new(palette: &Palette) -> Self {
        let colors = palette.colors.clone();
        let lab = colors
            .iter()
            .map(|c| crate::utils::rgb_to_lab(*c))
            .collect();
        Self { colors, lab }
    }
}
