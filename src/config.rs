use crate::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMode {
    AllKeys,
    WhiteKeysFilled,
    WhiteKeysClipped,
    WhiteKeysFixed,
    BlackKeysFilled,
    BlackKeysClipped,
    BlackKeysFixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteLengthMode {
    Unlimited,
    FlowWithColor,
    SplitToGrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    AreaResampling,
    Bilinear,
    NearestNeighbor,
    Bicubic,
    Lanczos,
    Gaussian,
    Mitchell,
    BoxFilter,
    IntegralImage,
    ModePooling,
    Hermite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorIdMethod {
    Rgb,
    Hsv,
    Lab,
    Ciede2000,
    Hsl,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaletteSource {
    Manual(Vec<Color>),
    OnlyWpfMedianCut,
    OnlyKMeansPlusPlus,
    KMeans,
    KMeansPlusPlus,
    Popularity,
    Octree,
    VarianceSplit,
    Pca,
    MaxMin,
    NativeKMeans,
    MeanShift,
    Dbscan,
    Optics,
    Gmm,
    Hierarchical,
    Spectral,
    LabKMeans,
    FloydSteinbergDither,
    OrderedDither,
    FixedBitPalette,
}

#[derive(Clone, Debug)]
pub struct ConverterConfig {
    pub color_count: usize,
    pub palette: PaletteSource,
    pub start_key: u8,
    pub end_key: u8,
    pub key_mode: KeyMode,
    pub note_length_mode: NoteLengthMode,
    pub max_note_length: u32,
    pub target_height: u32,
    pub resize_algorithm: ResizeAlgorithm,
    pub color_id_method: ColorIdMethod,
    pub ticks_per_pixel: u32,
    pub ppq: u16,
    pub start_offset: u32,
    pub bpm: u16,
    pub emit_color_events: bool,
    pub random_colors: bool,
    pub random_color_seed: u32,
}

impl Default for ConverterConfig {
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
