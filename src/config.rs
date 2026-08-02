//! Conversion configuration.
//!
//! [`ConverterConfig`] is the single knob-panel for the whole pipeline: which
//! palette algorithm to use ([`PaletteSource`]), how image columns map onto
//! MIDI keys ([`KeyMode`]), how long notes may ring ([`NoteLengthMode`]), how
//! the image is resampled ([`ResizeAlgorithm`]) and how pixels are matched to
//! palette entries ([`ColorIdMethod`]).
//!
//! # Examples
//!
//! ```
//! use i2m_rs::{ConverterConfig, KeyMode, PaletteSource, ResizeAlgorithm};
//!
//! // Start from the defaults (A0..C8, K-Means++, 16 colors) and tweak:
//! let mut config = ConverterConfig::default();
//! config.color_count = 32;
//! config.palette = PaletteSource::Octree;
//! config.key_mode = KeyMode::WhiteKeysFilled; // diatonic feel
//! config.resize_algorithm = ResizeAlgorithm::Lanczos;
//! config.bpm = 140;
//! ```

use crate::color::Color;

/// How image columns are mapped onto MIDI keys.
///
/// The image is resized so its width matches the number of columns the mode
/// produces; column `x` then plays the key at position `x` in the key list
/// (see [`crate::convert::build_key_list`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMode {
    /// Every key in `start_key..=end_key` is used, chromatically.
    AllKeys,
    /// Only white keys are used; the image width equals the *white-key count*
    /// (the image is "filled" onto white keys, black keys never sound).
    WhiteKeysFilled,
    /// Image width equals the full key range, but columns that would land on a
    /// black key are skipped ("clipped" away).
    WhiteKeysClipped,
    /// Like [`WhiteKeysClipped`](Self::WhiteKeysClipped), but keys are
    /// addressed by `start_key + column` and black-key hits are dropped.
    WhiteKeysFixed,
    /// Only black keys are used; the image width equals the *black-key count*.
    BlackKeysFilled,
    /// Image width equals the full key range; white-key columns are skipped.
    BlackKeysClipped,
    /// Like [`BlackKeysClipped`](Self::BlackKeysClipped), addressed by
    /// `start_key + column`, dropping white-key hits.
    BlackKeysFixed,
}

/// How the length of generated notes is limited.
///
/// Only takes effect when [`ConverterConfig::max_note_length`] is non-zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteLengthMode {
    /// Notes ring for the entire vertical run of their color (no limit).
    Unlimited,
    /// A note is re-triggered after `max_note_length` ticks even if its color
    /// run continues (the note "flows" but is chopped).
    FlowWithColor,
    /// Notes are split at a fixed grid: every `max_note_length` rows from the
    /// bottom of the image, regardless of color changes.
    SplitToGrid,
}

/// Interpolation algorithm used when resizing the image to the key range.
///
/// See [`crate::resize::resize`]. As a rule of thumb:
/// [`NearestNeighbor`](Self::NearestNeighbor) keeps hard pixel edges,
/// [`AreaResampling`](Self::AreaResampling) is the balanced default, and
/// [`Lanczos`](Self::Lanczos) gives the sharpest high-quality result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    /// Weighted average over the source area covered by each output pixel.
    AreaResampling,
    /// Bilinear interpolation over the 2×2 neighborhood.
    Bilinear,
    /// Nearest-neighbor sampling (no smoothing).
    NearestNeighbor,
    /// Cubic convolution over the 4×4 neighborhood.
    Bicubic,
    /// Lanczos-3 windowed sinc filter.
    Lanczos,
    /// Gaussian filter (σ = 1, radius 2).
    Gaussian,
    /// Mitchell–Netravali filter (B = C = 1/3).
    Mitchell,
    /// Unweighted box average over the covered source rectangle.
    BoxFilter,
    /// Box average accelerated by an integral (summed-area) image.
    IntegralImage,
    /// The most frequent source color in each covered rectangle wins.
    ModePooling,
    /// Hermite interpolation over the 2×2 neighborhood.
    Hermite,
}

