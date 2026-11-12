//! Round-9 desktop v4: a LIVE cancellation flag stops work inside ONE kernel call.
//!
//! The desktop Cancel button does not know which statement the worker is inside, so the engine
//! must poll a flag the host can set from another thread. This module proves the engine half
//! directly: the evaluation runs on the test thread while a helper thread sets the shared
//! `Arc<AtomicBool>` 200 ms into ONE long kernel call (`matmul(eye(400), eye(400))`, a single
//! array kernel that polls its own loop counter). The engine must publish the same structured
//! `Cancelled` / `BudgetExceeded` failure a spent deadline produces, well inside the two-second
//! bound; the measured millisecond latency is printed.
//!
//! A kernel whose work is a single opaque primitive has no poll seam, and the flag cannot reach
//! it: `sqrt(2)` at precision 200000 spends its time inside `lovelace-real`'s big-integer
//! operations and is documented as the remaining uninterruptible case in
//! `docs/goal-desktop-v4/round-9/kernel-cancel.md`. There is deliberately no test asserting that
//! it cancels.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::Engine;

/// How long the helper waits before the flag is set: long enough that the kernel is really
/// running (the whole computation takes many seconds), short enough to leave the 2 s budget
/// almost entirely for the stop itself.
const CANCEL_AFTER: Duration = Duration::from_millis(200);

/// The objective's hard bound, from flag-set to the returned failure.
const BOUND: Duration = Duration::from_secs(2);

#[test]
fn a_live_flag_set_from_another_thread_stops_one_long_kernel() {
    let mut engine = Engine::new();
    let flag = Arc::new(AtomicBool::new(false));
    engine.set_cancel_flag(Some(Arc::clone(&flag)));

    let setter = thread::spawn(move || {
        thread::sleep(CANCEL_AFTER);
        flag.store(true, Ordering::Release);
        // Read AFTER the store: the measured latency can only be an over-estimate.
        Instant::now()
    });

    let started = Instant::now();
    let outcome = engine.eval("matmul(eye(400), eye(400))");
    let stopped = Instant::now();
    let set_at = setter
        .join()
        .expect("the flag setter thread must not panic");

    let elapsed = stopped.saturating_duration_since(set_at);
    eprintln!(
        "kernel-cancel: matmul(eye(400), eye(400)) stopped {:.1} ms after the flag was set ({:.1} ms after eval began)",
        elapsed.as_secs_f64() * 1000.0,
        stopped.saturating_duration_since(started).as_secs_f64() * 1000.0
    );

    let error = match outcome {
        Ok(value) => panic!(
            "the live flag was not observed: the kernel returned {} after {:.3} s",
            value.display(),
            stopped.saturating_duration_since(started).as_secs_f64()
        ),
        Err(error) => error,
    };
    assert_eq!(
        error.code, "Cancelled",
        "the live flag must produce the structured cancellation failure: {error:?}"
    );
    assert_eq!(error.category, "BudgetExceeded");
    assert_eq!(error.message, crate::CANCELLED_MESSAGE);
    assert!(
        engine.was_cancelled(),
        "the engine must report that the live flag stopped the run"
    );
    assert!(
        elapsed < BOUND,
        "the live flag took {:.1} ms to stop one kernel, expected under {:.0} ms",
        elapsed.as_secs_f64() * 1000.0,
        BOUND.as_secs_f64() * 1000.0
    );
}

#[test]
fn a_cleared_flag_does_not_change_a_short_run() {
    let mut engine = Engine::new();
    let flag = Arc::new(AtomicBool::new(false));
    engine.set_cancel_flag(Some(flag));
    let value = engine
        .eval("sum(1..1000)")
        .expect("an un-cancelled run answers");
    assert_eq!(value.display(), "500500");
    assert!(!engine.was_cancelled());
}
