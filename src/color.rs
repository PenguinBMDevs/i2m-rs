//! Color types: RGBA colors, palettes, and pre-computed Lab caches.
//!
//! These types flow through the whole pipeline: [`crate::cluster`] produces a
//! [`Palette`], [`crate::match_color`] maps pixels back onto palette entries
//! (optionally using a [`PaletteLabCache`] for perceptual matching), and
//! [`crate::convert`] assigns one MIDI track per palette color.

/// An 8-bit-per-channel RGBA color.
///
/// Pixels with `a < 128` are treated as **transparent** throughout the crate
/// and never produce notes (see [`crate::match_color::TRANSPARENT_ID`]).
///
/// # Examples
///
/// ```
/// use i2m_rs::Color;
///
/// let red = Color::new(255, 0, 0, 255);
/// assert_eq!(red.r, 255);
/// assert_eq!(Color::BLACK, Color::new(0, 0, 0, 255));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red channel, `0..=255`.
    pub r: u8,
    /// Green channel, `0..=255`.
    pub g: u8,
    /// Blue channel, `0..=255`.
    pub b: u8,
    /// Alpha channel, `0..=255`. Values below 128 count as transparent.
    pub a: u8,
}

impl Color {
    /// Opaque black (`#000000`).
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Create a color from individual RGBA channels.
    ///
    /// This is a `const fn`, so it can be used in constants:
    ///
    /// ```
    /// use i2m_rs::Color;
    /// const BRAND: Color = Color::new(0x33, 0x66, 0xCC, 255);
    /// ```
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// An ordered list of colors, one per MIDI track.
///
/// The **index** of a color inside `colors` is its *color ID*: track `0` plays
/// color `0`, and so on. Palettes are usually built by
/// [`crate::cluster::generate_palette`], but can also be supplied by hand via
/// [`crate::config::PaletteSource::Manual`].
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, Palette};
///
/// // A two-color palette -> a two-track MIDI file.
/// let palette = Palette::new(vec![Color::BLACK, Color::new(255, 255, 255, 255)]);
/// assert_eq!(palette.colors.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Palette {
    /// The palette entries; each index doubles as a MIDI track index.
    pub colors: Vec<Color>,
}

impl Palette {
    /// Wrap a vector of colors into a palette.
    pub fn new(colors: Vec<Color>) -> Self {
        Self { colors }
    }
}

/// A color in the CIE L\*a\*b\* perceptual color space.
///
/// Produced by [`crate::utils::rgb_to_lab`] and used by the [`Lab`](crate::config::ColorIdMethod::Lab)
/// and [`Ciede2000`](crate::config::ColorIdMethod::Ciede2000) color-matching methods.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lab {
    /// Lightness, roughly `0..=100`.
    pub l: f64,
    /// Green–red opponent channel.
    pub a: f64,
    /// Blue–yellow opponent channel.
    pub b: f64,
}

/// A palette together with the Lab representation of every entry.
///
/// Converting RGB → Lab is comparatively expensive, so when matching thousands
/// of pixels with [`ColorIdMethod::Lab`](crate::config::ColorIdMethod::Lab) or
/// [`ColorIdMethod::Ciede2000`](crate::config::ColorIdMethod::Ciede2000) the
/// palette side is converted once and cached here. [`crate::convert`] builds
/// this cache automatically; you only need it when calling the
/// [`crate::match_color`] functions directly.
///
/// # Panics
///
/// [`crate::match_color::match_pixel`] panics if a Lab-based method is used
/// without passing a cache built from the *same* palette.
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, Palette, PaletteLabCache};
///
/// let palette = Palette::new(vec![Color::BLACK, Color::new(255, 255, 255, 255)]);
/// let cache = PaletteLabCache::new(&palette);
/// assert_eq!(cache.lab.len(), palette.colors.len());
/// ```
#[derive(Clone, Debug)]
pub struct PaletteLabCache {
    /// The original RGB palette entries.
    pub colors: Vec<Color>,
    /// `lab[i]` is the Lab conversion of `colors[i]`.
    pub lab: Vec<Lab>,
}

impl PaletteLabCache {
    /// Convert every color of `palette` to Lab and store both side by side.
    pub fn new(palette: &Palette) -> Self {
        let colors = palette.colors.clone();
        let lab = colors
            .iter()
            .map(|c| crate::utils::rgb_to_lab(*c))
            .collect();
        Self { colors, lab }
    }
}
