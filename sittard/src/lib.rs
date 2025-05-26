pub use sittard_macros::test;

#[cfg(test)]
use sittard_macros::test_priv;

mod rt;
#[cfg(test)]
mod test_util;
pub mod time;

pub use rt::Runtime;
pub use rt::clock::AdvanceClock;
pub use rt::clock::AdvanceToNextWake;
pub use rt::clock::AdvanceToNextWakeWithResolution;
pub use rt::task::JoinHandle;

#[cfg(feature = "send")]
pub use rt::send::SendRuntime;

pub fn spawn<T: 'static>(f: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    let rt = Runtime::active();
    let (tx, rx) = futures::channel::oneshot::channel();
    rt.spawn(async move {
        tx.send(f.await).ok();
    });

    JoinHandle { rx }
}
