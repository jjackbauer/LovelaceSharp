//! The explicit run-state machine behind the status strip.
//!
//! One document owns one [`RunStateMachine`]. Every state change goes through it, so the state
//! shown in the status strip and the sequence asserted by `--gui-selftest` come from the same
//! transitions instead of from conditions scattered across the window code.
//!
//! # States
//!
//! | state | meaning | status strip |
//! | --- | --- | --- |
//! | [`RunState::Idle`] | no run has been requested for this document | `run Idle` |
//! | [`RunState::Queued`] | `submit` was accepted, the worker has not reported `RunStarted` | `run Queued` |
//! | [`RunState::Running`] | the worker reported `RunStarted` | `run Running` |
//! | [`RunState::Cancelling`] | a cancellation was accepted, the terminal event has not arrived | `run Cancelling` |
//! | [`RunState::Finished`] | terminal success, its snapshot is displayed | `run Finished` |
//! | [`RunState::Cancelled`] | terminal cancellation | `run Cancelled` |
//! | [`RunState::Failed`] | terminal failure | `run Failed` |
//! | [`RunState::Stale`] | the source advanced after a finished result; the result no longer describes the buffer | `run Stale` |
//!
//! # Transitions
//!
//! ```text
//! Idle | Finished | Cancelled | Failed | Stale --submit accepted--> Queued
//! Queued --RunStarted--> Running
//! Queued | Running --cancel requested--> Cancelling
//! Queued | Running | Cancelling --Finished event--> Finished
//! Queued | Running | Cancelling --Cancelled event--> Cancelled
//! Queued | Running | Cancelling --Failed event--> Failed
//! Finished --source revision advances--> Stale
//! Stale --source revision advances--> Stale   (no transition; Finished is not resurrected)
//! ```
//!
//! Exactly one terminal run event (`Finished`, `Cancelled` or `Failed`) is accepted per run:
//! [`RunStateMachine::finish`] requires the state to still be in flight and the run id to match
//! the tracked run, so a duplicated or late terminal event is refused.

use std::fmt;
use std::time::Instant;

use crate::execution::RunId;

/// Maximum number of transitions retained per document; the oldest are evicted first.
pub const TRANSITION_CAP: usize = 256;

/// The explicit state of one document's run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RunState {
    /// No run has been requested for this document.
    Idle,
    /// `submit` was accepted; the worker has not reported `RunStarted` yet.
    Queued,
    /// The worker reported `RunStarted`.
    Running,
    /// Cancellation was accepted; the terminal event has not arrived yet.
    Cancelling,
    /// Terminal success; the published snapshot is displayed.
    Finished,
    /// Terminal cancellation.
    Cancelled,
    /// Terminal failure (a statement failed or the engine panicked).
    Failed,
    /// A finished result is displayed but the source revision has advanced since then.
    Stale,
}

impl RunState {
    /// The seven run states, in lifecycle order. [`RunState::Idle`] is the no-run state and is
    /// deliberately not part of this list.
    #[allow(dead_code)] // part of the documented state table; exercised by the unit tests
    pub const RUN_STATES: [RunState; 7] = [
        RunState::Queued,
        RunState::Running,
        RunState::Cancelling,
        RunState::Finished,
        RunState::Cancelled,
        RunState::Failed,
        RunState::Stale,
    ];

    /// The status-strip label, without the `run ` prefix.
    pub fn label(self) -> &'static str {
        match self {
            RunState::Idle => "Idle",
            RunState::Queued => "Queued",
            RunState::Running => "Running",
            RunState::Cancelling => "Cancelling",
            RunState::Finished => "Finished",
            RunState::Cancelled => "Cancelled",
            RunState::Failed => "Failed",
            RunState::Stale => "Stale",
        }
    }

    /// True while a run is in flight: `Queued`, `Running` or `Cancelling`.
    pub fn is_in_flight(self) -> bool {
        matches!(
            self,
            RunState::Queued | RunState::Running | RunState::Cancelling
        )
    }

    /// True when the state is a displayed result and no run is in flight.
    ///
    /// This includes [`RunState::Stale`], which is a displayed (out-of-date) result but not a
    /// terminal *run event*: the run event was `Finished` and an edit moved the display on.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RunState::Finished | RunState::Cancelled | RunState::Failed | RunState::Stale
        )
    }

    /// True for the three terminal run events the execution core emits exactly once per run.
    pub fn is_terminal_run_event(self) -> bool {
        matches!(
            self,
            RunState::Finished | RunState::Cancelled | RunState::Failed
        )
    }

    /// The [`crate::run_trace`] mark used when this state is entered.
    pub fn trace_label(self) -> &'static str {
        match self {
            RunState::Idle => "run_state_idle",
            RunState::Queued => "run_state_queued",
            RunState::Running => "run_state_running",
            RunState::Cancelling => "run_state_cancelling",
            RunState::Finished => "run_state_finished",
            RunState::Cancelled => "run_state_cancelled",
            RunState::Failed => "run_state_failed",
            RunState::Stale => "run_state_stale",
        }
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// One accepted transition, with the run it belongs to and a monotonic timestamp.
#[derive(Clone, Debug)]
pub struct RunStateTransition {
    /// The run the transition belongs to (the run whose terminal result is displayed for a
    /// `Finished -> Stale` transition).
    pub run: Option<RunId>,
    /// State before the transition.
    pub from: RunState,
    /// State after the transition.
    pub to: RunState,
    /// Monotonic time the transition was accepted.
    pub at: Instant,
    /// Why the transition happened, for the event log and the selftest report.
    pub reason: String,
}

