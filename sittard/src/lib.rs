//! Sittard - A **S**ans-**I**O **t**ickless **a**sync **r**untime, fully **d**eterministic.
//!
//! That's a mouthful, so let's unpack it:
//! - Async runtime: sittard runs async Rust code, i.e. stuff that implements the `Future` trait.
//! - Sans-IO: sittard doesn't support asynchronous IO (e.g. network requests, filesystem operations,
//!   etc).
//! - Tickless: sittard allows async code to "sleep", but instead of waiting for the time to elapse,
//!   sittard advances its virtual clock whenever necessary.
//! - Fully deterministic: running the same code under sittard always yields the same results, unless
//!   the async code itself is a source of non-determinism.
//!
//! # Example
//!
//! ```rust
//! use sittard::Runtime;
//! use std::time::Duration;
//!
//! // Create a runtime and run a future
//! let rt = Runtime::default();
//! rt.block_on(async move {
//!     let now = sittard::time::Instant::now();
//!     sittard::time::sleep(Duration::from_secs(60)).await;
//!     let elapsed_secs = now.elapsed().as_secs_f64();
//!     println!("Here we are, {elapsed_secs} seconds later...");
//! });
//! ```
//!
//! The code above completes instantly, even though it "sleeps" for 60 seconds.
//!
//! # Use Cases
//!
//! Sittard is particularly useful for:
//! - Creating reproducible simulations that depend on timing, e.g. QUIC network traffic with deep-space delays
//! - Testing time-dependent code without waiting for real time to pass
//!
//! Note that sittard is unsuitable for common async scenarios like web servers and clients.

#![deny(missing_docs)]

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
pub use rt::clock::AdvanceToNextWakeWithGranularity;
pub use rt::task::JoinHandle;

#[cfg(feature = "send")]
pub use rt::send::SendRuntime;

/// Spawns a new task on the currently active runtime.
///
/// The spawned task will run concurrently, but not in parallel, with other tasks. Returns a
/// `JoinHandle` that can be awaited to get the result of the task.
///
/// # Panics
///
/// Panics if called outside of a sittard runtime context (i.e. not within `Runtime::block_on`).
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
pub fn spawn<T: 'static>(f: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    let rt = Runtime::active();
    let (tx, rx) = futures::channel::oneshot::channel();
    rt.spawn(async move {
        tx.send(f.await).ok();
    });

    JoinHandle { rx }
}

#[cfg(test)]
mod test {
    use crate::rt::waker::TaskWaker;
    use crate::test_util::assert_send;
    use crate::time::Instant;
    use crate::time::{sleep, timeout};
    use crate::{Runtime, spawn};
    use futures::{FutureExt, SinkExt, StreamExt};
    use parking_lot::Mutex;
    use sittard_macros::test_priv;
    use std::collections::VecDeque;
    use std::future::{self, Future};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::task::{Context, Poll};
    use std::time::Duration;

    #[test]
    fn test_waker_is_send() {
        assert_send::<TaskWaker>();
    }

    #[test]
    fn test_waiting_timers_ordered_correctly() {
        let rt = Runtime::default();
        rt.block_on(async {
            rt.spawn(Box::pin(async move {
                sleep(Duration::from_secs(5)).await;
            }));

            let now = rt.now();
            rt.sleep(Duration::from_secs(10)).await;
            let later = rt.now();
            assert_eq!(Duration::from_secs(10), later - now);
        });
    }

    #[test_priv]
    async fn test_timeout_elapsed() {
        let start = Instant::now();
        let result = timeout(Duration::from_secs(10), async move {
            sleep(Duration::from_secs(15)).await
        })
        .await;

        assert!(result.is_err());
        assert_eq!(start.elapsed(), Duration::from_secs(10));
    }

