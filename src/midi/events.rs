use crate::color::Color;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MidiEvent {
    NoteOn { channel: u8, key: u8, velocity: u8 },
    NoteOff { channel: u8, key: u8, velocity: u8 },
    Tempo { tempo: u32 },
    Color { track: usize, color: Color },
}

impl MidiEvent {
    pub fn note_on(channel: u8, key: u8, velocity: u8) -> Self {
        Self::NoteOn {
            channel,
            key,
            velocity,
        }
    }

    pub fn note_off(channel: u8, key: u8, velocity: u8) -> Self {
        Self::NoteOff {
            channel,
            key,
            velocity,
        }
    }

    pub fn tempo(tempo: u32) -> Self {
        Self::Tempo { tempo }
    }

    pub fn color(track: usize, color: Color) -> Self {
        Self::Color { track, color }
    }
}
