pub use sittard_macros::test;

#[cfg(test)]
use sittard_macros::test_priv;

mod rt;
#[cfg(test)]
mod test_util;
pub mod time;

pub use rt::{Runtime, JoinHandle, AdvanceClock, AdvanceToNextWake, AdvanceToNextWakeWithResolution};

pub fn spawn<T: 'static>(f: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    let rt = Runtime::active();
    let (tx, rx) = futures::channel::oneshot::channel();
    rt.spawn(async move {
        tx.send(f.await).ok();
    });

    JoinHandle { rx }
}
