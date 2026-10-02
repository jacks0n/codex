//! Runtime-only Full Access state shared by an agent tree.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// Lock-free Full Access override shared by a root thread and its agents.
#[derive(Debug, Default)]
pub(crate) struct RuntimeFullAccessState {
    enabled: AtomicBool,
}

impl RuntimeFullAccessState {
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }
}

#[cfg(test)]
#[path = "runtime_permissions_tests.rs"]
mod tests;
