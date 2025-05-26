use crate::rt::task::WakeTask;
use parking_lot::Mutex;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

#[derive(Clone)]
pub(super) struct TaskWaker {
    task_id: u64,
    rt_event_queue: Arc<Mutex<Vec<WakeTask>>>,
}

impl TaskWaker {
    pub(super) fn new(rt_event_queue: Arc<Mutex<Vec<WakeTask>>>, task_id: u64) -> Self {
        Self {
            rt_event_queue,
            task_id,
        }
    }

    fn wake(self: Arc<Self>) {
        self.rt_event_queue.lock().push(WakeTask {
            task_id: self.task_id,
        });
    }

    pub(super) fn task_id(&self) -> u64 {
        self.task_id
    }

    pub(super) fn into_waker(self: Arc<Self>) -> Waker {
        unsafe { Waker::from_raw(RawWaker::new(Arc::into_raw(self) as _, &VTABLE)) }
    }
}

pub(crate) static VTABLE: RawWakerVTable = RawWakerVTable::new(
    // Clone
    |data| {
        let arc: Arc<TaskWaker> = unsafe { Arc::from_raw(data as _) };
        let cloned = arc.clone();
        std::mem::forget(arc);

        RawWaker::new(Arc::into_raw(cloned) as _, &VTABLE)
    },
    // Wake
    |data| {
        let arc: Arc<TaskWaker> = unsafe { Arc::from_raw(data as _) };
        arc.wake();
    },
    // Wake by ref
    |data| {
        let arc: Arc<TaskWaker> = unsafe { Arc::from_raw(data as _) };
        let cloned = arc.clone();
        std::mem::forget(arc);

        cloned.wake();
    },
    // Drop
    |data| {
        let _arc: Arc<TaskWaker> = unsafe { Arc::from_raw(data as _) };
    },
);