/// The run-state machine and transition log of one document.
#[derive(Clone, Debug)]
pub struct RunStateMachine {
    state: RunState,
    run: Option<RunId>,
    transitions: Vec<RunStateTransition>,
}

impl Default for RunStateMachine {
    fn default() -> RunStateMachine {
        RunStateMachine::new()
    }
}

impl RunStateMachine {
    /// Creates a machine in [`RunState::Idle`].
    pub fn new() -> RunStateMachine {
        RunStateMachine {
            state: RunState::Idle,
            run: None,
            transitions: Vec::new(),
        }
    }

    /// The current state.
    pub fn state(&self) -> RunState {
        self.state
    }

    /// The run the machine is following: the in-flight run, or the run whose terminal result (or
    /// [`RunState::Stale`] marker) is displayed.
    pub fn run(&self) -> Option<RunId> {
        self.run
    }

    /// Every accepted transition, oldest first.
    pub fn transitions(&self) -> &[RunStateTransition] {
        &self.transitions
    }

    /// Number of transitions retained.
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }

    /// Number of accepted terminal run events (`Finished`, `Cancelled`, `Failed`) in the log.
    #[allow(dead_code)] // host-facing invariant accessor used by the workbench tests
    pub fn terminal_run_event_count(&self) -> usize {
        self.transitions
            .iter()
            .filter(|transition| transition.to.is_terminal_run_event())
            .count()
    }

    /// Duration of the first adjacent `from -> to` transition pair in the log, when present.
    pub fn duration_between(&self, from: RunState, to: RunState) -> Option<std::time::Duration> {
        self.transitions
            .windows(2)
            .find(|pair| pair[0].to == from && pair[1].to == to)
            .map(|pair| pair[1].at.saturating_duration_since(pair[0].at))
    }

    /// Records that `submit` was accepted: `Idle` or any terminal display state -> `Queued`.
    ///
    /// Returns false when a run is already in flight, so a second submit can never overwrite the
    /// active run.
    pub fn begin(&mut self, run: RunId, reason: impl Into<String>) -> bool {
        if self.state.is_in_flight() {
            return false;
        }
        if !(self.state == RunState::Idle || self.state.is_terminal()) {
            return false;
        }
        self.transition(RunState::Queued, Some(run), reason.into())
    }

    /// Records that the worker reported `RunStarted`: `Queued` -> `Running`.
    ///
    /// A run that was cancelled while queued stays `Cancelling`; `RunStarted` must not resurrect
    /// `Running` after the user already asked to stop.
    pub fn started(&mut self, run: RunId, reason: impl Into<String>) -> bool {
        if self.state != RunState::Queued || self.run != Some(run) {
            return false;
        }
        self.transition(RunState::Running, Some(run), reason.into())
    }

    /// Records an accepted cancellation request: `Queued` or `Running` -> `Cancelling`.
    pub fn cancel(&mut self, run: RunId, reason: impl Into<String>) -> bool {
        if !matches!(self.state, RunState::Queued | RunState::Running) || self.run != Some(run) {
            return false;
        }
        self.transition(RunState::Cancelling, Some(run), reason.into())
    }

    /// Records the one terminal run event for this run.
    ///
    /// `terminal` must be [`RunState::Finished`], [`RunState::Cancelled`] or [`RunState::Failed`].
    /// The state must still be in flight and the run id must match, so a second terminal event
    /// for the same run (or a late event from an older run) is refused.
    pub fn finish(
        &mut self,
        run: RunId,
        terminal: RunState,
        reason: impl Into<String>,
    ) -> bool {
        if !terminal.is_terminal_run_event() {
            return false;
        }
        if !self.state.is_in_flight() || self.run != Some(run) {
            return false;
        }
        self.transition(terminal, Some(run), reason.into())
    }

    /// Marks a displayed finished result as stale: `Finished` -> `Stale`.
    ///
    /// Once stale, another edit is a no-op: the log keeps exactly one `Finished -> Stale`
    /// transition and nothing can move the display back to `Finished` except a new run.
    pub fn stale(&mut self, run: RunId, reason: impl Into<String>) -> bool {
        if self.state != RunState::Finished || self.run != Some(run) {
            return false;
        }
        self.transition(RunState::Stale, Some(run), reason.into())
    }

    /// Resets to [`RunState::Idle`], for a document whose content was replaced (open/load).
    ///
    /// Returns false when the machine was already idle.
    pub fn reset(&mut self, reason: impl Into<String>) -> bool {
        if self.state == RunState::Idle {
            return false;
        }
        let run = self.run;
        let from = self.state;
        self.state = RunState::Idle;
        self.run = None;
        self.push_transition(RunStateTransition {
            run,
            from,
            to: RunState::Idle,
            at: Instant::now(),
            reason: reason.into(),
        });
        true
    }

    /// Applies a legal transition and records it.
    fn transition(&mut self, to: RunState, run: Option<RunId>, reason: String) -> bool {
        let from = self.state;
        if from == to || !allowed(from, to) {
            return false;
        }
        self.state = to;
        if run.is_some() {
            self.run = run;
        }
        self.push_transition(RunStateTransition {
            run: self.run,
            from,
            to,
            at: Instant::now(),
            reason,
        });
        true
    }

    fn push_transition(&mut self, transition: RunStateTransition) {
        if crate::run_trace::enabled() {
            crate::run_trace::mark(
                transition.to.trace_label(),
                format!(
                    "from={} run={} document_state_reason={}",
                    transition.from.label(),
                    transition
                        .run
                        .map(|run| run.0.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    transition.reason
                ),
            );
        }
        if self.transitions.len() >= TRANSITION_CAP {
            self.transitions.remove(0);
        }
        self.transitions.push(transition);
    }
}

