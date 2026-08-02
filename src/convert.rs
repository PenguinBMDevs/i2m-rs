//! The core image → note-events conversion.
//!
//! [`convert`] takes a decoded image plus a palette, resizes the image so its
//! width equals the number of usable MIDI keys, then scans it **bottom to top**
//! (like a piano roll rolling upwards): each column is one key, each row step
//! advances time by one pixel-tick, and every maximal vertical run of one
//! color becomes one note. Events are grouped per color into
//! [`ConversionResult::track_events`], ready for
//! [`write_midi`](crate::midi::writer::write_midi).

use crate::color::{Color, Palette, PaletteLabCache};
use crate::config::{ColorIdMethod, ConverterConfig, KeyMode, NoteLengthMode};
use crate::error::{Error, Result};
use crate::image::RgbaImage;
use crate::match_color::{TRANSPARENT_ID, match_pixel};
use crate::midi::TimedMidiEvent;
use crate::midi::events::MidiEvent;
use crate::progress::{Progress, Stage};
use crate::resize::resize;
use crate::utils::is_white_key;
use std::sync::atomic::{AtomicBool, Ordering};

/// The outcome of a single [`convert`] call.
///
/// # Examples
///
/// ```
/// use i2m_rs::{Color, ConversionResult, ConverterConfig, Palette, convert};
/// use i2m_rs::image::RgbaImage;
/// use i2m_rs::config::PaletteSource;
/// use std::sync::atomic::AtomicBool;
///
/// // A 1x2 solid red image, one-color palette, one key (C4):
/// let image = RgbaImage::new(1, 2, Color::new(255, 0, 0, 255));
/// let palette = Palette::new(vec![Color::new(255, 0, 0, 255)]);
/// let config = ConverterConfig {
///     color_count: 1,
///     palette: PaletteSource::Manual(palette.colors.clone()),
///     start_key: 60,
///     end_key: 60,
///     target_height: 2,
///     ..Default::default()
/// };
/// let cancel = AtomicBool::new(false);
/// let result = convert(&image, &palette, &config, None, &cancel).unwrap();
/// assert_eq!(result.note_count, 1); // one note, 2 ticks long
/// ```
#[derive(Clone, Debug, Default)]
pub struct ConversionResult {
    /// Total number of notes (note-on events) across all tracks.
    pub note_count: usize,
    /// `note_count_per_color[i]` = notes in track `i`.
    pub note_count_per_color: Vec<usize>,
    /// Timed events per color track; `track_events[i]` belongs to
    /// `palette[i]`. Event ticks are in *pixel-ticks* (rows); multiply by
    /// [`ConverterConfig::ticks_per_pixel`] for MIDI ticks.
    pub track_events: Vec<Vec<TimedMidiEvent>>,
    /// Height of the resized image in pixels — the total duration in
    /// pixel-ticks.
    pub height: u32,
    /// The palette used for this conversion (clone of the input).
    pub palette: Vec<Color>,
}

