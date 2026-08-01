use i2m_rs::{
    Color, ColorIdMethod, ConverterConfig, KeyMode, NoteLengthMode, PaletteSource, ResizeAlgorithm,
    cluster,
};

fn default_config() -> ConverterConfig {
    ConverterConfig {
        color_count: 1,
        palette: PaletteSource::Manual(vec![Color::new(255, 0, 0, 255)]),
        start_key: 60,
        end_key: 60,
        key_mode: KeyMode::AllKeys,
        note_length_mode: NoteLengthMode::Unlimited,
        max_note_length: 0,
        target_height: 2,
        resize_algorithm: ResizeAlgorithm::NearestNeighbor,
        color_id_method: ColorIdMethod::Rgb,
        ticks_per_pixel: 1,
        ppq: 96,
        start_offset: 0,
        bpm: 120,
        emit_color_events: true,
        random_colors: false,
        random_color_seed: 0,
    }
}

fn write_test_png(path: &std::path::Path) {
    // 2x2 red image, RGBA
    let rgba: Vec<u8> = [255u8, 0, 0, 255]
        .iter()
        .cycle()
        .take(16)
        .copied()
        .collect();
    image::save_buffer(path, &rgba, 2, 2, image::ColorType::Rgba8).unwrap();
}

#[test]
fn full_pipeline_writes_midi() {
    let temp = std::env::temp_dir().join("i2m_end_to_end");
    let _ = std::fs::create_dir_all(&temp);
    let png = temp.join("red.png");
    let mid = temp.join("red.mid");

    write_test_png(&png);

    let image = i2m_rs::load_image(&png).unwrap();
    assert_eq!(image.width, 2);
    assert_eq!(image.height, 2);

    let (palette, _) = cluster::generate_palette(&image, &default_config().palette, 1).unwrap();
    assert_eq!(palette.colors.len(), 1);

    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = i2m_rs::convert(&image, &palette, &default_config(), None, &cancel).unwrap();
    assert_eq!(result.note_count, 1);

    let results: Vec<&i2m_rs::ConversionResult> = vec![&result];
    i2m_rs::write_midi(&mid, &results, &default_config()).unwrap();

    let data = std::fs::read(&mid).unwrap();
    assert!(!data.is_empty());

    let _ = std::fs::remove_dir_all(&temp);
}
