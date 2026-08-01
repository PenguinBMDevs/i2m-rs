pub mod events;
pub mod writer;

#[derive(Clone, Debug)]
pub struct TimedMidiEvent {
    pub tick: u64,
    pub event: events::MidiEvent,
}

impl TimedMidiEvent {
    pub fn new(tick: u64, event: events::MidiEvent) -> Self {
        Self { tick, event }
    }
}
