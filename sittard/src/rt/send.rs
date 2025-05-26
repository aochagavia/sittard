use crate::{AdvanceToNextWake, Runtime};
use send_wrapper::SendWrapper;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

/// A wrapper around [`Runtime`] that implements `Send`.
///
/// `SendRuntime` allows a sittard runtime to be sent between threads safely. However, the runtime
/// may only be accessed from the thread that created it, __or it will panic__.
///
/// This type is provided because some crates impose `Send` bounds on runtimes, even though they are
/// not actually using the runtime from multiple threads.
///
/// # Example
///
/// ```rust
/// use sittard::SendRuntime;
/// use std::time::Duration;
///
/// let rt = SendRuntime::default();
/// rt.block_on(async {
///     sittard::time::sleep(Duration::from_secs(1)).await;
///     println!("Task completed!");
/// });
/// ```
pub struct SendRuntime {
    inner: SendWrapper<Runtime>,
}

impl SendRuntime {
    /// Creates a new `SendRuntime` from an existing `Runtime`
    pub fn from_runtime(runtime: Runtime) -> Self {
        Self {
            inner: SendWrapper::new(runtime),
        }
    }
}

impl Deref for SendRuntime {
    type Target = Runtime;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for SendRuntime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SendRuntime")
    }
}

impl Default for SendRuntime {
    fn default() -> Self {
        Self::from_runtime(Runtime::new(Box::new(AdvanceToNextWake)))
    }
}
