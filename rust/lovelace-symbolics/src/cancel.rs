//! Cooperative deadline observation for long-running kernels.
//!
//! The runner's `--cancel-after` budget used to be observed only BETWEEN top-level statements,
//! so one long statement — a range build, a reduction, a matrix product, a simplifier pass, a
//! solver search — ran to completion and the runner reported cancellation post hoc, or refused
//! for an unrelated reason that happened to be reached first. This module is the shared, cheap
//! token those loops poll.
//!
//! The state is THREAD LOCAL, not global: a deadline belongs to the evaluation that armed it, so
//! two engines evaluated on two threads cannot cancel each other, and a test that arms a deadline
//! cannot leak it into the rest of the process. Reading it is one thread-local read; the clock is
//! read once per [`POLL_INTERVAL`] iterations, never once per element.
//!
//! When no deadline is armed the hot path is a single `Option` read, so an un-budgeted run pays
//! nothing measurable and its results are unchanged.

use std::cell::Cell;
use std::time::Instant;

/// Iterations between clock reads. 64 keeps the check below 0.1 % of a loop whose body is one
/// boxed arithmetic operation and bounds the stop latency by 64 loop bodies, which is what the
/// reference's `KernelCancellation.Interval` publishes (`Lovelace.Suite/TypedArrayOps.cs`).
pub const POLL_INTERVAL: u64 = 64;

thread_local! {
    /// The deadline the current evaluation armed, if any.
    static DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
    /// Set the first time a kernel observes the deadline. The evaluation reads it after the
    /// kernel returns, whatever the kernel chose to return.
    static EXPIRED: Cell<bool> = const { Cell::new(false) };
}

/// The armed state a [`Scope`] restores: an evaluation never inherits or leaks a deadline.
pub struct Scope {
    previous_deadline: Option<Instant>,
    previous_expired: bool,
}

impl Drop for Scope {
    fn drop(&mut self) {
        DEADLINE.with(|cell| cell.set(self.previous_deadline));
        EXPIRED.with(|cell| cell.set(self.previous_expired));
    }
}

/// Arms `deadline` for the current thread and returns the guard that restores the previous state
/// on drop. `None` disarms (the un-budgeted path).
pub fn arm(deadline: Option<Instant>) -> Scope {
    let previous_deadline = DEADLINE.with(|cell| cell.replace(deadline));
    let previous_expired = EXPIRED.with(|cell| cell.replace(false));
    Scope {
        previous_deadline,
        previous_expired,
    }
}

/// True when a kernel has already observed the deadline in this evaluation.
#[inline]
pub fn observed() -> bool {
    EXPIRED.with(Cell::get)
}

/// True when the armed deadline has passed (or a kernel already observed it).
///
/// This is the full check; a loop calls [`poll`], which only pays for the clock every
/// [`POLL_INTERVAL`] iterations.
#[inline]
pub fn expired() -> bool {
    if EXPIRED.with(Cell::get) {
        return true;
    }
    let Some(at) = DEADLINE.with(Cell::get) else {
        return false;
    };
    if Instant::now() >= at {
        EXPIRED.with(|cell| cell.set(true));
        return true;
    }
    false
}

/// One strided check for a hot loop. The caller passes the loop's own counter, so no second
/// mutable state (and no per-loop capture) is needed. Returns true when the loop must stop.
///
/// A kernel that gets `true` returns its identity/partial value immediately; the evaluation
/// discards that value and reports the structured `Cancelled` refusal instead, so the partial
/// value is never observable.
#[inline]
pub fn poll(iteration: u64) -> bool {
    if EXPIRED.with(Cell::get) {
        return true;
    }
    if DEADLINE.with(Cell::get).is_none() {
        return false;
    }
    if (iteration & (POLL_INTERVAL - 1)) == 0 {
        expired()
    } else {
        false
    }
}

/// An unstrided check for a recursion boundary or an outer loop whose body is already a full
/// sweep. Cheap enough to call once per node/statement.
#[inline]
pub fn check() -> bool {
    expired()
}
