//! Background, per-document execution of sliced Lovelace statements.
//!
//! This is the framework-independent core the desktop UI will drive: it knows nothing about
//! Win32, GDI or the application state. A run executes on a worker thread, publishes typed events
//! through [ExecutionHost::next_event], numbers them on delivery, and terminates exactly once with
//! [DesktopEvent::Finished], [DesktopEvent::Cancelled] or [DesktopEvent::Failed].
//!
//! Guarantees this module owns:
//!
//! - one active run per document, while other documents remain runnable ([SubmitError::Busy]);
//! - one engine instance per run, created on the worker thread from the host's factory, evaluated
//!   once per sliced statement so definitions made earlier persist and `take_output` is drained;
//! - a cooperative cancellation token checked at every statement boundary;
//! - panic isolation: a panicking engine fails only its own run, never the host;
//! - typed events carrying the document, run and source revision, applied through [EventSink],
//!   which ignores results from a stale revision.
//!
//! Diagnostics: [crate::engine::Evaluator] surfaces only a message string, so a position is
//! recovered from the message when the engine names one ("... at position N.") and translated
//! back into the full source; otherwise the diagnostic points at the statement start.

use std::any::Any;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::engine::{EngineState, Evaluator, PlotCapture};
use crate::statements::Statement;

/// Identifies one document buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(pub u64);

/// Identifies one run of one document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunId(pub u64);

/// Identifies the source revision a run was requested for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceRevision(pub u64);

/// Everything the panes need to update together after a run, plus the revision it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    /// Display form of the last value-producing committed statement ("Void" when none did).
    pub result: String,
    /// Variables visible after the committed statements, sorted by the engine.
    pub variables: Vec<(String, String)>,
    /// Functions visible after the committed statements.
    pub functions: Vec<(String, Vec<String>)>,
    /// The richer capture the completion catalogue reads: variables with their kinds and
    /// every function with its declared parameters and builtin flag.
    ///
    /// Taken from [`Evaluator::state`] at the same moment as `variables`/`functions`, so it
    /// describes exactly the committed prefix the panes display.
    pub state: EngineState,
    /// The most recent plot produced by the committed statements.
    pub plot: Option<PlotCapture>,
    /// Every print line produced by the committed statements, in order.
    pub output: Vec<String>,
    /// The source revision this snapshot belongs to.
    pub revision: SourceRevision,
    /// How many statements were committed when the snapshot was taken.
    pub committed_steps: usize,
    /// The failure that ended the run, when it ended in one.
    pub diagnostic: Option<String>,
}

/// A typed notification from a running document.
#[derive(Clone, Debug, PartialEq)]
pub enum DesktopEvent {
    /// A run started; `total` is the number of sliced top-level statements.
    RunStarted {
        /// The document being run.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// The source revision that was submitted.
        revision: SourceRevision,
        /// The number of statements the run will visit.
        total: usize,
    },
    /// A statement is about to be evaluated.
    StepStarted {
        /// The document being run.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// 0-based statement index.
        index: usize,
        /// Short display label for the statement.
        label: String,
    },
    /// A statement was evaluated successfully and its effects are part of the workspace.
    StepCommitted {
        /// The document being run.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// 0-based statement index.
        index: usize,
        /// Whether the value was reused from a cache; this core has no cache, so it is false.
        reused: bool,
    },
    /// One print line produced by a statement.
    Output {
        /// The document being run.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// The printed line.
        text: String,
    },
    /// Progress within the run. The evaluator boundary exposes no finer granularity, so this
    /// reports committed statements over the total.
    SubProgress {
        /// The document being run.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// Committed fraction of the run, from 0.0 to 1.0.
        fraction: f32,
        /// Human-readable progress label.
        label: String,
    },
    /// The run completed normally.
    Finished {
        /// The document that ran.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// The coherent state after the last committed statement.
        snapshot: WorkspaceSnapshot,
    },
    /// The run stopped because its cancellation token was signalled.
    Cancelled {
        /// The document that ran.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// The number of statements committed before cancellation was observed.
        committed_steps: usize,
    },
    /// The run stopped because a statement failed or the engine panicked.
    Failed {
        /// The document that ran.
        document: DocumentId,
        /// The run's identity.
        run: RunId,
        /// The structured failure: message plus its position in the full source.
        diagnostic: Diagnostic,
    },
}

impl DesktopEvent {
    /// The document this event belongs to.
    pub fn document(&self) -> DocumentId {
        match self {
            DesktopEvent::RunStarted { document, .. }
            | DesktopEvent::StepStarted { document, .. }
            | DesktopEvent::StepCommitted { document, .. }
            | DesktopEvent::Output { document, .. }
            | DesktopEvent::SubProgress { document, .. }
            | DesktopEvent::Finished { document, .. }
            | DesktopEvent::Cancelled { document, .. }
            | DesktopEvent::Failed { document, .. } => *document,
        }
    }

    /// The run this event belongs to.
    pub fn run(&self) -> RunId {
        match self {
            DesktopEvent::RunStarted { run, .. }
            | DesktopEvent::StepStarted { run, .. }
            | DesktopEvent::StepCommitted { run, .. }
            | DesktopEvent::Output { run, .. }
            | DesktopEvent::SubProgress { run, .. }
            | DesktopEvent::Finished { run, .. }
            | DesktopEvent::Cancelled { run, .. }
            | DesktopEvent::Failed { run, .. } => *run,
        }
    }
}

/// An event with the delivery-order sequence number and the revision it was produced for.
#[derive(Clone, Debug, PartialEq)]
pub struct SequencedEvent {
    /// Strictly increasing delivery order, starting at 1.
    pub sequence: u64,
    /// The source revision the event belongs to.
    pub revision: SourceRevision,
    /// The typed event.
    pub event: DesktopEvent,
}

/// Why a run could not be submitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmitError {
    /// That document already has an active run.
    Busy,
    /// The worker thread could not be created.
    Spawn(String),
}

/// A position in the full source, attached to an engine failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The engine's message, unchanged.
    pub message: String,
    /// The 0-based index of the statement that failed.
    pub statement_index: usize,
    /// The 1-based line the failing statement starts on.
    pub statement_line: usize,
    /// The absolute byte offset of the failure in the full source.
    pub byte_offset: usize,
    /// The 1-based line of the failure in the full source.
    pub line: usize,
    /// The 1-based column of the failure in the full source.
    pub column: usize,
}

