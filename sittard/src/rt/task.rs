use futures::FutureExt;
use futures::channel::oneshot::Canceled;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct WakeTask {
    pub(super) task_id: u64,
}

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