/// The complete transition relation of the machine.
fn allowed(from: RunState, to: RunState) -> bool {
    match (from, to) {
        (
            RunState::Idle
            | RunState::Finished
            | RunState::Cancelled
            | RunState::Failed
            | RunState::Stale,
            RunState::Queued,
        ) => true,
        (RunState::Queued, RunState::Running) => true,
        (RunState::Queued | RunState::Running, RunState::Cancelling) => true,
        (
            RunState::Queued | RunState::Running | RunState::Cancelling,
            RunState::Finished | RunState::Cancelled | RunState::Failed,
        ) => true,
        (RunState::Finished, RunState::Stale) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine() -> RunStateMachine {
        RunStateMachine::new()
    }

    #[test]
    fn every_run_state_has_its_own_status_strip_label() {
        let labels = RunState::RUN_STATES.map(RunState::label);
        assert_eq!(
            labels,
            [
                "Queued",
                "Running",
                "Cancelling",
                "Finished",
                "Cancelled",
                "Failed",
                "Stale"
            ]
        );
        assert_eq!(RunState::Idle.label(), "Idle");
        for state in RunState::RUN_STATES {
            assert!(
                format!("run {}", state.label()).starts_with("run "),
                "missing run prefix for {state}"
            );
        }
        let mut unique = labels.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 7, "labels must be distinct: {labels:?}");
    }

    #[test]
    fn idle_submit_run_start_finish_is_the_success_sequence() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert_eq!(machine.state(), RunState::Queued);
        assert!(machine.started(RunId(1), "RunStarted"));
        assert_eq!(machine.state(), RunState::Running);
        assert!(machine.finish(RunId(1), RunState::Finished, "Finished"));
        assert_eq!(machine.state(), RunState::Finished);
        let sequence = machine
            .transitions()
            .iter()
            .map(|transition| transition.to)
            .collect::<Vec<_>>();
        assert_eq!(
            sequence,
            [
                RunState::Queued,
                RunState::Running,
                RunState::Finished
            ]
        );
        assert_eq!(machine.terminal_run_event_count(), 1);
    }

    #[test]
    fn cancel_while_queued_stays_cancelling_when_run_started_arrives() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert!(machine.cancel(RunId(1), "cancel"));
        assert_eq!(machine.state(), RunState::Cancelling);
        assert!(
            !machine.started(RunId(1), "RunStarted"),
            "RunStarted must not resurrect Running after a cancel"
        );
        assert_eq!(machine.state(), RunState::Cancelling);
        assert!(machine.finish(RunId(1), RunState::Cancelled, "Cancelled"));
        assert_eq!(machine.state(), RunState::Cancelled);
    }

    #[test]
    fn cancel_while_running_is_cancelling_then_cancelled() {
        let mut machine = machine();
        assert!(machine.begin(RunId(7), "submit"));
        assert!(machine.started(RunId(7), "RunStarted"));
        assert!(machine.cancel(RunId(7), "cancel"));
        assert_eq!(machine.state(), RunState::Cancelling);
        assert!(machine.finish(RunId(7), RunState::Cancelled, "Cancelled"));
        assert_eq!(machine.state(), RunState::Cancelled);
    }

    #[test]
    fn exactly_one_terminal_run_event_is_accepted_per_run() {
        let mut machine = machine();
        assert!(machine.begin(RunId(3), "submit"));
        assert!(machine.started(RunId(3), "RunStarted"));
        assert!(machine.finish(RunId(3), RunState::Finished, "Finished"));
        assert!(
            !machine.finish(RunId(3), RunState::Finished, "duplicate Finished"),
            "a duplicate Finished must be refused"
        );
        assert!(
            !machine.finish(RunId(3), RunState::Cancelled, "late Cancelled"),
            "a different terminal event for the same run must be refused"
        );
        assert!(
            !machine.finish(RunId(4), RunState::Failed, "unknown run"),
            "a terminal event with the wrong run id must be refused"
        );
        assert_eq!(machine.terminal_run_event_count(), 1);
    }

    #[test]
    fn a_new_run_after_a_terminal_state_is_queued_again() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert!(machine.started(RunId(1), "RunStarted"));
        assert!(machine.finish(RunId(1), RunState::Failed, "Failed"));
        assert!(machine.begin(RunId(2), "submit"));
        assert_eq!(machine.state(), RunState::Queued);
        assert_eq!(machine.run(), Some(RunId(2)));
    }

    #[test]
    fn a_finished_result_goes_stale_once_and_editing_again_cannot_resurrect_it() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert!(machine.started(RunId(1), "RunStarted"));
        assert!(machine.finish(RunId(1), RunState::Finished, "Finished"));
        assert!(machine.stale(RunId(1), "source edit"));
        assert_eq!(machine.state(), RunState::Stale);
        assert!(
            !machine.stale(RunId(1), "second edit"),
            "a second edit must not add another Stale transition"
        );
        assert_eq!(machine.state(), RunState::Stale);
        let stales = machine
            .transitions()
            .iter()
            .filter(|transition| transition.to == RunState::Stale)
            .count();
        assert_eq!(stales, 1);
        let finished_after_stale = machine
            .transitions()
            .iter()
            .skip_while(|transition| transition.to != RunState::Stale)
            .any(|transition| transition.to == RunState::Finished);
        assert!(!finished_after_stale, "Finished must not reappear after Stale");
    }

    #[test]
    fn a_cancelled_result_is_not_marked_stale_by_an_edit() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert!(machine.started(RunId(1), "RunStarted"));
        assert!(machine.cancel(RunId(1), "cancel"));
        assert!(machine.finish(RunId(1), RunState::Cancelled, "Cancelled"));
        assert!(
            !machine.stale(RunId(1), "edit"),
            "Stale is defined for a finished result only"
        );
        assert_eq!(machine.state(), RunState::Cancelled);
    }

    #[test]
    fn reset_returns_to_idle_and_clears_the_tracked_run() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert!(machine.reset("file loaded"));
        assert_eq!(machine.state(), RunState::Idle);
        assert_eq!(machine.run(), None);
        assert!(!machine.reset("again"), "idle reset is a no-op");
        assert_eq!(machine.transitions().last().unwrap().to, RunState::Idle);
    }

    #[test]
    fn duration_between_reports_the_measured_transient_state() {
        let mut machine = machine();
        assert!(machine.begin(RunId(1), "submit"));
        assert!(machine.started(RunId(1), "RunStarted"));
        assert!(machine.finish(RunId(1), RunState::Finished, "Finished"));
        assert!(
            machine
                .duration_between(RunState::Queued, RunState::Running)
                .is_some()
        );
        assert!(
            machine
                .duration_between(RunState::Cancelling, RunState::Cancelled)
                .is_none()
        );
    }
}
