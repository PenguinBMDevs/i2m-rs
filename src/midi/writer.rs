use crate::color::Color;
use crate::config::ConverterConfig;
use crate::convert::ConversionResult;
use crate::error::{Error, Result};
use crate::midi::events::MidiEvent;
use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use std::path::Path;

struct PendingEvent {
    tick: u64,
    event: MidiEvent,
    payload_index: Option<usize>,
}

pub fn write_midi(
    path: &Path,
    results: &[&ConversionResult],
    config: &ConverterConfig,
) -> Result<()> {
    if results.is_empty() {
        return Err(Error::Midi("no conversion results to write".into()));
    }

    let track_count = results[0].track_events.len();
    if track_count == 0 {
        return Err(Error::Midi("no tracks in conversion result".into()));
    }

    let ticks_per_pixel = u64::from(config.ticks_per_pixel.max(1));
    let start_offset = u64::from(config.start_offset);

    let payloads: Vec<Vec<Vec<u8>>> = results[0]
        .palette
        .iter()
        .enumerate()
        .map(|(track, color)| {
            if config.emit_color_events {
                vec![color_event_payload(track, *color)]
            } else {
                Vec::new()
            }
        })
        .collect();

    let mut tracks: Vec<Vec<TrackEvent>> = Vec::with_capacity(track_count);

    for (track, payload_track) in payloads.iter().enumerate() {
        let color_payload_index = if config.emit_color_events && !payload_track.is_empty() {
            Some(0)
        } else {
            None
        };
        let mut pending: Vec<PendingEvent> = Vec::new();

        if config.emit_color_events
            && let Some(color) = results[0].palette.get(track)
        {
            pending.push(PendingEvent {
                tick: start_offset,
                event: MidiEvent::Color {
                    track,
                    color: *color,
                },
                payload_index: color_payload_index,
            });
        }

        let mut global_tick = start_offset;
        for result in results {
            for timed in &result.track_events[track] {
                let abs_tick = timed
                    .tick
                    .saturating_mul(ticks_per_pixel)
                    .saturating_add(global_tick);
                pending.push(PendingEvent {
                    tick: abs_tick,
                    event: timed.event.clone(),
                    payload_index: None,
                });
            }
            global_tick = global_tick
                .saturating_add(u64::from(result.height).saturating_mul(ticks_per_pixel));
        }

        pending.sort_by_key(|item| item.tick);

        let mut track_events: Vec<TrackEvent> = Vec::with_capacity(pending.len() + 2);
        let mut last_tick: u64 = 0;

        for item in pending {
            let delta = item.tick - last_tick;
            let kind = track_event_kind(&item.event, track, &payloads, item.payload_index)?;
            track_events.push(TrackEvent {
                delta: u28::new(delta as u32),
                kind,
            });
            last_tick = item.tick;
        }

        track_events.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        tracks.push(track_events);
    }

    insert_tempo_event(&mut tracks, config.bpm);

    let header = Header {
        format: Format::Parallel,
        timing: Timing::Metrical(u15::new(config.ppq)),
    };

    let smf = Smf { header, tracks };
    smf.save(path).map_err(|e| Error::Midi(e.to_string()))?;

    Ok(())
}

pub fn color_event_payload(track: usize, color: Color) -> Vec<u8> {
    vec![
        0x00,
        0x0F,
        u8::try_from(track % 16).unwrap_or(0),
        0x00,
        color.r,
        color.g,
        color.b,
        color.a,
    ]
}

fn track_event_kind<'a>(
    event: &MidiEvent,
    track: usize,
    payloads: &'a [Vec<Vec<u8>>],
    payload_index: Option<usize>,
) -> Result<TrackEventKind<'a>> {
    match event {
        MidiEvent::NoteOn {
            channel,
            key,
            velocity,
        } => Ok(TrackEventKind::Midi {
            channel: u4::new(*channel),
            message: MidiMessage::NoteOn {
                key: u7::new(*key),
                vel: u7::new(*velocity),
            },
        }),
        MidiEvent::NoteOff {
            channel,
            key,
            velocity,
        } => Ok(TrackEventKind::Midi {
            channel: u4::new(*channel),
            message: MidiMessage::NoteOff {
                key: u7::new(*key),
                vel: u7::new(*velocity),
            },
        }),
        MidiEvent::Color { .. } => {
            let index =
                payload_index.ok_or_else(|| Error::Midi("missing color payload index".into()))?;
            let payload = payloads
                .get(track)
                .and_then(|v| v.get(index))
                .ok_or_else(|| Error::Midi("missing color event payload".into()))?;
            Ok(TrackEventKind::Meta(MetaMessage::Unknown(0x0A, payload)))
        }
        MidiEvent::Tempo { tempo } => {
            Ok(TrackEventKind::Meta(MetaMessage::Tempo(u24::new(*tempo))))
        }
    }
}

fn insert_tempo_event(tracks: &mut [Vec<TrackEvent>], bpm: u16) {
    let tempo = 60_000_000 / u32::from(bpm.max(1));
    tracks[0].insert(
        0,
        TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(tempo))),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::TimedMidiEvent;

    fn simple_result() -> ConversionResult {
        ConversionResult {
            note_count: 1,
            note_count_per_color: vec![1],
            track_events: vec![vec![
                TimedMidiEvent::new(0, MidiEvent::note_on(0, 60, 1)),
                TimedMidiEvent::new(4, MidiEvent::note_off(0, 60, 0)),
            ]],
            height: 4,
            palette: vec![Color::new(255, 0, 0, 255)],
        }
    }

    #[test]
    fn color_event_payload_has_expected_prefix() {
        let payload = color_event_payload(0, Color::new(255, 0, 0, 255));
        assert_eq!(&payload[0..4], &[0x00, 0x0F, 0x00, 0x00]);
        assert_eq!(&payload[4..], &[255, 0, 0, 255]);
    }

    #[test]
    fn write_midi_creates_valid_file() {
        let temp = std::env::temp_dir().join("i2m_test.mid");
        let result = simple_result();
        let config = ConverterConfig {
            color_count: 1,
            palette: crate::config::PaletteSource::Manual(vec![Color::new(255, 0, 0, 255)]),
            start_key: 60,
            end_key: 60,
            key_mode: crate::config::KeyMode::AllKeys,
            note_length_mode: crate::config::NoteLengthMode::Unlimited,
            max_note_length: 0,
            target_height: 4,
            resize_algorithm: crate::config::ResizeAlgorithm::NearestNeighbor,
            color_id_method: crate::config::ColorIdMethod::Rgb,
            ticks_per_pixel: 1,
            ppq: 96,
            start_offset: 0,
            bpm: 120,
            emit_color_events: true,
            random_colors: false,
            random_color_seed: 0,
        };

        write_midi(&temp, &[&result], &config).unwrap();

        let data = std::fs::read(&temp).unwrap();
        assert!(!data.is_empty());

        let smf = Smf::parse(&data).unwrap();
        assert_eq!(smf.tracks.len(), 1);

        // Clean up
        let _ = std::fs::remove_file(&temp);
    }
}
