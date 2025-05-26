pub(crate) mod clock;
pub(crate) mod task;
pub(crate) mod waker;

#[cfg(feature = "send")]
pub(crate) mod send;

use crate::time::Instant;
use crate::time::timer::{PendingTimer, PendingTimerHandlerEnum, Timer};
use clock::{AdvanceClock, AdvanceToNextWake, RuntimeClock};
use parking_lot::Mutex;
use std::cell::{Cell, RefCell};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant as StdInstant};
use task::{Task, WakeTask};

thread_local! {
    static ACTIVE_RT: RefCell<Option<Runtime >> = const { RefCell::new(None) };
}

/// The main async runtime for sittard.
///
/// `Runtime` deterministically executes async tasks. It also keeps virtual time and advances the
/// clock as needed.
///
/// # Example
///
/// ```rust
/// use sittard::Runtime;
/// use std::time::Duration;
///
/// let rt = Runtime::default();
/// rt.block_on(async {
///     sittard::time::sleep(Duration::from_secs(1)).await;
///     println!("One second has passed (virtually)!");
/// });
/// ```
#[derive(Clone)]
pub struct Runtime {
    inner: Rc<RuntimeInner>,
}

pub(crate) struct RuntimeInner {
    // Time-keeping
    pub(crate) clock: RuntimeClock,
    pub(crate) advance_clock: Box<dyn AdvanceClock>,

    // Scheduling
    next_task_id: Cell<u64>,
    task_wakes_since_last_advance_clock: Arc<Mutex<Vec<WakeTask>>>,
    pub(crate) pending_timers_since_last_advance_clock: Arc<Mutex<Vec<Arc<PendingTimer>>>>,
    ready_to_poll_tasks: RefCell<VecDeque<Task>>,
    pending_timers: RefCell<BinaryHeap<Arc<PendingTimer>>>,
    wakers_by_timer_id: RefCell<HashMap<u64, Vec<Waker>>>,
    blocked_tasks_by_id: RefCell<HashMap<u64, Task>>,
}

impl RuntimeInner {
    pub(crate) fn get_next_id(&self) -> u64 {
        let next_id = self.next_task_id.get();
        self.next_task_id.set(next_id + 1);
        next_id
    }
}

impl Debug for Runtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rt")
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new(Box::new(AdvanceToNextWake))
    }
}

impl Runtime {
    /// Creates a new runtime with a custom [`AdvanceClock`] implementation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sittard::{Runtime, AdvanceToNextWake};
    /// let rt = Runtime::new(Box::new(AdvanceToNextWake));
    /// ```
    pub fn new(advance_clock: Box<dyn AdvanceClock>) -> Self {
        let now = StdInstant::now();
        Self {
            inner: Rc::new(RuntimeInner {
                clock: RuntimeClock {
                    now: Arc::new(Mutex::new(now)),
                },
                advance_clock,
                next_task_id: Default::default(),
                task_wakes_since_last_advance_clock: Arc::default(),
                pending_timers_since_last_advance_clock: Arc::default(),
                ready_to_poll_tasks: Default::default(),
                pending_timers: Default::default(),
                wakers_by_timer_id: Default::default(),
                blocked_tasks_by_id: Default::default(),
            }),
        }
    }

    /// Creates a new timer that will complete at the specified deadline.
    ///
    /// This is a low-level method for creating timers. Most users should prefer the functions in
    /// the [`crate::time`] module.
    pub fn new_timer(&self, deadline: StdInstant) -> Timer {
        Timer::new(&self.inner, deadline)
    }

    /// Creates a timer that will complete after the specified duration.
    ///
    /// This is a low-level method for creating timers. Most users should prefer the functions in
    /// the [`crate::time`] module.
    pub fn sleep(&self, duration: Duration) -> Timer {
        let deadline = self.now() + duration;
        self.new_timer(deadline)
    }

    /// Creates a timer that will complete at the specified absolute time.
    ///
    /// This is a low-level method for creating timers. Most users should prefer
    /// the functions in the [`crate::time`] module.
    pub fn sleep_until(&self, deadline: Instant) -> Timer {
        self.new_timer(deadline.0)
    }

