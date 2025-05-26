use futures::FutureExt;
use futures::channel::oneshot::Canceled;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct WakeTask {
    pub(super) task_id: u64,
}

/// A handle to a spawned task that can be awaited to get the task's result.
///
/// # Example
///
/// ```rust
/// use sittard::{spawn, Runtime};
/// use std::time::Duration;
///
/// let rt = Runtime::default();
/// rt.block_on(async {
///     let handle = spawn(async {
///         sittard::time::sleep(Duration::from_secs(1)).await;
///         42
///     });
///
///     let result = handle.await.unwrap();
///     assert_eq!(result, 42);
/// });
/// ```
pub struct JoinHandle<T> {
    pub(crate) rx: futures::channel::oneshot::Receiver<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, Canceled>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.rx.poll_unpin(cx)
    }
}

pub(super) struct Task {
    pub(super) future: Pin<Box<dyn Future<Output = ()>>>,
}
