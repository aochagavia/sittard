use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant as StdInstant};

#[derive(Clone)]
pub(crate) struct RuntimeClock {
    pub(super) now: Arc<Mutex<StdInstant>>,
}

impl RuntimeClock {
    pub(crate) fn now(&self) -> StdInstant {
        *self.now.lock()
    }

    pub(crate) fn set_now(&self, now: StdInstant) {
        *self.now.lock() = now;
    }
}

pub trait AdvanceClock {
    fn advance_clock(&self, now: StdInstant, next_timer_elapsed: StdInstant) -> StdInstant;
}

impl AdvanceClock for AdvanceToNextWake {
    fn advance_clock(&self, _now: StdInstant, next_timer_elapsed: StdInstant) -> StdInstant {
        next_timer_elapsed
    }
}

impl AdvanceClock for AdvanceToNextWakeWithResolution {
    fn advance_clock(&self, now: StdInstant, next_timer_elapsed: StdInstant) -> StdInstant {
        // Advance an exact multiple of `timer_granularity` since the last time jump
        let last_time_jump = self.last_time_jump_to.lock().unwrap_or(now);
        let diff = next_timer_elapsed - last_time_jump;
        let elapsed_intervals =
            (self.timer_granularity.as_secs_f64() / diff.as_secs_f64()).ceil() as u32;
        let new_time = now + self.timer_granularity * elapsed_intervals;
        *self.last_time_jump_to.lock() = Some(new_time);
        new_time
    }
}

pub struct AdvanceToNextWake;

pub struct AdvanceToNextWakeWithResolution {
    last_time_jump_to: Mutex<Option<std::time::Instant>>,
    timer_granularity: Duration,
}

impl AdvanceToNextWakeWithResolution {
    pub fn new(timer_granularity: Duration) -> Self {
        Self {
            last_time_jump_to: Mutex::default(),
            timer_granularity,
        }
    }
}
