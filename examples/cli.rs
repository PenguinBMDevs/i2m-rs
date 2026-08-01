use clap::{Parser, ValueEnum};
use i2m_rs::{
    ColorIdMethod, ConverterConfig, KeyMode, NoteLengthMode, PaletteSource, ResizeAlgorithm,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "i2m-cli", about = "Convert an image to a MIDI file")]
struct Args {
    /// Input image path
    input: PathBuf,
    /// Output MIDI path
    output: PathBuf,

    /// Number of colors in the palette
    #[arg(short, long, default_value_t = 16)]
    colors: usize,

    /// Palette generation method
    #[arg(long, value_enum, default_value_t = PaletteMethod::KMeansPlusPlus)]
    palette: PaletteMethod,

    /// Key mode (all, white-filled, white-clipped, white-fixed, black-filled, black-clipped, black-fixed)
    #[arg(long, value_enum, default_value_t = KeyMethod::All)]
    key_mode: KeyMethod,

    /// First MIDI key to use
    #[arg(long, default_value_t = 21)]
    start_key: u8,

    /// Last MIDI key to use
    #[arg(long, default_value_t = 108)]
    end_key: u8,

    /// Target height in pixels (width follows key range)
    #[arg(long, default_value_t = 120)]
    target_height: u32,

    /// Resize algorithm
    #[arg(long, value_enum, default_value_t = ResizeMethod::Area)]
    resize: ResizeMethod,

    /// Color matching method
    #[arg(long, value_enum, default_value_t = ColorMatchMethod::Rgb)]
    color_match: ColorMatchMethod,

    /// Note length mode
    #[arg(long, value_enum, default_value_t = NoteLengthArg::Unlimited)]
    note_length: NoteLengthArg,

    /// Maximum note length in rows (0 = unlimited)
    #[arg(long, default_value_t = 0)]
    max_note_length: u32,

    /// MIDI ticks per image row
    #[arg(long, default_value_t = 1)]
    ticks_per_pixel: u32,

    /// MIDI pulses per quarter note
    #[arg(long, default_value_t = 96)]
    ppq: u16,

    /// Tempo in beats per minute
    #[arg(long, default_value_t = 120)]
    bpm: u16,

