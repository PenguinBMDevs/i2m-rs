//! Progress reporting and cooperative cancellation.
//!
//! [`convert`](crate::convert) and [`convert_batch`](crate::convert_batch)
//! accept an optional `&dyn Progress` to report which [`Stage`] they are in
//! and how far along they are (`fraction` in `0.0..=1.0`). Cancellation is
//! cooperative: the caller flips a shared [`AtomicBool`] (or
//! [`CancellationToken`]) and the conversion returns
//! [`Error::Cancelled`](crate::Error::Cancelled) at the next checkpoint.
//!
//! # Examples
//!
//! ```
//! use i2m_rs::{Progress, Stage};
//!
//! // Any Send + Sync closure implements Progress automatically:
//! let progress = |stage: Stage, fraction: f64| {
//!     eprintln!("{stage:?}: {:.1}%", fraction * 100.0);
//! };
//!
//! fn check(p: &dyn Progress) { p.report(Stage::Loading, 0.0); }
//! check(&progress);
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

/// A stage of the conversion pipeline, reported through [`Progress`].
///
/// Stages occur roughly in declaration order, but [`convert`](crate::convert)
/// currently only reports [`GeneratingNotes`](Self::GeneratingNotes), and
/// [`convert_batch`](crate::convert_batch) additionally reports
/// [`WritingMidi`](Self::WritingMidi) after each item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Reading and decoding the input image.
    Loading,
    /// Clustering pixels to build the palette.
    Clustering,
    /// Resizing the image to the key range.
    Resizing,
    /// Mapping pixels onto palette entries.
    Quantizing,
    /// Turning color runs into note events.
    GeneratingNotes,
    /// Serializing tracks into the `.mid` file.
    WritingMidi,
}

/// Receiver for progress updates.
///
/// Implemented automatically for every `Fn(Stage, f64) + Send + Sync`, so a
/// plain closure is enough — see the module-level example. The trait is
/// object-safe and used as `Option<&dyn Progress>` throughout the crate.
pub trait Progress: Send + Sync {
    /// Called with the current stage and a completion fraction in
    /// `0.0..=1.0`. May be called from worker threads; implementations must
    /// not assume an ordering guarantee beyond monotonic stages per call site.
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

/// A cheap, thread-safe cancellation flag.
///
/// This is a convenience wrapper around [`AtomicBool`] for callers that prefer
/// a named type. Internally the conversion functions still take a plain
/// `&AtomicBool`, so a token can be shared as `&token.inner`-style or replaced
/// by your own flag.
///
/// # Examples
///
/// ```
/// use i2m_rs::CancellationToken;
///
/// let token = CancellationToken::new();
/// assert!(!token.is_cancelled());
/// token.cancel();
/// assert!(token.is_cancelled());
/// ```
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    /// Create a fresh, non-cancelled token.
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    /// Signal cancellation. Idempotent and safe to call from any thread.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Check whether [`cancel`](Self::cancel) has been called.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
