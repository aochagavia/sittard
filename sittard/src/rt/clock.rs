use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct RuntimeClock {
    pub(super) now: Arc<Mutex<Instant>>,
}

impl RuntimeClock {
    pub(crate) fn now(&self) -> Instant {
        *self.now.lock()
    }

    pub(crate) fn set_now(&self, now: Instant) {
        *self.now.lock() = now;
    }
}

/// Controls how sittard's virtual clock advances
pub trait AdvanceClock {
    /// Advances the virtual clock from the current time to at least the next timer deadline.
    ///
    /// This method gets called when the runtime is unable to make any progress because all tasks
    /// are blocked waiting for timers (or for other tasks to complete). Implementors should return
    /// a new `Instant` that is at least `next_timer_elapsed`.
    ///
    /// # Parameters
    ///
    /// * `now` - The current virtual time
    /// * `next_timer_elapsed` - The time at which the next known timer elapses
    ///
    /// # Returns
    ///
    /// The new virtual time, which must be >= `next_timer_elapsed` (otherwise the runtime will
    /// panic)
    fn advance_clock(&self, now: Instant, next_timer_elapsed: Instant) -> Instant;
}

impl AdvanceClock for AdvanceToNextWake {
    fn advance_clock(&self, _now: Instant, next_timer_elapsed: Instant) -> Instant {
        next_timer_elapsed
    }
}

impl AdvanceClock for AdvanceToNextWakeWithGranularity {
    fn advance_clock(&self, now: Instant, next_timer_elapsed: Instant) -> Instant {
        // Advance an exact multiple of `timer_granularity` since the last time jump
        let last_time_jump = self.last_time_jump_to.lock().unwrap_or(now);
        let diff = next_timer_elapsed - last_time_jump;
        let elapsed_intervals =
            (self.clock_granularity.as_secs_f64() / diff.as_secs_f64()).ceil() as u32;
        let new_time = now + self.clock_granularity * elapsed_intervals;
        *self.last_time_jump_to.lock() = Some(new_time);
        new_time
    }
}

/// An implementation of `AdvanceClock` that advances time to exactly the next timer deadline.
///
/// # Example
///
/// ```rust
/// use sittard::{Runtime, AdvanceToNextWake};
/// let rt = Runtime::new(Box::new(AdvanceToNextWake));
/// ```
pub struct AdvanceToNextWake;

/// An implementation of `AdvanceClock` that advances time according to a user-specified clock
/// granularity.
///
/// Instead of jumping to the next timer deadline, the clock will advance in multiples of
/// `clock_granularity` (see the `clock_granularity` parameter in the constructor). As with all
/// `AdvanceClock` implementations, time will advance at least until the next timer's deadline is
/// reached. Note that, since the jump must be a multiple of the granularity unit, the timer might
/// advance beyond the deadline.
///
/// # Example
///
/// ```rust
/// use sittard::{Runtime, AdvanceToNextWakeWithGranularity};
/// use std::time::Duration;
///
/// // Advance time in 10-millisecond increments
/// let clock_strategy = AdvanceToNextWakeWithGranularity::new(Duration::from_millis(10));
/// let rt = Runtime::new(Box::new(clock_strategy));
/// ```
pub struct AdvanceToNextWakeWithGranularity {
    last_time_jump_to: Mutex<Option<Instant>>,
    clock_granularity: Duration,
}

impl AdvanceToNextWakeWithGranularity {
    /// Creates a new instance of `AdvanceToNextWakeWithGranularity`.
    ///
    /// See the struct's documentation for additional details.
    pub fn new(clock_granularity: Duration) -> Self {
        Self {
            last_time_jump_to: Mutex::default(),
            clock_granularity,
        }
    }
}
