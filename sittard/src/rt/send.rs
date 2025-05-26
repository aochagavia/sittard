use crate::{AdvanceToNextWake, Runtime};
use send_wrapper::SendWrapper;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

pub struct SendRuntime {
    inner: SendWrapper<Runtime>,
}

impl SendRuntime {
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