impl Diagnostic {
    /// Builds a diagnostic for a statement, translating an engine position when the message has one.
    ///
    /// The engine reports a position as a **zero-based UTF-16 code-unit offset** inside the source
    /// it was given (`lovelace_lang::LangError::position`). When the message names one (for
    /// example "at position 6.") the code-unit offset is converted to a byte offset inside the
    /// statement and added to the statement's absolute offset. The conversion clamps to the
    /// statement text so an out-of-range position can never point outside the full source. A
    /// message without a position points at the statement start.
    pub fn for_statement(
        statement: &Statement,
        source: &str,
        message: impl Into<String>,
    ) -> Diagnostic {
        let message = message.into();
        let relative = position_in_message(&message)
            .map(|position| byte_offset_utf16(statement.text(), position))
            .unwrap_or(0);
        let byte_offset = statement.byte_offset() + relative;
        let (line, column) = crate::statements::line_column(source, byte_offset);
        Diagnostic {
            message,
            statement_index: statement.index(),
            statement_line: statement.start_line(),
            byte_offset,
            line,
            column,
        }
    }

    /// Renders the diagnostic for a [WorkspaceSnapshot] and the log pane.
    pub fn render(&self) -> String {
        format!(
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

/// Converts a zero-based UTF-16 code-unit offset into a byte offset in `text`.
///
/// An offset inside a surrogate pair clamps back to the start of that code point; an offset past
/// the end clamps to the text length.
fn byte_offset_utf16(text: &str, units: usize) -> usize {
    let mut consumed = 0usize;
    for (byte_index, character) in text.char_indices() {
        if consumed >= units {
            return byte_index;
        }
        let next = consumed + character.len_utf16();
        if units < next {
            return byte_index;
        }
        consumed = next;
    }
    text.len()
}

/// The first decimal number following "position" in an engine message, when there is one.
fn position_in_message(message: &str) -> Option<usize> {
    let marker = "position";
    let after_marker = message.find(marker)? + marker.len();
    let digits = message[after_marker..]
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// Creates one fresh evaluator for one run. It is called on the worker thread.
pub type EvaluatorFactory = Box<dyn Fn() -> Box<dyn Evaluator> + Send + Sync>;

struct RawEvent {
    revision: SourceRevision,
    event: DesktopEvent,
}

#[derive(Debug)]
struct RunControl {
    cancelled: AtomicBool,
    finished: AtomicBool,
}

impl RunControl {
    fn new() -> RunControl {
        RunControl {
            cancelled: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

#[derive(Default)]
struct HostState {
    active: HashMap<DocumentId, (RunId, Arc<RunControl>)>,
}

/// A submitted run: its identity, its revision and its cancellation token.
#[derive(Clone)]
pub struct RunHandle {
    document: DocumentId,
    run: RunId,
    /// The submitted revision, exposed through [RunHandle::revision] for hosts that label runs;
    /// the execution core itself does not need to read it back.
    #[allow(dead_code)]
    revision: SourceRevision,
    control: Arc<RunControl>,
    /// The live flag this run's engine polls inside a long kernel. [`ExecutionHost::cancel`] sets
    /// it together with the statement-boundary token, so one kernel that never reaches a
    /// statement boundary still stops.
    cancel_flag: Arc<AtomicBool>,
}

impl RunHandle {
    /// The document this handle belongs to.
    #[allow(dead_code)] // host-facing accessor used by the execution tests
    pub fn document(&self) -> DocumentId {
        self.document
    }

    /// The run's identity.
    pub fn run(&self) -> RunId {
        self.run
    }

    /// The source revision the run was submitted for.
    #[allow(dead_code)] // part of the host-facing handle API; the core tracks the revision itself
    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    /// True while the run has not delivered its terminal event.
    pub fn is_active(&self) -> bool {
        !self.control.is_finished()
    }
}

/// Owns the worker threads and the event queue for every document.
pub struct ExecutionHost {
    factory: Arc<dyn Fn() -> Box<dyn Evaluator> + Send + Sync>,
    sender: Sender<RawEvent>,
    receiver: Mutex<Receiver<RawEvent>>,
    sequence: AtomicU64,
    state: Arc<Mutex<HostState>>,
    next_run: AtomicU64,
    snapshots: Arc<Mutex<HashMap<DocumentId, WorkspaceSnapshot>>>,
}

impl ExecutionHost {
    /// Creates a host with no active runs.
    pub fn new(factory: EvaluatorFactory) -> ExecutionHost {
        let (sender, receiver) = mpsc::channel();
        ExecutionHost {
            factory: Arc::from(factory),
            sender,
            receiver: Mutex::new(receiver),
            sequence: AtomicU64::new(0),
            state: Arc::new(Mutex::new(HostState::default())),
            next_run: AtomicU64::new(0),
            snapshots: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Submits a run for one document.
    ///
    /// Returns [SubmitError::Busy] when that document already has an active run; other documents
    /// are unaffected. The call only records the run and spawns its worker; it never evaluates
    /// source, so a slow engine cannot block the caller.
    pub fn submit(
        &self,
        document: DocumentId,
        revision: SourceRevision,
        source: String,
        precision: usize,
    ) -> Result<RunHandle, SubmitError> {
        crate::run_trace::mark("submit_enter", format!("document={}", document.0));
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.active.contains_key(&document) {
            return Err(SubmitError::Busy);
        }
        let run = RunId(self.next_run.fetch_add(1, Ordering::SeqCst) + 1);
        let control = Arc::new(RunControl::new());
        // One live flag per run: the worker gives it to its engine, the handle keeps a clone so
        // `cancel()` can set it from the UI thread while the kernel is running.
        let cancel_flag = Arc::new(AtomicBool::new(false));
        state.active.insert(document, (run, Arc::clone(&control)));
        drop(state);

        let statements = crate::statements::slice(&source);
        let worker = RunWorker {
            factory: Arc::clone(&self.factory),
            sender: self.sender.clone(),
            state: Arc::clone(&self.state),
            snapshots: Arc::clone(&self.snapshots),
            document,
            run,
            revision,
            source,
            precision,
            statements,
            control: Arc::clone(&control),
            cancel_flag: Arc::clone(&cancel_flag),
        };
        let spawned = thread::Builder::new()
            .name(format!("lovelace-run-{}-{}", document.0, run.0))
            .spawn(move || worker.run());
        match spawned {
            Ok(_detached) => {
                crate::run_trace::mark(
                    "submit_exit",
                    format!("document={} run={}", document.0, run.0),
                );
                Ok(RunHandle {
                    document,
                    run,
                    revision,
                    control,
                    cancel_flag,
                })
            }
            Err(error) => {
                let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
                if let Some((active_run, _)) = state.active.get(&document) {
                    if *active_run == run {
                        state.active.remove(&document);
                    }
                }
                control.finished.store(true, Ordering::Release);
                Err(SubmitError::Spawn(error.to_string()))
            }
        }
    }

    /// Requests cancellation of a run.
    ///
    /// Sets both cancellation seams: the statement-boundary token and the live flag the engine
    /// polls inside a long kernel. Returns true when the token was accepted, false when the run
    /// already finished or was already cancelled.
    pub fn cancel(&self, handle: &RunHandle) -> bool {
        if handle.control.is_finished() {
            return false;
        }
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        match state.active.get(&handle.document) {
            Some((run, control))
                if *run == handle.run && !control.is_finished() && !control.is_cancelled() =>
            {
                control.cancelled.store(true, Ordering::Release);
                handle.cancel_flag.store(true, Ordering::Release);
                true
            }
            _ => false,
        }
    }

    /// Waits up to `timeout` for the next event and assigns it a delivery sequence number.
    pub fn next_event(&self, timeout: Duration) -> Option<SequencedEvent> {
        let receiver = self
            .receiver
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match receiver.recv_timeout(timeout) {
            Ok(raw) => {
                let sequence = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
                Some(SequencedEvent {
                    sequence,
                    revision: raw.revision,
                    event: raw.event,
                })
            }
            Err(_) => None,
        }
    }

    /// The last snapshot published for a document, if any run reached a terminal state.
    pub fn last_snapshot(&self, document: DocumentId) -> Option<WorkspaceSnapshot> {
        let snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        snapshots.get(&document).cloned()
    }
}

/// Everything one worker accumulates while it visits the sliced statements.
struct Progress {
    /// Index of the statement currently being evaluated.
    current: usize,
    /// Statements whose effects are part of the workspace.
    committed: usize,
    /// Display form of the last committed value ("Void" before any).
    result: String,
    /// Print lines drained from the engine, in order.
    output: Vec<String>,
    /// Variables observed after the last committed statement.
    variables: Vec<(String, String)>,
    /// Functions observed after the last committed statement.
    functions: Vec<(String, Vec<String>)>,
    /// The engine's kind-aware symbol capture for the last committed statement.
    state: EngineState,
    /// Most recent plot observed during the run. A statement that produces no plot does not
    /// erase an earlier one: the real engine clears its own capture at the start of every
    /// `eval`, and this core evaluates one statement at a time.
    plot: Option<PlotCapture>,
    /// True once a terminal event has been published for the run.
    ended: bool,
}

impl Default for Progress {
    fn default() -> Progress {
        Progress {
            current: 0,
            committed: 0,
            result: "Void".to_string(),
            output: Vec::new(),
            variables: Vec::new(),
            functions: Vec::new(),
            state: EngineState::default(),
            plot: None,
            ended: false,
        }
    }
}

/// Owns one run on its worker thread.
struct RunWorker {
    factory: Arc<dyn Fn() -> Box<dyn Evaluator> + Send + Sync>,
    sender: Sender<RawEvent>,
    state: Arc<Mutex<HostState>>,
    snapshots: Arc<Mutex<HashMap<DocumentId, WorkspaceSnapshot>>>,
    document: DocumentId,
    run: RunId,
    revision: SourceRevision,
    source: String,
    precision: usize,
    statements: Vec<Statement>,
    control: Arc<RunControl>,
    /// The live flag handed to the engine for this run; [`ExecutionHost::cancel`] sets the other
    /// clone.
    cancel_flag: Arc<AtomicBool>,
}

impl RunWorker {
    /// Runs the whole script, turning every exit path into exactly one terminal event.
    fn run(self) {
        crate::run_trace::mark(
            "worker_start",
            format!("document={} run={}", self.document.0, self.run.0),
        );
        let mut progress = Progress::default();
        match panic::catch_unwind(AssertUnwindSafe(|| self.execute(&mut progress))) {
            Ok(()) => {}
            Err(payload) => {
                if !progress.ended {
                    let diagnostic = self.diagnostic_for(progress.current, panic_text(payload));
                    self.publish(self.snapshot(&progress, Some(diagnostic.render())));
                    self.retire();
                    self.emit(DesktopEvent::Failed {
                        document: self.document,
                        run: self.run,
                        diagnostic,
                    });
                }
            }
        }
    }

    /// True when either cancellation seam has been signalled for this run.
    fn cancel_requested(&self) -> bool {
        self.control.is_cancelled() || self.cancel_flag.load(Ordering::Acquire)
    }

    /// Ends the run as cancelled: publishes the committed snapshot, retires the document and
    /// emits exactly one terminal `Cancelled` event.
    fn finish_cancelled(&self, progress: &mut Progress) {
        let snapshot = self.snapshot(progress, None);
        self.publish(snapshot);
        self.retire();
        self.emit(DesktopEvent::Cancelled {
            document: self.document,
            run: self.run,
            committed_steps: progress.committed,
        });
        progress.ended = true;
    }

    /// Evaluates every statement, publishing typed events on the way.
    fn execute(&self, progress: &mut Progress) {
        crate::run_trace::mark("engine_build_enter", "engine factory");
        let engine = match panic::catch_unwind(AssertUnwindSafe(|| (self.factory)())) {
            Ok(engine) => engine,
            Err(payload) => {
                let diagnostic = self.diagnostic_for(progress.current, panic_text(payload));
                self.fail(progress, diagnostic);
                return;
            }
        };
        crate::run_trace::mark("engine_constructed", "engine factory returned");
        let mut engine = engine;
        let _ = panic::catch_unwind(AssertUnwindSafe(|| engine.set_precision(self.precision)));
        // The live kernel seam: the engine keeps this handle for the whole run and its kernels
        // poll it beside their deadline, so Cancel does not have to wait for a statement boundary.
        let cancel_flag = Arc::clone(&self.cancel_flag);
        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
            engine.set_cancel_flag(Some(cancel_flag));
        }));
        self.emit(DesktopEvent::RunStarted {
            document: self.document,
            run: self.run,
            revision: self.revision,
            total: self.statements.len(),
        });
        crate::run_trace::mark(
            "run_started_emitted",
            format!(
                "total={} document={}",
                self.statements.len(),
                self.document.0
            ),
        );

        for (index, statement) in self.statements.iter().enumerate() {
            progress.current = index;
            if self.cancel_requested() {
                self.finish_cancelled(progress);
                return;
            }
            self.emit(DesktopEvent::StepStarted {
                document: self.document,
                run: self.run,
                index,
                label: statement.label(),
            });
            match panic::catch_unwind(AssertUnwindSafe(|| engine.eval(statement.text()))) {
                Ok(Ok(value)) => {
                    progress.result = value;
                    self.drain_output(&mut engine, progress);
                    progress.committed += 1;
                    if let Ok(variables) =
                        panic::catch_unwind(AssertUnwindSafe(|| engine.variables()))
                    {
                        progress.variables = variables;
                    }
                    if let Ok(functions) =
                        panic::catch_unwind(AssertUnwindSafe(|| engine.functions()))
                    {
                        progress.functions = functions;
                    }
                    // The kind-aware capture the completion catalogue reads. It is taken at the
                    // same commit point as the pane lists, so the popup can never offer a
                    // variable the displayed workspace does not have.
                    if let Ok(Some(state)) =
                        panic::catch_unwind(AssertUnwindSafe(|| engine.state()))
                    {
                        progress.state = state;
                    }
                    if let Ok(Some(plot)) =
                        panic::catch_unwind(AssertUnwindSafe(|| engine.last_plot()))
                    {
                        // Keep the last plot the run produced. The engine resets its capture
                        // at the start of each `eval`, so a later statement legitimately
                        // reports None; that must not erase the plot from an earlier one.
                        progress.plot = Some(plot);
                    }
                    self.emit(DesktopEvent::StepCommitted {
                        document: self.document,
                        run: self.run,
                        index,
                        reused: false,
                    });
                    let total = self.statements.len().max(1);
                    self.emit(DesktopEvent::SubProgress {
                        document: self.document,
                        run: self.run,
                        fraction: progress.committed as f32 / total as f32,
                        label: format!("{}/{}", progress.committed, self.statements.len()),
                    });
                }
                Ok(Err(message)) => {
                    self.drain_output(&mut engine, progress);
                    // The engine reports a kernel that observed the live flag as an ordinary
                    // statement error; the host knows the run was cancelled, so publish the
                    // terminal `Cancelled` the caller requested instead of a failure.
                    if self.cancel_requested() {
                        self.finish_cancelled(progress);
                        return;
                    }
                    let diagnostic = Diagnostic::for_statement(statement, &self.source, message);
                    self.fail(progress, diagnostic);
                    return;
                }
                Err(payload) => {
                    let diagnostic =
                        Diagnostic::for_statement(statement, &self.source, panic_text(payload));
                    self.fail(progress, diagnostic);
                    return;
                }
            }
        }

        let snapshot = self.snapshot(progress, None);
        self.publish(snapshot.clone());
        self.retire();
        self.emit(DesktopEvent::Finished {
            document: self.document,
            run: self.run,
            snapshot,
        });
        progress.ended = true;
    }

    /// Drains the engine's print output and publishes one event per line.
    fn drain_output(&self, engine: &mut Box<dyn Evaluator>, progress: &mut Progress) {
        let lines =
            panic::catch_unwind(AssertUnwindSafe(|| engine.take_output())).unwrap_or_default();
        for line in lines {
            self.emit(DesktopEvent::Output {
                document: self.document,
                run: self.run,
                text: line.clone(),
            });
            progress.output.push(line);
        }
    }

    /// Publishes a final snapshot and the single Failed event for the run.
    fn fail(&self, progress: &mut Progress, diagnostic: Diagnostic) {
        self.publish(self.snapshot(progress, Some(diagnostic.render())));
        self.retire();
        self.emit(DesktopEvent::Failed {
            document: self.document,
            run: self.run,
            diagnostic,
        });
        progress.ended = true;
    }

    /// Builds the coherent snapshot for the committed prefix plus an optional diagnostic.
    fn snapshot(&self, progress: &Progress, diagnostic: Option<String>) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            result: progress.result.clone(),
            variables: progress.variables.clone(),
            functions: progress.functions.clone(),
            state: progress.state.clone(),
            plot: progress.plot.clone(),
            output: progress.output.clone(),
            revision: self.revision,
            committed_steps: progress.committed,
            diagnostic,
        }
    }

    /// Builds a diagnostic at `index`, or at the start of the source when there is no statement.
    fn diagnostic_for(&self, index: usize, message: String) -> Diagnostic {
        match self.statements.get(index) {
            Some(statement) => Diagnostic::for_statement(statement, &self.source, message),
            None => Diagnostic {
                message,
                statement_index: index,
                statement_line: 1,
                byte_offset: 0,
                line: 1,
                column: 1,
            },
        }
    }

    /// Removes the run from the host's active map and marks its handle inactive.
    fn retire(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some((run, _)) = state.active.get(&self.document) {
            if *run == self.run {
                state.active.remove(&self.document);
            }
        }
        drop(state);
        self.control.finished.store(true, Ordering::Release);
    }

    fn publish(&self, snapshot: WorkspaceSnapshot) {
        let mut snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        snapshots.insert(self.document, snapshot);
    }

    fn emit(&self, event: DesktopEvent) {
        let _ = self.sender.send(RawEvent {
            revision: self.revision,
            event,
        });
    }
}

/// Extracts the text out of a panic payload.
fn panic_text(payload: Box<dyn Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "engine panicked".to_string()
    }
}

/// Applies sequenced events to the workspace, ignoring results from a stale revision.
#[derive(Clone, Debug, Default)]
pub struct EventSink {
    revision: SourceRevision,
    snapshot: Option<WorkspaceSnapshot>,
}

impl EventSink {
    /// Creates an empty sink at revision zero.
    pub fn new() -> EventSink {
        EventSink::default()
    }

    /// Applies one event. Returns true when it was accepted, false when it was stale.
    ///
    /// Only [DesktopEvent::Finished] replaces the snapshot and advances the sink's revision; an
    /// event whose revision is older than the applied revision never changes the sink.
    pub fn apply(&mut self, event: &SequencedEvent) -> bool {
        if event.revision < self.revision {
            return false;
        }
        if let DesktopEvent::Finished { snapshot, .. } = &event.event {
            if snapshot.revision < self.revision {
                return false;
            }
            self.revision = snapshot.revision;
            self.snapshot = Some(snapshot.clone());
        }
        true
    }

    /// The revision of the applied snapshot.
    #[allow(dead_code)] // host-facing accessor used by the execution tests
    pub fn revision(&self) -> SourceRevision {
        self.revision
    }

    /// The applied snapshot, if a non-stale run finished.
    #[allow(dead_code)] // host-facing accessor used by the execution tests
    pub fn snapshot(&self) -> Option<&WorkspaceSnapshot> {
        self.snapshot.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Instant;

    /// An engine whose every statement sleeps, so submission and cancellation timing are visible.
    struct SleepEngine {
        delay: Duration,
        calls: usize,
    }

    impl Evaluator for SleepEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            thread::sleep(self.delay);
            self.calls += 1;
            Ok(format!("v{}", self.calls))
        }
    }

    fn sleep_factory(delay: Duration) -> EvaluatorFactory {
        Box::new(move || Box::new(SleepEngine { delay, calls: 0 }))
    }

    /// An engine that reports its executed-statement count as a variable, so a snapshot can be
    /// compared against the committed step count.
    struct ProgressEngine {
        delay: Duration,
        steps: usize,
    }

    impl Evaluator for ProgressEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            thread::sleep(self.delay);
            self.steps += 1;
            Ok(self.steps.to_string())
        }

        fn variables(&self) -> Vec<(String, String)> {
            vec![("step".to_string(), self.steps.to_string())]
        }
    }

