use crate::rt::Runtime;
use std::ops::{Add, Sub};
use std::time::Duration;

/// An instant in sittard's virtual time.
///
/// # Example
///
/// ```rust
/// use sittard::{time::Instant, Runtime};
/// use std::time::Duration;
///
/// let rt = Runtime::default();
/// rt.block_on(async {
///     let start = Instant::now();
///     sittard::time::sleep(Duration::from_secs(1)).await;
///     assert_eq!(start.elapsed(), Duration::from_secs(1));
/// });
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Instant(pub(crate) std::time::Instant);

impl Instant {
    /// Returns the current virtual time.
    ///
    /// # Panics
    ///
    /// Panics if called outside of a sittard runtime context (i.e. not within `Runtime::block_on`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use sittard::{time::Instant, Runtime};
    ///
    /// let rt = Runtime::default();
    /// rt.block_on(async {
    ///     let now = Instant::now();
    ///     // Use the instant...
    /// });
    /// ```
    pub fn now() -> Self {
        let now = Runtime::active().now();
        Self(now)
    }

    /// Returns the amount of time elapsed since this instant.
    ///
    /// This function calculates the duration between this instant and the
    /// current virtual time.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sittard::{time::Instant, Runtime};
    /// use std::time::Duration;
    ///
    /// let rt = Runtime::default();
    /// rt.block_on(async {
    ///     let start = Instant::now();
    ///     sittard::time::sleep(Duration::from_millis(500)).await;
    ///     assert_eq!(start.elapsed(), Duration::from_millis(500));
    /// });
    /// ```
    pub fn elapsed(&self) -> Duration {
        Self::now().0 - self.0
    }

    /// Creates a sittard `Instant` from a `std::time::Instant`
    pub fn from_std(instant: std::time::Instant) -> Self {
        Self(instant)
    }

    /// Converts this sittard `Instant` to a standard library `Instant`
    pub fn into_std(self) -> std::time::Instant {
        self.0
    }
}

impl Sub for Instant {
    type Output = Duration;

    fn sub(self, other: Self) -> Self::Output {
        self.0 - other.0
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl<'b> Sub<&'b Instant> for &Instant {
    type Output = Duration;

    fn sub(self, other: &'b Instant) -> Self::Output {
        self.0 - other.0
    }
}

impl Add<Duration> for &Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        Instant(self.0 + rhs)
    }
}