    #[test_priv]
    async fn test_timeout_not_elapsed() {
        let start = Instant::now();
        let result = timeout(Duration::from_secs(10), async move {
            sleep(Duration::from_secs(5)).await
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(start.elapsed(), Duration::from_secs(5));
    }

    #[test_priv]
    async fn test_channels() {
        let (mut tx, mut rx) = futures::channel::mpsc::channel(42);
        spawn(async move {
            tx.send(1234).await.unwrap();
        });

        let received = rx.next().await.unwrap();
        assert_eq!(received, 1234);
    }

    #[test]
    fn test_select_timer() {
        let rt = Runtime::default();
        rt.block_on(async {
            for _ in 0..100 {
                let result;
                futures::select_biased! {
                    _ = sleep(Duration::from_secs(4)).fuse() => result = Some(42),
                    _ = sleep(Duration::from_secs(1)).fuse() => result = Some(1234),
                }

                assert_eq!(result, Some(1234));
            }
        });
    }

    #[test]
    fn test_select_task() {
        let rt = Runtime::default();
        rt.block_on(async {
            for _ in 0..100 {
                let t1 = async {
                    sleep(Duration::from_secs(4)).await;
                };
                let t2 = async {
                    sleep(Duration::from_secs(1)).await;
                };

                let result;
                futures::select_biased! {
                    _ = t1.fuse() => result = Some(42),
                    _ = t2.fuse() => result = Some(1234),
                }

                assert_eq!(result, Some(1234));
            }
        });
    }

    #[test]
    #[should_panic]
    fn test_block_on_stuck_panics() {
        let rt = Runtime::default();
        rt.block_on(async {
            // A future that never completes and has no timers
            future::pending::<()>().await;
        });
    }

    #[test]
    fn test_multiple_spawned_tasks_timer_order() {
        let rt = Runtime::default();
        let completion_order = Arc::new(Mutex::new(VecDeque::new()));

        rt.block_on(async {
            let order_clone = completion_order.clone();
            rt.spawn(Box::pin(async move {
                sleep(Duration::from_millis(200)).await;
                order_clone.lock().push_back(2);
            }));

            let order_clone = completion_order.clone();
            rt.spawn(Box::pin(async move {
                sleep(Duration::from_millis(100)).await;
                order_clone.lock().push_back(1);
            }));

            let order_clone = completion_order.clone();
            rt.spawn(Box::pin(async move {
                sleep(Duration::from_millis(300)).await;
                order_clone.lock().push_back(3);
            }));

            // Wait for all tasks to complete
            sleep(Duration::from_secs(1)).await;
        });

        let guard = completion_order.lock();
        assert_eq!(guard.len(), 3, "Not all tasks completed");
        assert_eq!(guard[0], 1);
        assert_eq!(guard[1], 2);
        assert_eq!(guard[2], 3);
    }

    #[test]
    fn test_nested_spawn() {
        let rt = Runtime::default();
        let inner_task_completed = Arc::new(AtomicBool::new(false));

        rt.block_on(async {
            let flag_clone = inner_task_completed.clone();
            spawn(Box::pin(async move {
                sleep(Duration::from_millis(50)).await;
                spawn(Box::pin(async move {
                    sleep(Duration::from_millis(50)).await;
                    flag_clone.store(true, Ordering::SeqCst);
                }));
            }));

            // Wait for inner task to complete
            sleep(Duration::from_millis(101)).await;
        });
        assert!(
            inner_task_completed.load(Ordering::SeqCst),
            "Nested spawned task did not complete"
        );
    }

    #[test]
    fn test_many_tasks() {
        const NUM_TASKS: usize = 1000;
        let rt = Runtime::default();
        let completed_count = Arc::new(AtomicUsize::new(0));

        rt.block_on(async {
            for i in 0..NUM_TASKS {
                let count_clone = completed_count.clone();
                rt.spawn(Box::pin(async move {
                    // Vary sleep times slightly to mix things up
                    sleep(Duration::from_millis(i as u64 % 100)).await;
                    count_clone.fetch_add(1, Ordering::SeqCst);
                }));
            }

            // Wait for everything to complete
            sleep(Duration::from_secs(1234)).await;
        });

        assert_eq!(
            completed_count.load(Ordering::SeqCst),
            NUM_TASKS,
            "Not all of the many tasks completed"
        );
    }

    #[test]
    #[should_panic(expected = "Spawned task panicked")]
    fn test_spawned_task_panics() {
        let rt = Runtime::default();
        rt.block_on(async {
            rt.spawn(Box::pin(async {
                sleep(Duration::from_millis(10)).await;
                panic!("Spawned task panicked");
            }));

            // Wait for the spawned task to execute
            sleep(Duration::from_secs(42)).await;
        });
    }

    #[test]
    fn test_zero_duration_sleep() {
        let rt = Runtime::default();
        let task_completed = Arc::new(AtomicBool::new(false));
        let flag_clone = task_completed.clone();

        rt.block_on(async {
            rt.spawn(Box::pin(async move {
                sleep(Duration::from_secs(0)).await;
                flag_clone.store(true, Ordering::SeqCst);
            }));

            // At least one await is necessary for the spawned future to run
            sleep(Duration::from_nanos(1)).await;
        });
        assert!(
            task_completed.load(Ordering::SeqCst),
            "Task with zero duration sleep did not complete"
        );
    }

    #[test]
    fn test_main_future_completes_before_spawned_tasks() {
        let rt = Runtime::default();
        let done_tasks = Arc::new(Mutex::new(Vec::new()));

        rt.block_on(async {
            let done_tasks_clone = done_tasks.clone();
            rt.spawn(Box::pin(async move {
                // This task will take longer than the main future and won't be polled to completion
                sleep(Duration::from_millis(200)).await;
                done_tasks_clone.lock().push("inner");
            }));

            done_tasks.lock().push("main");
        });

        let done_tasks = done_tasks.lock();
        assert_eq!(done_tasks[0], "main");
        assert_eq!(done_tasks.len(), 1);
    }

    struct WakyTask {
        wakes: Arc<AtomicUsize>,
        max_wakes: usize,
    }

    impl Future for WakyTask {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let current_wakes = self.wakes.fetch_add(1, Ordering::SeqCst);
            if current_wakes >= self.max_wakes {
                Poll::Ready(())
            } else {
                // Don't sleep on last iteration
                if current_wakes < self.max_wakes - 1 {
                    let waker = cx.waker().clone();

                    // Wake this task up after a short delay, to ensure the task actually
                    // goes to sleep
                    spawn(async move {
                        sleep(Duration::from_millis(10)).await;
                        waker.wake();
                    });
                } else {
                    cx.waker().wake_by_ref();
                }

                Poll::Pending
            }
        }
    }

    #[test]
    fn test_task_repeatedly_wakes_itself() {
        let rt = Runtime::default();
        let wakes = Arc::new(AtomicUsize::new(0));
        const MAX_WAKES: usize = 5;

        rt.block_on(async {
            spawn(WakyTask {
                wakes: wakes.clone(),
                max_wakes: MAX_WAKES,
            });

            // Wait for everything to complete
            sleep(Duration::from_secs(42)).await;
        });

        assert_eq!(
            wakes.load(Ordering::SeqCst),
            MAX_WAKES + 1,
            "Task was not polled the expected number of times"
        );
    }
}