/// Convert one image into timed MIDI events.
///
/// Pipeline performed here:
///
/// 1. validate config and palette;
/// 2. build the key list with [`build_key_list`] and compute the target
///    width (one column per usable key);
/// 3. [`resize`] the image (height keeps aspect ratio unless
///    [`ConverterConfig::target_height`] is set);
/// 4. scan bottom-up: on each color change (or note-length limit hit, see
///    [`NoteLengthMode`]) close the ringing note with a note-off and open a
///    new one with a note-on (velocity 1) on the corresponding color track;
/// 5. close all remaining notes at the top edge.
///
/// `progress` (optional) reports [`Stage::GeneratingNotes`] with increasing
/// fractions. Set `cancel` to `true` from another thread to abort.
///
/// # Errors
///
/// * [`Error::Config`] — empty palette, `start_key > end_key`, a key range
///   that yields no usable keys, or a zero-width image when the height must
///   be derived from the aspect ratio.
/// * [`Error::Resize`] — the resized image ended up with zero dimensions.
/// * [`Error::Cancelled`] — `cancel` was set.
///
/// # Examples
///
/// See [`ConversionResult`].
pub fn convert(
    image: &RgbaImage,
    palette: &Palette,
    config: &ConverterConfig,
    progress: Option<&dyn Progress>,
    cancel: &AtomicBool,
) -> Result<ConversionResult> {
    if palette.colors.is_empty() {
        return Err(Error::Config("palette must not be empty".into()));
    }
    if config.start_key > config.end_key {
        return Err(Error::Config("start_key must not exceed end_key".into()));
    }

    let key_list = build_key_list(config.start_key, config.end_key, config.key_mode);
    if key_list.is_empty() {
        return Err(Error::Config("key range produces no usable keys".into()));
    }

    let effective_width = match config.key_mode {
        KeyMode::AllKeys
        | KeyMode::WhiteKeysClipped
        | KeyMode::BlackKeysClipped
        | KeyMode::WhiteKeysFixed
        | KeyMode::BlackKeysFixed => u32::from(config.end_key - config.start_key + 1),
        KeyMode::WhiteKeysFilled | KeyMode::BlackKeysFilled => key_list.len() as u32,
    };

    let target_height = if config.target_height == 0 {
        if image.width == 0 {
            return Err(Error::Config(
                "image width is zero, cannot derive height".into(),
            ));
        }
        (f64::from(image.height) * f64::from(effective_width) / f64::from(image.width)).round()
            as u32
    } else {
        config.target_height
    };

    let resized = resize(
        image,
        effective_width,
        target_height,
        config.resize_algorithm,
    )?;
    if resized.width == 0 || resized.height == 0 {
        return Err(Error::Resize("resized image has zero dimensions".into()));
    }

    report_progress(progress, Stage::GeneratingNotes, 0.0);

    let track_count = palette.colors.len();
    let mut tracks: Vec<Vec<TimedMidiEvent>> = vec![Vec::new(); track_count];
    let mut last_times: Vec<u64> = vec![0; track_count];
    let mut column_colors: Vec<i32> = vec![TRANSPARENT_ID; resized.width as usize];
    let mut last_on_times: Vec<i64> = vec![0; resized.width as usize];

    let use_max_length = config.max_note_length > 0;
    if use_max_length && config.note_length_mode == NoteLengthMode::FlowWithColor {
        let initial = -(i64::from(config.max_note_length) + 1);
        last_on_times.fill(initial);
    }

    let cache = build_lab_cache(palette, config.color_id_method);
    let height = resized.height;
    let width = resized.width;

    let mut time: u64 = 0;

    for row in (0..height).rev() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }

        for column in 0..width {
            let key_result = resolve_key(column as u8, &key_list, config.key_mode);
            let Some(midi_key) = key_result else {
                column_colors[column as usize] = TRANSPARENT_ID;
                continue;
            };

            let color = resized.get(column, row);
            let mut new_color = match_pixel(color, palette, config.color_id_method, cache.as_ref());
            if new_color < 0 || new_color >= track_count as i32 {
                new_color = TRANSPARENT_ID;
            }

            let old_color = column_colors[column as usize];
            let color_changed = new_color != old_color;
            let new_note = should_start_new_note(
                time,
                height,
                row,
                column as usize,
                use_max_length,
                config,
                &last_on_times,
            );

            if color_changed || new_note {
                if old_color >= 0 && (old_color as usize) < track_count {
                    let track = old_color as usize;
                    tracks[track].push(TimedMidiEvent::new(
                        time,
                        MidiEvent::note_off(0, midi_key, 0),
                    ));
                    last_times[track] = time;
                }

                if new_color >= 0 && (new_color as usize) < track_count {
                    let track = new_color as usize;
                    tracks[track].push(TimedMidiEvent::new(
                        time,
                        MidiEvent::note_on(0, midi_key, 1),
                    ));
                    last_times[track] = time;
                }

                column_colors[column as usize] = new_color;
                last_on_times[column as usize] = time as i64;
            }
        }

        time += 1;

        if row % 32 == 0 {
            let fraction = 1.0 - f64::from(row) / f64::from(height);
            report_progress(progress, Stage::GeneratingNotes, fraction.clamp(0.0, 1.0));
        }
    }

    emit_final_note_offs(
        &mut tracks,
        &mut last_times,
        &column_colors,
        &key_list,
        config,
        width,
        time,
    );

    report_progress(progress, Stage::GeneratingNotes, 1.0);

    let mut note_count = 0;
    let mut note_count_per_color = vec![0; track_count];
    for (track, events) in tracks.iter().enumerate() {
        let count = events
            .iter()
            .filter(|e| matches!(e.event, MidiEvent::NoteOn { .. }))
            .count();
        note_count_per_color[track] = count;
        note_count += count;
    }

    Ok(ConversionResult {
        note_count,
        note_count_per_color,
        track_events: tracks,
        height: resized.height,
        palette: palette.colors.clone(),
    })
}

/// Build the ordered list of MIDI keys used for the given range and mode.
///
/// * [`KeyMode::AllKeys`] and the `Clipped`/`Fixed` modes keep every key in
///   `start_key..=end_key`.
/// * [`KeyMode::WhiteKeysFilled`] / [`KeyMode::BlackKeysFilled`] filter to
///   only white / only black keys, so the image width shrinks to the count
///   of those keys.
///
/// # Examples
///
/// ```
/// use i2m_rs::{KeyMode, convert::build_key_list};
///
/// assert_eq!(build_key_list(60, 62, KeyMode::AllKeys), vec![60, 61, 62]);
/// // 60=C, 61=C#, 62=D, 63=D# — only the white keys survive:
/// assert_eq!(build_key_list(60, 63, KeyMode::WhiteKeysFilled), vec![60, 62]);
/// ```
pub fn build_key_list(start_key: u8, end_key: u8, mode: KeyMode) -> Vec<u8> {
    let mut keys = Vec::new();
    match mode {
        KeyMode::AllKeys
        | KeyMode::WhiteKeysClipped
        | KeyMode::BlackKeysClipped
        | KeyMode::WhiteKeysFixed
        | KeyMode::BlackKeysFixed => {
            for key in start_key..=end_key {
                keys.push(key);
            }
        }
        KeyMode::WhiteKeysFilled => {
            for key in start_key..=end_key {
                if is_white_key(key) {
                    keys.push(key);
                }
            }
        }
        KeyMode::BlackKeysFilled => {
            for key in start_key..=end_key {
                if !is_white_key(key) {
                    keys.push(key);
                }
            }
        }
    }
    keys
}

