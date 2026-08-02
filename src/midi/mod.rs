//! MIDI event types.
//!
//! [`MidiEvent`](events::MidiEvent) models the small set of events the
//! converter emits (notes, tempo, color meta events); [`TimedMidiEvent`]
//! pairs one with an absolute tick. [`writer`] serializes whole
//! [`ConversionResult`](crate::convert::ConversionResult)s into a standard
//! MIDI file.

pub mod events;
pub mod writer;

/// A [`MidiEvent`](events::MidiEvent) stamped with an absolute tick.
///
/// During conversion the tick is measured in *pixel-ticks* (image rows,
/// counted from the bottom); [`writer::write_midi`] multiplies by
/// [`ConverterConfig::ticks_per_pixel`](crate::config::ConverterConfig::ticks_per_pixel)
/// when laying events out in real MIDI ticks.
#[derive(Clone, Debug)]
pub struct TimedMidiEvent {
    /// Absolute tick of the event (pixel-ticks before writing).
    pub tick: u64,
    /// The event payload.
    pub event: events::MidiEvent,
}

impl TimedMidiEvent {
    /// Pair `event` with its absolute `tick`.
    pub fn new(tick: u64, event: events::MidiEvent) -> Self {
        Self { tick, event }
    }
}