    /// Emit color meta events per track
    #[arg(long)]
    color_events: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum PaletteMethod {
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

#[derive(Clone, Debug, ValueEnum)]
enum KeyMethod {
    All,
    WhiteFilled,
    WhiteClipped,
    WhiteFixed,
    BlackFilled,
    BlackClipped,
    BlackFixed,
}

#[derive(Clone, Debug, ValueEnum)]
enum ResizeMethod {
    Area,
    Bilinear,
    Nearest,
    Bicubic,
    Lanczos,
    Gaussian,
    Mitchell,
    Box,
    Integral,
    Mode,
    Hermite,
}

#[derive(Clone, Debug, ValueEnum)]
enum ColorMatchMethod {
    Rgb,
    Hsv,
    Lab,
    Ciede2000,
    Hsl,
}

#[derive(Clone, Debug, ValueEnum)]
enum NoteLengthArg {
    Unlimited,
    FlowWithColor,
    SplitToGrid,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config = ConverterConfig {
        color_count: args.colors,
        palette: palette_source_from(args.palette),
        start_key: args.start_key,
        end_key: args.end_key,
        key_mode: key_mode_from(args.key_mode),
        note_length_mode: note_length_mode_from(args.note_length),
        max_note_length: args.max_note_length,
        target_height: args.target_height,
        resize_algorithm: resize_algorithm_from(args.resize),
        color_id_method: color_id_method_from(args.color_match),
        ticks_per_pixel: args.ticks_per_pixel,
        ppq: args.ppq,
        start_offset: 0,
        bpm: args.bpm,
        emit_color_events: args.color_events,
        random_colors: false,
        random_color_seed: 0,
    };

    let image = i2m_rs::load_image(&args.input)?;
    eprintln!("Loaded {}x{} image", image.width, image.height);

    let (palette, _dithered) =
        i2m_rs::cluster::generate_palette(&image, &config.palette, config.color_count)?;
    eprintln!("Generated palette with {} colors", palette.colors.len());

    let progress = |stage: i2m_rs::Stage, fraction: f64| {
        eprintln!("{:?}: {:.0}%", stage, fraction * 100.0);
    };

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = i2m_rs::convert(&image, &palette, &config, Some(&progress), &cancel)?;
    eprintln!(
        "Converted to {} notes across {} tracks",
        result.note_count,
        result.track_events.len()
    );

    let results: Vec<&i2m_rs::ConversionResult> = vec![&result];
    i2m_rs::write_midi(&args.output, &results, &config)?;
    eprintln!("Wrote MIDI to {}", args.output.display());

    Ok(())
}

fn palette_source_from(method: PaletteMethod) -> PaletteSource {
    match method {
        PaletteMethod::OnlyKMeansPlusPlus => PaletteSource::OnlyKMeansPlusPlus,
        PaletteMethod::KMeans => PaletteSource::KMeans,
        PaletteMethod::KMeansPlusPlus => PaletteSource::KMeansPlusPlus,
        PaletteMethod::Popularity => PaletteSource::Popularity,
        PaletteMethod::Octree => PaletteSource::Octree,
        PaletteMethod::VarianceSplit => PaletteSource::VarianceSplit,
        PaletteMethod::Pca => PaletteSource::Pca,
        PaletteMethod::MaxMin => PaletteSource::MaxMin,
        PaletteMethod::NativeKMeans => PaletteSource::NativeKMeans,
        PaletteMethod::MeanShift => PaletteSource::MeanShift,
        PaletteMethod::Dbscan => PaletteSource::Dbscan,
        PaletteMethod::Optics => PaletteSource::Optics,
        PaletteMethod::Gmm => PaletteSource::Gmm,
        PaletteMethod::Hierarchical => PaletteSource::Hierarchical,
        PaletteMethod::Spectral => PaletteSource::Spectral,
        PaletteMethod::LabKMeans => PaletteSource::LabKMeans,
        PaletteMethod::FloydSteinbergDither => PaletteSource::FloydSteinbergDither,
        PaletteMethod::OrderedDither => PaletteSource::OrderedDither,
        PaletteMethod::FixedBitPalette => PaletteSource::FixedBitPalette,
    }
}

fn key_mode_from(method: KeyMethod) -> KeyMode {
    match method {
        KeyMethod::All => KeyMode::AllKeys,
        KeyMethod::WhiteFilled => KeyMode::WhiteKeysFilled,
        KeyMethod::WhiteClipped => KeyMode::WhiteKeysClipped,
        KeyMethod::WhiteFixed => KeyMode::WhiteKeysFixed,
        KeyMethod::BlackFilled => KeyMode::BlackKeysFilled,
        KeyMethod::BlackClipped => KeyMode::BlackKeysClipped,
        KeyMethod::BlackFixed => KeyMode::BlackKeysFixed,
    }
}

fn resize_algorithm_from(method: ResizeMethod) -> ResizeAlgorithm {
    match method {
        ResizeMethod::Area => ResizeAlgorithm::AreaResampling,
        ResizeMethod::Bilinear => ResizeAlgorithm::Bilinear,
        ResizeMethod::Nearest => ResizeAlgorithm::NearestNeighbor,
        ResizeMethod::Bicubic => ResizeAlgorithm::Bicubic,
        ResizeMethod::Lanczos => ResizeAlgorithm::Lanczos,
        ResizeMethod::Gaussian => ResizeAlgorithm::Gaussian,
        ResizeMethod::Mitchell => ResizeAlgorithm::Mitchell,
        ResizeMethod::Box => ResizeAlgorithm::BoxFilter,
        ResizeMethod::Integral => ResizeAlgorithm::IntegralImage,
        ResizeMethod::Mode => ResizeAlgorithm::ModePooling,
        ResizeMethod::Hermite => ResizeAlgorithm::Hermite,
    }
}

fn color_id_method_from(method: ColorMatchMethod) -> ColorIdMethod {
    match method {
        ColorMatchMethod::Rgb => ColorIdMethod::Rgb,
        ColorMatchMethod::Hsv => ColorIdMethod::Hsv,
        ColorMatchMethod::Lab => ColorIdMethod::Lab,
        ColorMatchMethod::Ciede2000 => ColorIdMethod::Ciede2000,
        ColorMatchMethod::Hsl => ColorIdMethod::Hsl,
    }
}

fn note_length_mode_from(arg: NoteLengthArg) -> NoteLengthMode {
    match arg {
        NoteLengthArg::Unlimited => NoteLengthMode::Unlimited,
        NoteLengthArg::FlowWithColor => NoteLengthMode::FlowWithColor,
        NoteLengthArg::SplitToGrid => NoteLengthMode::SplitToGrid,
    }
}