/// Metric used to match a pixel to its nearest palette entry.
///
/// See [`crate::match_color::match_pixel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorIdMethod {
    /// Euclidean distance in sRGB space (fastest, the default).
    Rgb,
    /// Weighted distance in HSV space; low-saturation/low-value colors are
    /// matched mostly by value.
    Hsv,
    /// Euclidean distance in CIE Lab space (perceptual).
    Lab,
    /// CIEDE2000 delta-E (most perceptually accurate, slowest).
    Ciede2000,
    /// Weighted distance in HSL space; dark/low-saturation colors are matched
    /// mostly by lightness.
    Hsl,
}

/// Where the color palette comes from.
///
/// Every variant except [`Manual`](Self::Manual) runs an automatic
/// palette-generation algorithm on the image (see
/// [`crate::cluster::generate_palette`]).
#[derive(Clone, Debug, PartialEq)]
pub enum PaletteSource {
    /// Use exactly these colors; no clustering is performed.
    Manual(Vec<Color>),
    /// WPF median cut. **Currently a fallback to K-Means++** because the WPF
    /// algorithm is not available in Rust.
    OnlyWpfMedianCut,
    /// K-Means++ initialization only, without Lloyd refinement.
    OnlyKMeansPlusPlus,
    /// Lloyd-style K-Means with randomly shuffled initial centers.
    KMeans,
    /// Lloyd-style K-Means with K-Means++ initialization (default).
    KMeansPlusPlus,
    /// The `color_count` most frequent opaque colors.
    Popularity,
    /// 8-level octree color quantization.
    Octree,
    /// Iteratively split the RGB box with the largest variance at its median.
    VarianceSplit,
    /// Sample colors along the first principal component of RGB covariance.
    Pca,
    /// Frequency-weighted Max-Min center selection, then a few Lloyd steps.
    MaxMin,
    /// Incremental K-Means with a fixed learning rate.
    NativeKMeans,
    /// Mean-shift clustering with a Gaussian kernel (bandwidth 32).
    MeanShift,
    /// DBSCAN density clustering with grid acceleration.
    Dbscan,
    /// OPTICS — **currently a fallback to [`Dbscan`](Self::Dbscan)**.
    Optics,
    /// Gaussian mixture model with diagonal covariance (EM, K-Means++ init).
    Gmm,
    /// Agglomerative hierarchical clustering (single linkage).
    Hierarchical,
    /// Spectral clustering in Lab space over a KNN similarity graph.
    Spectral,
    /// K-Means performed in CIE Lab space.
    LabKMeans,
    /// K-Means++ palette, then Floyd–Steinberg dither the image into it.
    FloydSteinbergDither,
    /// K-Means++ palette, then 4×4 Bayer ordered dithering.
    OrderedDither,
    /// A fixed bit-depth palette (2/4/16 colors get classic game/PC palettes).
    FixedBitPalette,
}