fn build_lab_cache(palette: &Palette, method: ColorIdMethod) -> Option<PaletteLabCache> {
    if matches!(method, ColorIdMethod::Lab | ColorIdMethod::Ciede2000) {
        Some(PaletteLabCache::new(palette))
    } else {
        None
    }
}

fn resolve_key(column: u8, key_list: &[u8], mode: KeyMode) -> Option<u8> {
    let column = column as usize;
    let midi_key = match mode {
        KeyMode::AllKeys | KeyMode::WhiteKeysClipped | KeyMode::BlackKeysClipped => {
            *key_list.get(column)?
        }
        KeyMode::WhiteKeysFilled | KeyMode::BlackKeysFilled => *key_list.get(column)?,
        KeyMode::WhiteKeysFixed | KeyMode::BlackKeysFixed => key_list.first()? + column as u8,
    };

    match mode {
        KeyMode::WhiteKeysFixed if !is_white_key(midi_key) => None,
        KeyMode::BlackKeysFixed if is_white_key(midi_key) => None,
        _ => Some(midi_key),
    }
}

fn should_start_new_note(
    time: u64,
    height: u32,
    row: u32,
    column: usize,
    use_max_length: bool,
    config: &ConverterConfig,
    last_on_times: &[i64],
) -> bool {
    if !use_max_length {
        return false;
    }

    match config.note_length_mode {
        NoteLengthMode::Unlimited => false,
        NoteLengthMode::SplitToGrid => {
            let row_from_bottom = height - 1 - row;
            row_from_bottom > 0 && row_from_bottom.is_multiple_of(config.max_note_length)
        }
        NoteLengthMode::FlowWithColor => {
            let time_since_last_on =
                i64::try_from(time).unwrap_or(i64::MAX) - last_on_times[column];
            time_since_last_on >= i64::from(config.max_note_length)
        }
    }
}

fn emit_final_note_offs(
    tracks: &mut [Vec<TimedMidiEvent>],
    last_times: &mut [u64],
    column_colors: &[i32],
    key_list: &[u8],
    config: &ConverterConfig,
    width: u32,
    time: u64,
) {
    let track_count = tracks.len();

    for column in 0..width {
        let Some(midi_key) = resolve_key(column as u8, key_list, config.key_mode) else {
            continue;
        };

        let color = column_colors[column as usize];
        if color >= 0 && (color as usize) < track_count {
            let track = color as usize;
            tracks[track].push(TimedMidiEvent::new(
                time,
                MidiEvent::note_off(0, midi_key, 0),
            ));
            last_times[track] = time;
        }
    }
}

fn report_progress(progress: Option<&dyn Progress>, stage: Stage, fraction: f64) {
    if let Some(p) = progress {
        p.report(stage, fraction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Color, Palette};
    use crate::config::{ConverterConfig, KeyMode, NoteLengthMode, ResizeAlgorithm};
    use crate::image::RgbaImage;

    fn default_config() -> ConverterConfig {
        ConverterConfig {
            color_count: 1,
            palette: crate::config::PaletteSource::Manual(vec![Color::new(255, 0, 0, 255)]),
            start_key: 60,
            end_key: 60,
            key_mode: KeyMode::AllKeys,
            note_length_mode: NoteLengthMode::Unlimited,
            max_note_length: 0,
            target_height: 2,
            resize_algorithm: ResizeAlgorithm::NearestNeighbor,
            color_id_method: crate::config::ColorIdMethod::Rgb,
            ticks_per_pixel: 1,
            ppq: 96,
            start_offset: 0,
            bpm: 120,
            emit_color_events: false,
            random_colors: false,
            random_color_seed: 0,
        }
    }

    #[test]
    fn build_key_list_all_keys() {
        let keys = build_key_list(60, 62, KeyMode::AllKeys);
        assert_eq!(keys, vec![60, 61, 62]);
    }

    #[test]
    fn build_key_list_white_filled() {
        let keys = build_key_list(60, 63, KeyMode::WhiteKeysFilled);
        assert_eq!(keys, vec![60, 62]);
    }

    #[test]
    fn single_column_produces_one_note() {
        let mut image = RgbaImage::new(1, 2, Color::BLACK);
        image.set(0, 0, Color::new(255, 0, 0, 255));
        image.set(0, 1, Color::new(255, 0, 0, 255));

        let palette = Palette::new(vec![Color::new(255, 0, 0, 255)]);
        let cancel = AtomicBool::new(false);
        let result = convert(&image, &palette, &default_config(), None, &cancel).unwrap();

        assert_eq!(result.note_count, 1);
        assert_eq!(result.track_events.len(), 1);
        assert_eq!(result.track_events[0].len(), 2); // NoteOn + NoteOff
    }
}
