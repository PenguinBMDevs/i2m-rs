use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Loading,
    Clustering,
    Resizing,
    Quantizing,
    GeneratingNotes,
    WritingMidi,
}

pub trait Progress: Send + Sync {
    fn report(&self, stage: Stage, fraction: f64);
}

impl<F> Progress for F
where
    F: Fn(Stage, f64) + Send + Sync,
{
    fn report(&self, stage: Stage, fraction: f64) {
        (self)(stage, fraction)
    }
}

pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
