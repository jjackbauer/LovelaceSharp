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
//! When neither a deadline nor a flag is armed the hot path is one thread-local `Option` read
//! plus one slot-presence check, so an un-budgeted run pays nothing measurable and its results
//! are unchanged.
//!
//! A host that must stop a run from OUTSIDE the evaluating thread — the desktop Cancel button —
//! arms a live [`std::sync::atomic::AtomicBool`] instead of (or beside) the deadline. The flag is
//! read from the same polling seams as the clock, so one long kernel stops even when no statement
//! boundary is reached. The deadline is thread local and the flag is a shared handle installed in
//! that same thread-local slot: other threads can set it, but only the evaluating thread can
//! observe it.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Iterations between clock reads. 64 keeps the check below 0.1 % of a loop whose body is one
/// boxed arithmetic operation and bounds the stop latency by 64 loop bodies, which is what the
/// reference's `KernelCancellation.Interval` publishes (`Lovelace.Suite/TypedArrayOps.cs`).
pub const POLL_INTERVAL: u64 = 64;

thread_local! {
    /// The deadline the current evaluation armed, if any.
    static DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
    /// Set the first time a kernel observes the deadline or the live flag. The evaluation reads it
    /// after the kernel returns, whatever the kernel chose to return.
    static EXPIRED: Cell<bool> = const { Cell::new(false) };
    /// The live flag a host can set while this evaluation is running (desktop Cancel). `None` when
    /// the caller only budgeted by deadline.
    static FLAG: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

/// The armed state a [`Scope`] restores: an evaluation never inherits or leaks a deadline or a
/// live flag.
pub struct Scope {
    previous_deadline: Option<Instant>,
    previous_expired: bool,
    previous_flag: Option<Arc<AtomicBool>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        DEADLINE.with(|cell| cell.set(self.previous_deadline));
        EXPIRED.with(|cell| cell.set(self.previous_expired));
        FLAG.with(|cell| {
            *cell.borrow_mut() = self.previous_flag.take();
        });
    }
}

/// Arms `deadline` for the current thread and returns the guard that restores the previous state
/// on drop. `None` disarms (the un-budgeted path).
pub fn arm(deadline: Option<Instant>) -> Scope {
    arm_with_flag(deadline, None)
}

/// Arms `deadline` and the live `flag` for the current thread and returns the guard that restores
/// the previous state on drop.
///
/// Either argument may be `None`: a deadline-only arm is exactly [`arm`], a flag-only arm lets a
/// host stop a run whose work has no budget, and `(None, None)` disarms both. The flag is checked
/// before the clock, so a set flag costs one atomic load and never reads the clock.
pub fn arm_with_flag(deadline: Option<Instant>, flag: Option<Arc<AtomicBool>>) -> Scope {
    let previous_deadline = DEADLINE.with(|cell| cell.replace(deadline));
    let previous_expired = EXPIRED.with(|cell| cell.replace(false));
    let previous_flag = FLAG.with(|cell| cell.replace(flag));
    Scope {
        previous_deadline,
        previous_expired,
        previous_flag,
    }
}

/// True when the thread-local slot holds a flag that is currently set.
#[inline]
fn flag_set() -> bool {
    FLAG.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Acquire))
    })
}

/// True when either a deadline or a live flag is armed for this thread.
#[inline]
fn armed() -> bool {
    DEADLINE.with(|cell| cell.get().is_some()) || FLAG.with(|cell| cell.borrow().is_some())
}

/// True when a kernel has already observed the deadline or the live flag in this evaluation.
#[inline]
pub fn observed() -> bool {
    EXPIRED.with(Cell::get)
}

/// True when the armed deadline has passed, the live flag is set, or a kernel already observed
/// either.
///
/// This is the full check; a loop calls [`poll`], which only pays for it every [`POLL_INTERVAL`]
/// iterations. The live flag is checked before the clock: one atomic load, no clock read.
#[inline]
pub fn expired() -> bool {
    if EXPIRED.with(Cell::get) {
        return true;
    }
    if flag_set() {
        EXPIRED.with(|cell| cell.set(true));
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
    if !armed() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A set live flag expires immediately, with no deadline armed.
    #[test]
    fn a_set_flag_expires_immediately() {
        let flag = Arc::new(AtomicBool::new(true));
        let _scope = arm_with_flag(None, Some(Arc::clone(&flag)));
        assert!(expired(), "a set flag must expire the evaluation");
        assert!(observed(), "expiring must latch the observation");
    }

    /// A flag that is not set does not expire, and neither does the un-armed state.
    #[test]
    fn a_cleared_flag_does_not_expire() {
        let flag = Arc::new(AtomicBool::new(false));
        let _scope = arm_with_flag(None, Some(Arc::clone(&flag)));
        assert!(!expired());
        assert!(!check());
        assert!(!poll(0));
        assert!(!observed());
    }

    /// Setting the flag AFTER the arm is what the desktop does: the next poll seam must observe it
    /// without touching the deadline.
    #[test]
    fn a_flag_set_after_arming_is_observed_at_the_poll_seam() {
        let flag = Arc::new(AtomicBool::new(false));
        let _scope = arm_with_flag(None, Some(Arc::clone(&flag)));
        assert!(!poll(63), "nothing is set yet");
        flag.store(true, Ordering::Release);
        assert!(
            poll(64),
            "the stride seam must read the live flag, not only the clock"
        );
        assert!(observed());
        assert!(poll(65), "the observation stays latched");
    }

    /// Arming without a flag keeps the old deadline behaviour: no deadline is not expired, a
    /// spent deadline is, a future one is not, and an installed flag is disarmed.
    #[test]
    fn arming_without_a_flag_keeps_the_old_behaviour() {
        let flag = Arc::new(AtomicBool::new(true));
        {
            let _with_flag = arm_with_flag(None, Some(Arc::clone(&flag)));
            assert!(expired());
        }
        {
            let _plain = arm(None);
            assert!(!expired(), "arm(None) must be the un-budgeted path");
            assert!(!check());
        }
        {
            let _spent = arm(Some(Instant::now() - Duration::from_secs(1)));
            assert!(expired(), "a spent deadline still expires");
        }
        {
            let _future = arm(Some(Instant::now() + Duration::from_secs(60)));
            assert!(!expired(), "a future deadline does not expire");
        }
    }

    /// Dropping a scope restores the flag it replaced, including the outer flag's live value.
    #[test]
    fn the_scope_restores_a_previous_flag() {
        let outer = Arc::new(AtomicBool::new(false));
        let outer_scope = arm_with_flag(None, Some(Arc::clone(&outer)));
        {
            let inner = Arc::new(AtomicBool::new(true));
            let _inner_scope = arm_with_flag(None, Some(inner));
            assert!(expired(), "the inner armed flag expires");
        }
        assert!(
            !expired(),
            "the inner flag must not leak into the outer scope"
        );
        outer.store(true, Ordering::Release);
        assert!(expired(), "the outer flag is installed again and now set");
        drop(outer_scope);
        let _disarmed = arm(None);
        assert!(!expired(), "scope drop must restore the un-armed state");
    }
}