/// Every tunable of the image→MIDI conversion.
///
/// The [`Default`] implementation reproduces the original C# tool's defaults
/// and is the recommended starting point.
///
/// # Field summary
///
/// | Group | Fields |
/// |-------|--------|
/// | Palette | [`color_count`](Self::color_count), [`palette`](Self::palette), [`color_id_method`](Self::color_id_method) |
/// | Key mapping | [`start_key`](Self::start_key), [`end_key`](Self::end_key), [`key_mode`](Self::key_mode) |
/// | Note length | [`note_length_mode`](Self::note_length_mode), [`max_note_length`](Self::max_note_length) |
/// | Image sizing | [`target_height`](Self::target_height), [`resize_algorithm`](Self::resize_algorithm) |
/// | MIDI timing | [`ticks_per_pixel`](Self::ticks_per_pixel), [`ppq`](Self::ppq), [`start_offset`](Self::start_offset), [`bpm`](Self::bpm) |
/// | Extra events | [`emit_color_events`](Self::emit_color_events), [`random_colors`](Self::random_colors), [`random_color_seed`](Self::random_color_seed) |
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, ConverterConfig, PaletteSource};
///
/// // Convert a black-and-white score image with a hand-made palette:
/// let config = ConverterConfig {
///     color_count: 2,
///     palette: PaletteSource::Manual(vec![Color::BLACK, Color::new(255, 255, 255, 255)]),
///     start_key: 36, // C2
///     end_key: 96,   // C7
///     ..Default::default()
/// };
/// assert_eq!(config.end_key - config.start_key, 60);
/// ```
#[derive(Clone, Debug)]
pub struct ConverterConfig {
    /// Number of palette colors (== number of MIDI tracks). Must be `> 0`.
    pub color_count: usize,
    /// Palette source algorithm, or a manual palette.
    pub palette: PaletteSource,
    /// Lowest MIDI key used (0–127). Default 21 = A0 (lowest piano key).
    pub start_key: u8,
    /// Highest MIDI key used (0–127). Default 108 = C8 (highest piano key).
    ///
    /// Must be `>= start_key`, and the resulting image width must fit in
    /// `u8` (the effective range is limited to 255 columns).
    pub end_key: u8,
    /// How columns map onto keys; see [`KeyMode`].
    pub key_mode: KeyMode,
    /// How note lengths are limited; see [`NoteLengthMode`].
    pub note_length_mode: NoteLengthMode,
    /// Maximum note length in pixel-ticks. `0` disables the limit.
    pub max_note_length: u32,
    /// Force the resized image height in pixels. `0` = keep the source aspect
    /// ratio relative to the effective width.
    pub target_height: u32,
    /// Resampling filter used by the resize step; see [`ResizeAlgorithm`].
    pub resize_algorithm: ResizeAlgorithm,
    /// Color metric for pixel→palette matching; see [`ColorIdMethod`].
    pub color_id_method: ColorIdMethod,
    /// MIDI ticks produced per pixel row. Values `> 1` slow the music down.
    /// `0` is clamped to `1` by the writer.
    pub ticks_per_pixel: u32,
    /// Pulses (ticks) per quarter note written into the MIDI header.
    pub ppq: u16,
    /// Ticks inserted before the first note (useful to chain multiple
    /// conversions into one file).
    pub start_offset: u32,
    /// Tempo of the output file. The writer inserts one tempo meta event
    /// computed as `60_000_000 / bpm` microseconds per quarter.
    pub bpm: u16,
    /// If `true`, prepend an `0x0A` "unknown" meta event carrying the RGBA of
    /// each track's color at tick [`start_offset`](Self::start_offset) (see
    /// [`crate::midi::writer::color_event_payload`]).
    pub emit_color_events: bool,
    /// Reserved: replace palette colors with random colors before writing.
    /// **Not yet implemented.**
    pub random_colors: bool,
    /// Seed for [`random_colors`](Self::random_colors).
    pub random_color_seed: u32,
}

impl Default for ConverterConfig {
    /// Defaults mirroring the original C# tool: 16 colors via K-Means++,
    /// full 88-key piano range (A0–C8), all keys, unlimited note length,
    /// area-resampling resize, RGB matching, 96 PPQ at 120 BPM.
    fn default() -> Self {
        Self {
            color_count: 16,
            palette: PaletteSource::KMeansPlusPlus,
            start_key: 21,
            end_key: 108,
            key_mode: KeyMode::AllKeys,
            note_length_mode: NoteLengthMode::Unlimited,
            max_note_length: 0,
            target_height: 0,
            resize_algorithm: ResizeAlgorithm::AreaResampling,
            color_id_method: ColorIdMethod::Rgb,
            ticks_per_pixel: 1,
            ppq: 96,
            start_offset: 0,
            bpm: 120,
            emit_color_events: false,
            random_colors: false,
            random_color_seed: 0,
        }
    }
}
