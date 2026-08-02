//! The event vocabulary used between conversion and writing.

use crate::color::Color;

/// A single MIDI (or MIDI-adjacent) event.
///
/// `NoteOn`/`NoteOff` map directly onto MIDI messages; `Tempo` and `Color`
/// become meta events when serialized by
/// [`write_midi`](crate::midi::writer::write_midi).
///
/// Use the constructor helpers ([`MidiEvent::note_on`], …) instead of building
/// the variants by hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MidiEvent {
    /// Note on. The converter always uses channel 0 and velocity 1.
    NoteOn { channel: u8, key: u8, velocity: u8 },
    /// Note off. The converter always uses channel 0 and velocity 0.
    NoteOff { channel: u8, key: u8, velocity: u8 },
    /// Tempo meta event, `tempo` in microseconds per quarter note.
    Tempo { tempo: u32 },
    /// Track-color meta event, written as an `0x0A` unknown meta event whose
    /// payload is produced by
    /// [`color_event_payload`](crate::midi::writer::color_event_payload).
    Color { track: usize, color: Color },
}

impl MidiEvent {
    /// Create a [`NoteOn`](Self::NoteOn) event.
    pub fn note_on(channel: u8, key: u8, velocity: u8) -> Self {
        Self::NoteOn {
            channel,
            key,
            velocity,
        }
    }

    /// Create a [`NoteOff`](Self::NoteOff) event.
    pub fn note_off(channel: u8, key: u8, velocity: u8) -> Self {
        Self::NoteOff {
            channel,
            key,
            velocity,
        }
    }

    /// Create a [`Tempo`](Self::Tempo) meta event (`tempo` = µs per quarter).
    pub fn tempo(tempo: u32) -> Self {
        Self::Tempo { tempo }
    }

    /// Create a [`Color`](Self::Color) meta event for `track`.
    pub fn color(track: usize, color: Color) -> Self {
        Self::Color { track, color }
    }
}
