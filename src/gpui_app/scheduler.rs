//! Placeholder scheduler for c43e505 compatibility.
//!
//! Later commits removed this module; keep minimal API surface here so
//! older call sites continue to compile without changing behavior.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

/// Schedule a repeating task that can be stopped via the provided flag.
#[allow(dead_code)]
pub fn schedule<F>(interval: Duration, stop: Arc<AtomicBool>, mut task: F)
where
    F: FnMut() + Send + 'static,
{
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            task();
            std::thread::sleep(interval);
        }
    });
}
