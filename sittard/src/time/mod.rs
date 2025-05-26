//! Time-related primitives that work with sittard's virtual clock

use crate::rt::Runtime;
pub use crate::time::instant::Instant;
pub use crate::time::timer::Timer;
use futures::{FutureExt, select_biased};
use std::pin::pin;
use std::time::Duration;

mod instant;
pub(crate) mod timer;

/// Creates a timer that completes after the specified duration, relative to the current virtual time
///
/// # Example
///
/// ```rust
/// use sittard::{time::sleep, Runtime};
/// use std::time::Duration;
///
/// let rt = Runtime::default();
/// rt.block_on(async {
///     sleep(Duration::from_secs(2)).await;
///     println!("2 seconds have passed!");
/// });
/// ```
#[must_use]
pub fn sleep(duration: Duration) -> Timer {
    Runtime::active().sleep(duration)
}

/// Creates a timer that completes at the specified absolute virtual time
///
/// # Example
///
/// ```rust
/// use sittard::{time::{sleep_until, Instant}, Runtime};
/// use std::time::Duration;
///
/// let rt = Runtime::default();
/// rt.block_on(async {
///     let deadline = Instant::now() + Duration::from_secs(5);
///     sleep_until(deadline).await;
///     println!("Deadline reached!");
/// });
/// ```
#[must_use]
pub fn sleep_until(instant: Instant) -> Timer {
    Runtime::active().sleep_until(instant)
}

/// Applies a timeout to a future.
///
/// This function runs the provided future with a time limit. If the future completes before the
/// timeout duration elapses, its result is returned wrapped in `Ok`. If the timeout elapses first,
/// the function returns `Err(())`.
///
/// # Parameters
///
/// * `duration` - The maximum time to wait for the future to complete
/// * `future` - The future to run with a timeout
///
/// # Example
///
/// ```rust
/// use sittard::{time::{timeout, sleep}, Runtime};
/// use std::time::Duration;
///
/// let rt = Runtime::default();
/// rt.block_on(async {
///     // This will timeout
///     let result = timeout(Duration::from_secs(1), async {
///         sleep(Duration::from_secs(2)).await;
///         42
///     }).await;
///     assert!(result.is_err());
///
///     // This will complete in time
///     let result = timeout(Duration::from_secs(2), async {
///         sleep(Duration::from_secs(1)).await;
///         42
///     }).await;
///     assert_eq!(result.unwrap(), 42);
/// });
/// ```
pub async fn timeout<T>(duration: Duration, future: impl Future<Output = T>) -> Result<T, ()> {
    let timer = sleep(duration);
    let future = pin!(future);
    select_biased! {
        _ = timer.fuse() => { Err(()) },
        output = future.fuse() => { Ok(output) }
    }
}
