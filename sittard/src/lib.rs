pub use sittard_macros::test;

#[cfg(test)]
use sittard_macros::test_priv;

pub mod rt;
#[cfg(test)]
mod test_util;
pub mod time;

pub fn spawn<T: 'static>(f: impl Future<Output = T> + 'static) -> rt::JoinHandle<T> {
    let rt = rt::Rt::active();
    let (tx, rx) = futures::channel::oneshot::channel();
    rt.spawn(async move {
        tx.send(f.await).ok();
    });

    rt::JoinHandle { rx }
}