    /// Spawns a future as a new task on this runtime.
    ///
    /// This is a low-level method for spawning tasks. Most users should prefer the [`crate::spawn`]
    /// function, which allows you to await tasks and observe their return values.
    ///
    /// Note: spawned tasks run concurrently, but not in parallel, with other tasks.
    pub fn spawn<T: Future<Output = ()> + 'static>(&self, future: T) {
        self.spawn_boxed(Box::pin(future))
    }

    /// Spawns a boxed future as a new task on this runtime.
    ///
    /// This is a low-level method that accepts an already-boxed future.
    /// Most users should prefer [`crate::spawn`].
    pub fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = ()> + 'static>>) {
        self.inner
            .ready_to_poll_tasks
            .borrow_mut()
            .push_back(Task { future });
    }

    /// Returns the current virtual time.
    ///
    /// This returns the current time according to the runtime's virtual clock,
    /// which may be different from the real system time.
    pub fn now(&self) -> std::time::Instant {
        self.inner.clock.now()
    }

    /// Returns the currently active runtime.
    ///
    /// This function returns the runtime that is currently executing in the current thread.
    /// It can be used to access the runtime from within async code.
    ///
    /// # Panics
    ///
    /// Panics if called outside of a runtime context (i.e., not within `Runtime::block_on`).
    pub fn active() -> Runtime {
        let maybe_rt = ACTIVE_RT.with_borrow(Option::clone);
        match maybe_rt {
            Some(rt) => rt,
            None => panic!("async runtime is not active in the current thread"),
        }
    }

    fn register_active(&self) {
        let previously_registered = ACTIVE_RT.replace(Some(self.clone()));
        if previously_registered.is_some() {
            panic!("Called `Rt::block_on` inside an active `Rt::block_on`");
        }
    }

    fn unregister_active(&self) {
        ACTIVE_RT.set(None);
    }

    /// Runs a future to completion.
    ///
    /// This method executes the runtime's event loop, advancing the virtual clock as needed, until
    /// the provided future completes. It is the main entry point for running async code with
    /// sittard.
    ///
    /// Important: `block_on` will return as soon as the future completes, meaning that any spawned
    /// tasks will stop being polled (until the next time `block_on` is called).
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Called from within an already active runtime context.
    /// - The future is blocked but no progress can be made (i.e. there are no pending timers).
    ///
    /// # Example
    ///
    /// ```rust
    /// use sittard::Runtime;
    /// use std::time::Duration;
    ///
    /// let rt = Runtime::default();
    /// let result = rt.block_on(async {
    ///     sittard::time::sleep(Duration::from_secs(1)).await;
    ///     42
    /// });
    /// assert_eq!(result, 42);
    /// ```
    pub fn block_on<T>(&self, f: impl Future<Output = T>) -> T {
        self.register_active();

        let mut f = pin!(f);
        let ready = loop {
            self.wake_tasks_since_last_advance_clock();
            self.poll_tasks_until_all_blocked();
            self.wake_tasks_since_last_advance_clock();

            // Poll the main task. Note: we poll on every loop iteration, so we don't need to use a
            // real waker.
            let mut cx = Context::from_waker(Waker::noop());
            if let Poll::Ready(value) = f.as_mut().poll(&mut cx) {
                // We are done as soon as the main task is ready
                break value;
            }

            self.set_timer_wakers_since_last_advance_clock();

            // Advance the clock only when there are no ready-to-poll tasks
            if self.inner.ready_to_poll_tasks.borrow().is_empty() {
                self.advance_clock();
            }
        };

        self.unregister_active();
        ready
    }

    fn wake_tasks_since_last_advance_clock(&self) {
        let mut task_wakes = self.inner.task_wakes_since_last_advance_clock.lock();
        for wake in task_wakes.drain(..) {
            let Some(blocked) = self
                .inner
                .blocked_tasks_by_id
                .borrow_mut()
                .remove(&wake.task_id)
            else {
                // Already woken, nothing to do here
                continue;
            };

            self.inner
                .ready_to_poll_tasks
                .borrow_mut()
                .push_back(blocked);
        }
    }

    fn poll_tasks_until_all_blocked(&self) {
        loop {
            let Some(mut task) = self.inner.ready_to_poll_tasks.borrow_mut().pop_front() else {
                break;
            };

            // Poll the next task in the queue
            let task_id = self.inner.get_next_id();
            let waker = Arc::new(waker::TaskWaker::new(
                self.inner.task_wakes_since_last_advance_clock.clone(),
                task_id,
            ));
            let cx_waker = waker.clone().into_waker();
            let mut cx = Context::from_waker(&cx_waker);
            match task.future.as_mut().poll(&mut cx) {
                Poll::Ready(()) => {
                    // Task is done, nothing else to do
                }
                Poll::Pending => {
                    // Task is pending and is waiting to be woken
                    self.inner
                        .blocked_tasks_by_id
                        .borrow_mut()
                        .insert(task_id, task);
                }
            }
        }
    }

    fn set_timer_wakers_since_last_advance_clock(&self) {
        for pending in self
            .inner
            .pending_timers_since_last_advance_clock
            .lock()
            .drain(..)
        {
            self.inner
                .wakers_by_timer_id
                .borrow_mut()
                .entry(pending.timer_id)
                .or_default()
                .push(pending.waker.clone());

            self.inner.pending_timers.borrow_mut().push(pending);
        }
    }

    fn advance_clock(&self) {
        loop {
            let blocked_tasks = self.inner.blocked_tasks_by_id.borrow().len();
            let timer = match self.inner.pending_timers.borrow_mut().pop() {
                // There's a pending timer, so we can advance!
                Some(timer) => timer,
                // No pending timers, so advancing time won't let us make progress
                None => panic!(
                    "`block_on` is stuck: the task's future is pending and there is nothing else to do, nor timers waiting to advance. There are {blocked_tasks} blocked tasks."
                ),
            };

            let at_least_one_task_unblocked = self.handle_timer_elapsed(timer);
            if at_least_one_task_unblocked {
                // We made progress, so we can stop handling timers. However, if the next timer is
                // ready, we will handle it right away.
                let next_timer_ready = self
                    .inner
                    .pending_timers
                    .borrow()
                    .peek()
                    .is_some_and(|t| t.elapsed_at <= self.inner.clock.now());
                if !next_timer_ready {
                    break;
                }
            }
        }
    }

    fn handle_timer_elapsed(&self, timer: Arc<PendingTimer>) -> bool {
        match timer.handler.as_enum() {
            PendingTimerHandlerEnum::WakeWaitingTasks => {
                // Advance the timer if necessary
                let now = self.inner.clock.now();
                if now < timer.elapsed_at {
                    let new_time = self
                        .inner
                        .advance_clock
                        .advance_clock(now, timer.elapsed_at);
                    if new_time < timer.elapsed_at {
                        panic!(
                            "`AdvanceClock` implementation returned an instant before the next wake is reached"
                        );
                    }

                    self.inner.clock.set_now(new_time);
                }

                // Wake all waiting tasks
                let wakers = self
                    .inner
                    .wakers_by_timer_id
                    .borrow_mut()
                    .remove(&timer.timer_id)
                    .unwrap_or_default();
                for waker in wakers {
                    waker.wake();
                }

                // At least one task was unblocked
                true
            }
            PendingTimerHandlerEnum::Ignore => {
                // No tasks unblocked
                false
            }
            PendingTimerHandlerEnum::CancelWaitingTasks => {
                let wakers = self
                    .inner
                    .wakers_by_timer_id
                    .borrow_mut()
                    .remove(&timer.timer_id)
                    .unwrap_or_default();

                // Cancel waiting tasks
                for waker in wakers {
                    let is_noop_waker = waker.data().is_null();
                    if is_noop_waker {
                        continue;
                    }

                    // Obtain the task id from the waker
                    let waker: Arc<waker::TaskWaker> = unsafe { Arc::from_raw(waker.data() as _) };
                    let task_id = waker.task_id();
                    std::mem::forget(waker);

                    // Remove blocked task
                    self.inner.blocked_tasks_by_id.borrow_mut().remove(&task_id);
                }

                // No tasks unblocked, only cancelled
                false
            }
        }
    }
}