    fn progress_factory(delay: Duration) -> EvaluatorFactory {
        Box::new(move || Box::new(ProgressEngine { delay, steps: 0 }))
    }

    /// Panics on the first evaluation that reaches it, then succeeds. The flag is shared across
    /// engine instances, so the second run on the same document is healthy.
    struct PanicOnceEngine {
        panicked: Arc<AtomicBool>,
    }

    impl Evaluator for PanicOnceEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            if !self.panicked.swap(true, Ordering::SeqCst) {
                panic!("boom");
            }
            Ok("ok".to_string())
        }
    }

    /// Produces two print lines per evaluation.
    #[derive(Default)]
    struct PrintingEngine {
        pending: Vec<String>,
    }

    impl Evaluator for PrintingEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            self.pending.push("first".to_string());
            self.pending.push("second".to_string());
            Ok("done".to_string())
        }

        fn take_output(&mut self) -> Vec<String> {
            std::mem::take(&mut self.pending)
        }
    }

    /// Counts evaluations and returns a distinct display value for each.
    #[derive(Default)]
    struct CountingEngine {
        calls: usize,
    }

    impl Evaluator for CountingEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            self.calls += 1;
            Ok(format!("v{}", self.calls))
        }
    }

    fn counting_factory() -> EvaluatorFactory {
        Box::new(|| Box::new(CountingEngine::default()))
    }

    fn event_summary(event: &DesktopEvent) -> String {
        match event {
            DesktopEvent::RunStarted { total, .. } => format!("RunStarted({total})"),
            DesktopEvent::StepStarted { index, .. } => format!("StepStarted({index})"),
            DesktopEvent::StepCommitted { index, .. } => format!("StepCommitted({index})"),
            DesktopEvent::Output { text, .. } => format!("Output({text:?})"),
            DesktopEvent::SubProgress { .. } => "SubProgress".to_string(),
            DesktopEvent::Finished { .. } => "Finished".to_string(),
            DesktopEvent::Cancelled {
                committed_steps, ..
            } => {
                format!("Cancelled({committed_steps})")
            }
            DesktopEvent::Failed { diagnostic, .. } => format!("Failed({})", diagnostic.render()),
        }
    }

    fn summaries(events: &[SequencedEvent]) -> Vec<String> {
        events
            .iter()
            .map(|event| event_summary(&event.event))
            .collect()
    }

    /// Reads events until a terminal one arrives, failing with the collected names on timeout.
    fn drain_until_terminal(host: &ExecutionHost, timeout: Duration) -> Vec<SequencedEvent> {
        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                panic!(
                    "timed out waiting for a terminal event; collected {events:?}",
                    events = summaries(&events)
                );
            }
            match host.next_event(remaining) {
                Some(event) => {
                    let terminal = matches!(
                        event.event,
                        DesktopEvent::Finished { .. }
                            | DesktopEvent::Cancelled { .. }
                            | DesktopEvent::Failed { .. }
                    );
                    events.push(event);
                    if terminal {
                        return events;
                    }
                }
                None => panic!(
                    "next_event returned None before a terminal event; collected {events:?}",
                    events = summaries(&events)
                ),
            }
        }
    }

    fn test_snapshot(result: &str, revision: SourceRevision) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            result: result.to_string(),
            variables: vec![("x".to_string(), result.to_string())],
            functions: Vec::new(),
            state: EngineState {
                variables: vec![crate::engine::EngineVariable {
                    name: "x".to_string(),
                    kind: "Natural".to_string(),
                    display: result.to_string(),
                }],
                functions: Vec::new(),
            },
            plot: None,
            output: vec![result.to_string()],
            revision,
            committed_steps: 1,
            diagnostic: None,
        }
    }

    fn sequenced(sequence: u64, revision: SourceRevision, event: DesktopEvent) -> SequencedEvent {
        SequencedEvent {
            sequence,
            revision,
            event,
        }
    }

    #[test]
    fn t2_submit_does_not_block_and_run_started_arrives_quickly() {
        let host = ExecutionHost::new(sleep_factory(Duration::from_millis(300)));
        let started = Instant::now();
        let handle = host
            .submit(
                DocumentId(1),
                SourceRevision(7),
                "a = 1\nb = 2\n".to_string(),
                1000,
            )
            .expect("submit must accept an idle document");
        let submit_elapsed = started.elapsed();
        assert!(
            submit_elapsed < Duration::from_millis(100),
            "submit took {submit_elapsed:?}, expected under 100 ms"
        );
        eprintln!("T2 submit returned in {submit_elapsed:?}");

        let first = host
            .next_event(Duration::from_millis(100))
            .unwrap_or_else(|| panic!("RunStarted was not available within 100 ms of submit"));
        eprintln!(
            "T2 RunStarted received {:?} after submit",
            started.elapsed()
        );
        match &first.event {
            DesktopEvent::RunStarted {
                document,
                run,
                revision,
                total,
            } => {
                assert_eq!(*document, DocumentId(1));
                assert_eq!(*run, handle.run());
                assert_eq!(*revision, SourceRevision(7));
                assert_eq!(*total, 2);
            }
            other => panic!(
                "first event was {}, expected RunStarted",
                event_summary(other)
            ),
        }

        assert!(
            host.cancel(&handle),
            "cancel must be accepted on an active run"
        );
        let events = drain_until_terminal(&host, Duration::from_secs(2));
        assert!(
            matches!(
                events.last().map(|event| &event.event),
                Some(DesktopEvent::Cancelled { .. })
            ),
            "terminal event was {:?}, expected Cancelled",
            summaries(&events)
        );
    }

    #[test]
    fn t3_events_are_sequenced_and_ordered_with_run_identity() {
        let host = ExecutionHost::new(counting_factory());
        let handle = host
            .submit(
                DocumentId(4),
                SourceRevision(9),
                "a = 1\nb = 2\nc = 3\n".to_string(),
                1000,
            )
            .expect("submit");
        let events = drain_until_terminal(&host, Duration::from_secs(5));
        assert_eq!(
            events.len(),
            11,
            "expected RunStarted + 3*(StepStarted, StepCommitted, SubProgress) + Finished; got {:?}",
            summaries(&events)
        );

        let mut ordered = Vec::new();
        for event in &events {
            assert_eq!(
                event.event.document(),
                DocumentId(4),
                "wrong document on {}",
                event_summary(&event.event)
            );
            assert_eq!(
                event.event.run(),
                handle.run(),
                "wrong run on {}",
                event_summary(&event.event)
            );
            assert_eq!(
                event.revision,
                SourceRevision(9),
                "wrong revision on {}",
                event_summary(&event.event)
            );
            if !matches!(event.event, DesktopEvent::SubProgress { .. }) {
                ordered.push(event_summary(&event.event));
            }
        }
        assert_eq!(
            ordered,
            vec![
                "RunStarted(3)",
                "StepStarted(0)",
                "StepCommitted(0)",
                "StepStarted(1)",
                "StepCommitted(1)",
                "StepStarted(2)",
                "StepCommitted(2)",
                "Finished",
            ]
        );
        for pair in events.windows(2) {
            assert!(
                pair[0].sequence < pair[1].sequence,
                "sequence did not increase: {} then {}",
                pair[0].sequence,
                pair[1].sequence
            );
        }

        let finished = match &events.last().unwrap().event {
            DesktopEvent::Finished { snapshot, .. } => snapshot,
            other => panic!("terminal was {}, expected Finished", event_summary(other)),
        };
        assert_eq!(finished.result, "v3");
        assert_eq!(finished.committed_steps, 3);
    }

    #[test]
    fn t4_cancellation_stops_a_real_run_and_keeps_the_committed_snapshot() {
        let host = ExecutionHost::new(progress_factory(Duration::from_millis(20)));
        let source = (0..200)
            .map(|index| format!("step = {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let handle = host
            .submit(DocumentId(2), SourceRevision(1), source, 1000)
            .expect("submit");
        let mut events = Vec::new();
        loop {
            match host.next_event(Duration::from_secs(5)) {
                Some(event) => {
                    let committed = matches!(event.event, DesktopEvent::StepCommitted { .. });
                    events.push(event);
                    if committed {
                        break;
                    }
                }
                None => panic!(
                    "no StepCommitted event within 5 s; collected {:?}",
                    summaries(&events)
                ),
            }
        }

        let cancel_sent = Instant::now();
        assert!(
            host.cancel(&handle),
            "cancel must be accepted while the run is active"
        );
        loop {
            match host.next_event(Duration::from_secs(2)) {
                Some(event) => {
                    let terminal = matches!(
                        event.event,
                        DesktopEvent::Finished { .. }
                            | DesktopEvent::Cancelled { .. }
                            | DesktopEvent::Failed { .. }
                    );
                    events.push(event);
                    if terminal {
                        break;
                    }
                }
                None => panic!(
                    "no terminal event within 2 s after cancel; collected {:?}",
                    summaries(&events)
                ),
            }
        }
        let elapsed = cancel_sent.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "cancellation took {elapsed:?}, expected under 2 s"
        );
        eprintln!("T4 cancel-to-terminal took {elapsed:?}");

        let terminal = events.last().unwrap();
        match &terminal.event {
            DesktopEvent::Cancelled {
                document,
                run,
                committed_steps,
            } => {
                assert_eq!(*document, DocumentId(2));
                assert_eq!(*run, handle.run());
                assert!(
                    *committed_steps < 200,
                    "committed_steps was {committed_steps}, expected fewer than 200"
                );
                assert!(*committed_steps >= 1, "expected at least the first step");
                let snapshot = host
                    .last_snapshot(DocumentId(2))
                    .expect("a cancelled run must publish its last snapshot");
                assert_eq!(snapshot.committed_steps, *committed_steps);
                assert_eq!(
                    snapshot.variables,
                    vec![("step".to_string(), committed_steps.to_string())],
                    "snapshot state must contain exactly the committed statements"
                );
                assert_eq!(snapshot.result, committed_steps.to_string());
                assert_eq!(snapshot.revision, SourceRevision(1));
            }
            other => panic!(
                "terminal after cancel was {}, expected Cancelled",
                event_summary(other)
            ),
        }
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.event, DesktopEvent::Finished { .. }))
                .count(),
            0,
            "a cancelled run must not also finish: {:?}",
            summaries(&events)
        );
    }

    /// The actual desktop defect: Cancel while the worker is inside ONE kernel call.
    ///
    /// `matmul(eye(400), eye(400))` is a single statement whose whole cost is one array kernel
    /// (a triple loop that polls the cancellation seam); there is no statement boundary to
    /// observe for many seconds. Cancel must reach the engine's live flag and the run must
    /// publish a terminal `Cancelled` inside the two-second budget. The measured latency is
    /// printed. A single opaque primitive with no poll seam (`sqrt(2)` at precision 200000) is
    /// budgeted as uninterruptible and documented in `docs/goal-desktop-v4/round-9/kernel-cancel.md`.
    #[cfg(feature = "real-engine")]
    #[test]
    fn t9_cancel_inside_one_kernel_is_terminal_within_two_seconds() {
        let host = ExecutionHost::new(Box::new(|| {
            Box::new(crate::engine::RealEngine::new()) as Box<dyn Evaluator>
        }));
        let document = DocumentId(9);
        let handle = host
            .submit(
                document,
                SourceRevision(1),
                "matmul(eye(400), eye(400))".to_string(),
                1000,
            )
            .expect("submit");
        // Wait for the worker to enter the kernel statement (the only statement), then give the
        // kernel 200 ms so Cancel cannot win at a boundary the run does not reach anyway.
        let mut events = Vec::new();
        loop {
            let event = host.next_event(Duration::from_secs(5)).unwrap_or_else(|| {
                panic!(
                    "no StepStarted event for the kernel statement; collected {:?}",
                    summaries(&events)
                )
            });
            let entered_kernel =
                matches!(event.event, DesktopEvent::StepStarted { index: 0, .. });
            events.push(event);
            if entered_kernel {
                break;
            }
        }
        thread::sleep(Duration::from_millis(200));
        assert!(
            handle.is_active(),
            "the one long kernel finished before it could be cancelled"
        );

        let cancel_sent = Instant::now();
        assert!(
            host.cancel(&handle),
            "cancel must be accepted while the kernel is running"
        );
        let deadline = cancel_sent + Duration::from_secs(2);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                panic!(
                    "no terminal event within 2 s after cancel; collected {:?}",
                    summaries(&events)
                );
            }
            match host.next_event(remaining) {
                Some(event) => {
                    let terminal = matches!(
                        event.event,
                        DesktopEvent::Finished { .. }
                            | DesktopEvent::Cancelled { .. }
                            | DesktopEvent::Failed { .. }
                    );
                    events.push(event);
                    if terminal {
                        break;
                    }
                }
                None => panic!(
                    "no terminal event within 2 s after cancel; collected {:?}",
                    summaries(&events)
                ),
            }
        }
        let elapsed = cancel_sent.elapsed();
        eprintln!(
            "T9 single-kernel cancel-to-terminal took {:.3} ms",
            elapsed.as_secs_f64() * 1000.0
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "single-kernel cancellation took {elapsed:?}, expected under 2 s"
        );
        let terminal = events.last().expect("a terminal event was appended");
        match &terminal.event {
            DesktopEvent::Cancelled {
                document: stopped,
                run,
                committed_steps,
            } => {
                assert_eq!(*stopped, document);
                assert_eq!(*run, handle.run());
                assert_eq!(*committed_steps, 0, "the one kernel statement did not commit");
                let snapshot = host
                    .last_snapshot(document)
                    .expect("a cancelled run must publish its last snapshot");
                assert_eq!(snapshot.committed_steps, 0);
                assert_eq!(snapshot.result, "Void");
                assert_eq!(snapshot.revision, SourceRevision(1));
            }
            other => panic!(
                "terminal after cancel was {}, expected Cancelled; collected {:?}",
                event_summary(other),
                summaries(&events)
            ),
        }
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.event, DesktopEvent::Finished { .. }))
                .count(),
            0,
            "a cancelled run must not also finish: {:?}",
            summaries(&events)
        );
    }

    #[test]
    fn t5_duplicate_submit_is_busy_and_other_documents_run_concurrently() {
        let host = ExecutionHost::new(sleep_factory(Duration::from_millis(300)));
        let doc1 = DocumentId(1);
        let doc2 = DocumentId(2);
        let handle1 = host
            .submit(
                doc1,
                SourceRevision(3),
                "a = 1\nb = 2\nc = 3\nd = 4\ne = 5\n".to_string(),
                1000,
            )
            .expect("first submit for document 1");
        match host.submit(doc1, SourceRevision(4), "x = 1".to_string(), 1000) {
            Err(SubmitError::Busy) => {}
            Err(other) => {
                panic!("second submit for document 1 failed with {other:?}, expected Busy")
            }
            Ok(_) => panic!("second submit for document 1 succeeded, expected Busy"),
        }

        let handle2 = host
            .submit(doc2, SourceRevision(3), "y = 1\n".to_string(), 1000)
            .expect("document 2 must stay runnable while document 1 runs");
        assert!(
            handle1.is_active(),
            "document 1 must still be active when document 2 starts"
        );

        let finished2_at = loop {
            match host.next_event(Duration::from_secs(5)) {
                Some(event) => match &event.event {
                    DesktopEvent::Finished { document, .. } if *document == doc2 => {
                        break Instant::now();
                    }
                    DesktopEvent::Finished { document, .. } => {
                        panic!("document {document:?} finished before document 2")
                    }
                    _ => {}
                },
                None => panic!("document 2 did not finish within 5 s"),
            }
        };
        assert!(
            handle1.is_active(),
            "document 1 must still be running when document 2 finishes"
        );
        let cancel_at = Instant::now();
        assert!(
            finished2_at < cancel_at,
            "document 2's Finished ({finished2_at:?}) must precede document 1's cancellation ({cancel_at:?})"
        );
        assert!(
            host.cancel(&handle1),
            "document 1 must be cancellable after document 2 finished"
        );

        let events = drain_until_terminal(&host, Duration::from_secs(2));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.event, DesktopEvent::Finished { .. }))
                .count(),
            0,
            "document 1 must not have finished before cancellation: {:?}",
            summaries(&events)
        );
        match &events.last().unwrap().event {
            DesktopEvent::Cancelled {
                document,
                run,
                committed_steps,
            } => {
                assert_eq!(*document, doc1);
                assert_eq!(*run, handle1.run());
                assert!(
                    *committed_steps < 5,
                    "committed_steps was {committed_steps}"
                );
            }
            other => panic!(
                "terminal for document 1 was {}, expected Cancelled",
                event_summary(other)
            ),
        }
        assert_eq!(handle2.document(), doc2);
    }

    #[test]
    fn t6_panicking_engine_fails_one_run_and_the_host_stays_usable() {
        let panicked = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&panicked);
        let host = ExecutionHost::new(Box::new(move || {
            Box::new(PanicOnceEngine {
                panicked: Arc::clone(&flag),
            })
        }));
        let doc = DocumentId(9);
        let _first = host
            .submit(doc, SourceRevision(1), "a = 1\nb = 2\n".to_string(), 1000)
            .expect("first submit");
        let events = drain_until_terminal(&host, Duration::from_secs(5));
        let diagnostic = match &events.last().unwrap().event {
            DesktopEvent::Failed { diagnostic, .. } => diagnostic.clone(),
            other => panic!(
                "first run terminal was {}, expected Failed",
                event_summary(other)
            ),
        };
        assert!(
            diagnostic.message.contains("boom"),
            "diagnostic must carry the panic message: {diagnostic:?}"
        );
        assert!(
            diagnostic.render().contains("line 1"),
            "diagnostic must point into the full source: {diagnostic:?}"
        );
        let after_panic = host
            .last_snapshot(doc)
            .expect("a failed run still records its last committed snapshot");
        assert_eq!(after_panic.committed_steps, 0);

        let _second = host
            .submit(doc, SourceRevision(1), "a = 1\nb = 2\n".to_string(), 1000)
            .expect("the host must accept a new run on the same document after a panic");
        let events = drain_until_terminal(&host, Duration::from_secs(5));
        match &events.last().unwrap().event {
            DesktopEvent::Finished { snapshot, .. } => {
                assert_eq!(snapshot.result, "ok");
                assert_eq!(snapshot.committed_steps, 2);
            }
            other => panic!(
                "second run terminal was {}, expected Finished",
                event_summary(other)
            ),
        }
    }

    #[test]
    fn t7_stale_finished_events_are_ignored_by_the_sink() {
        let document = DocumentId(1);
        let run = RunId(1);
        let mut sink = EventSink::new();
        assert_eq!(sink.revision(), SourceRevision(0));

        let applied = sink.apply(&sequenced(
            1,
            SourceRevision(2),
            DesktopEvent::Finished {
                document,
                run,
                snapshot: test_snapshot("new", SourceRevision(2)),
            },
        ));
        assert!(applied, "the first Finished must be applied");
        assert_eq!(sink.revision(), SourceRevision(2));
        let state_before = sink.snapshot().cloned();
        assert_eq!(
            state_before
                .as_ref()
                .map(|snapshot| snapshot.result.as_str()),
            Some("new")
        );

        let stale = sink.apply(&sequenced(
            2,
            SourceRevision(1),
            DesktopEvent::Finished {
                document,
                run,
                snapshot: test_snapshot("stale", SourceRevision(1)),
            },
        ));
        assert!(!stale, "a Finished from an older revision must be ignored");
        assert_eq!(
            sink.snapshot(),
            state_before.as_ref(),
            "stale Finished must not change the sink state"
        );
        assert_eq!(
            sink.revision(),
            SourceRevision(2),
            "stale Finished must not advance the sink revision"
        );

        let newer = sink.apply(&sequenced(
            3,
            SourceRevision(3),
            DesktopEvent::Finished {
                document,
                run,
                snapshot: test_snapshot("newest", SourceRevision(3)),
            },
        ));
        assert!(newer, "a newer Finished must be applied");
        assert_eq!(
            sink.snapshot().map(|snapshot| snapshot.result.as_str()),
            Some("newest")
        );
        assert_eq!(sink.revision(), SourceRevision(3));
    }

    #[test]
    fn t8_output_events_are_exactly_the_printed_lines_and_do_not_accumulate_across_runs() {
        let host = ExecutionHost::new(Box::new(|| Box::new(PrintingEngine::default())));
        let doc = DocumentId(5);
        for run_number in 0..2 {
            let _handle = host
                .submit(doc, SourceRevision(1), "print twice".to_string(), 1000)
                .expect("submit");
            let events = drain_until_terminal(&host, Duration::from_secs(5));
            let outputs = events
                .iter()
                .filter_map(|event| match &event.event {
                    DesktopEvent::Output { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                outputs,
                vec!["first", "second"],
                "run {run_number} output events"
            );

            let finished = events
                .iter()
                .find_map(|event| match &event.event {
                    DesktopEvent::Finished { snapshot, .. } => Some(snapshot.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("run {run_number} had no Finished event"));
            assert_eq!(
                finished.output,
                vec!["first".to_string(), "second".to_string()],
                "run {run_number} snapshot output"
            );
        }
    }

    #[test]
    fn diagnostic_position_is_translated_back_to_the_full_source() {
        let source = "first = 1\nsecond = 2\nthird = 3\nfourth = 4\n";
        let statements = crate::statements::slice(source);
        assert_eq!(statements.len(), 4, "{statements:?}");
        let statement = &statements[2];
        assert_eq!(statement.start_line(), 3);

        let diagnostic =
            Diagnostic::for_statement(statement, source, "Unexpected token 'x' at position 6.");
        assert_eq!(diagnostic.statement_index, 2);
        assert_eq!(diagnostic.statement_line, 3);
        assert_eq!(diagnostic.byte_offset, statement.byte_offset() + 6);
        assert_eq!(diagnostic.line, 3);
        assert_eq!(diagnostic.column, 7);
        assert_eq!(diagnostic.message, "Unexpected token 'x' at position 6.");
        assert_eq!(
            diagnostic.render(),
            "line 3, column 7: Unexpected token 'x' at position 6."
        );

        let fallback = Diagnostic::for_statement(statement, source, "Cannot divide by zero.");
        assert_eq!(fallback.byte_offset, statement.byte_offset());
        assert_eq!(fallback.line, 3);
        assert_eq!(fallback.column, 1);
    }

    #[test]
    fn diagnostic_position_is_a_utf16_offset_not_a_byte_offset() {
        // "s = \"<emoji>\" + x": the emoji is four UTF-8 bytes but two UTF-16 code units, so the
        // engine's position 11 for 'x' is byte 13 and column 12 (1-based code units).
        let source = "s = \"\u{1f600}\" + x\n";
        let statements = crate::statements::slice(source);
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].text(), "s = \"\u{1f600}\" + x");

        let diagnostic = Diagnostic::for_statement(
            &statements[0],
            source,
            "Unexpected token 'x' at position 11.",
        );
        assert_eq!(diagnostic.byte_offset, 13);
        assert_eq!(diagnostic.line, 1);
        assert_eq!(diagnostic.column, 12);
        assert_eq!(byte_offset_utf16("s = \"\u{1f600}\" + x", 11), 13);
        assert_eq!(byte_offset_utf16("\u{1f600}x", 99), 5, "clamped to the end");
        assert_eq!(
            byte_offset_utf16("\u{1f600}x", 1),
            0,
            "inside a surrogate pair clamps to the character start"
        );
    }

    #[test]
    fn real_engine_executes_sliced_statements_and_snapshots_variables() {
        let names = Arc::new(Mutex::new(Vec::new()));
        let engine_factory = crate::engine::default_engine_factory();
        let recorded = Arc::clone(&names);
        let factory: EvaluatorFactory = Box::new(move || {
            let engine = engine_factory();
            recorded
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(engine.name());
            engine
        });
        let host = ExecutionHost::new(factory);
        let _handle = host
            .submit(
                DocumentId(11),
                SourceRevision(1),
                "a = 2\nb = a + 1\nb\n".to_string(),
                1000,
            )
            .expect("submit");
        let events = drain_until_terminal(&host, Duration::from_secs(30));
        let snapshot = match &events.last().unwrap().event {
            DesktopEvent::Finished { snapshot, .. } => snapshot,
            other => panic!(
                "real-engine run terminal was {}, expected Finished",
                event_summary(other)
            ),
        };
        assert_eq!(snapshot.result, "3");
        assert_eq!(snapshot.committed_steps, 3);
        // The real engine's variables() is contractually (name, typed display): CONTRACT.md
        // "Real API" says "name, typed display (sorted)". It also records the display of a
        // top-level expression under "_" (the reference behavior asserted by the engine's own
        // suite_parity_batch test evaluate_given_result_stores_in_underscore_variable), so the
        // script above leaves exactly a, b and _. These assertions therefore check the typed
        // contract and the underscore entry explicitly instead of the fake engine's plain form.
        #[cfg(feature = "real-engine")]
        {
            assert_eq!(snapshot.variables.len(), 3, "{:?}", snapshot.variables);
            assert!(
                snapshot
                    .variables
                    .contains(&("a".to_string(), "2 (Natural)".to_string())),
                "variables were {:?}",
                snapshot.variables
            );
            assert!(
                snapshot
                    .variables
                    .contains(&("b".to_string(), "3 (Natural)".to_string())),
                "variables were {:?}",
                snapshot.variables
            );
            assert!(
                snapshot
                    .variables
                    .contains(&("_".to_string(), "3 (Natural)".to_string())),
                "variables were {:?}",
                snapshot.variables
            );
        }
        #[cfg(not(feature = "real-engine"))]
        {
            assert_eq!(snapshot.variables.len(), 2, "{:?}", snapshot.variables);
            assert!(
                snapshot
                    .variables
                    .contains(&("a".to_string(), "2".to_string())),
                "variables were {:?}",
                snapshot.variables
            );
            assert!(
                snapshot
                    .variables
                    .contains(&("b".to_string(), "3".to_string())),
                "variables were {:?}",
                snapshot.variables
            );
        }
        #[cfg(feature = "real-engine")]
        assert_eq!(
            *names.lock().unwrap_or_else(|error| error.into_inner()),
            vec!["lovelace-lang"],
            "the default build must use the real engine"
        );
    }
}
