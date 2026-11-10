//! The headless application core: documents, commands, effects and the pane models.
//!
//! Everything in this module is pure Rust with no Win32 in sight, which is what makes the whole
//! application testable without a window. The shell owns a single AppState, converts menu and
//! accelerator input into Command values, calls apply, and then performs the returned Effect
//! values (start a run, show an error, repaint). The state transition itself never touches a
//! window handle.
//!
//! A document is one script buffer plus one engine instance, matching the Studio's "one session
//! per tab" rule: running in one document cannot mutate another document's buffer, revision,
//! variables or engine. Precision is document-scoped for the same reason.

use std::path::PathBuf;

use crate::engine::{self, EngineFactory, Evaluator, PlotCapture, RunOutput};
use crate::settings::{LayoutFractions, MAX_PRECISION, MIN_PRECISION, Settings};

/// Maximum number of log entries retained; the oldest are evicted first.
pub const LOG_CAP: usize = 500;

/// Maximum number of undo snapshots kept per document.
pub const UNDO_CAP: usize = 128;

/// Gap between adjacent panes, in pixels.
pub const PANE_GAP: i32 = 6;

/// Pixel sizes of the panes, recomputed by the shell on every resize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutSizes {
    /// Editor column width.
    pub editor_w: i32,
    /// Editor height (the plot pane sits below it).
    pub editor_h: i32,
    /// Right-hand workspace pane width.
    pub right_w: i32,
    /// Plot pane height.
    pub plot_h: i32,
    /// Logs strip height.
    pub logs_h: i32,
}

impl Default for LayoutSizes {
    fn default() -> LayoutSizes {
        LayoutSizes { editor_w: 800, editor_h: 420, right_w: 340, plot_h: 260, logs_h: 180 }
    }
}

/// Turns layout fractions into pane sizes for a client area, clamping so that no pane can
/// become negative or push another pane out of the window. Tiny client areas are tolerated.
pub fn compute_layout(client_w: i32, client_h: i32, fractions: &LayoutFractions) -> LayoutSizes {
    let width = client_w.max(1);
    let height = client_h.max(1);

    let logs_raw = ((height as i64 * fractions.logs_height_permille as i64) / 1000) as i32;
    let logs_h = logs_raw
        .max(60)
        .min((height / 2).max(60))
        .min(height);
    let top_h = (height - logs_h - PANE_GAP).max(0);

    let editor_raw = ((width as i64 * fractions.editor_width_permille as i64) / 1000) as i32;
    let min_editor = 240.min(width);
    let max_editor = (width - 240 - PANE_GAP).max(min_editor).min(width);
    let editor_w = editor_raw.clamp(min_editor, max_editor.max(min_editor));
    let right_w = (width - editor_w - PANE_GAP).max(0);

    let plot_h = if top_h <= 0 {
        0
    } else {
        let raw = ((top_h as i64 * fractions.plot_height_permille as i64) / 1000) as i32;
        let low = 80.min(top_h);
        let high = (top_h - 80).max(low).min(top_h);
        raw.clamp(low, high.max(low))
    };
    let editor_h = (top_h - plot_h - PANE_GAP).max(0);

    LayoutSizes { editor_w, editor_h, right_w, plot_h, logs_h }
}

/// Severity of a log entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    /// Informational message.
    Info,
    /// Warning.
    Warn,
    /// Error.
    Error,
    /// Raw print() output: rendered without a prefix so print output appears exactly once.
    Output,
    /// A result line, rendered without a prefix.
    Result,
}

impl LogLevel {
    /// The text shown in brackets before the message, or the empty string for raw output.
    pub fn tag(self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Output => "",
            LogLevel::Result => "",
        }
    }
}

/// One line in the logs pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// Severity.
    pub level: LogLevel,
    /// Message text (no trailing newline).
    pub text: String,
}

/// Whether a run is in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    /// No run in flight.
    Idle,
    /// A run has been requested and its result has not been published yet.
    Running,
}

impl RunState {
    /// The word shown in the status bar.
    pub fn label(self) -> &'static str {
        match self {
            RunState::Idle => "idle",
            RunState::Running => "running",
        }
    }
}

/// A run the shell is expected to perform and report back with finish_run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRequest {
    /// Index of the document the run belongs to.
    pub document: usize,
    /// Source text captured at request time.
    pub source: String,
    /// Display precision of that document.
    pub precision: usize,
}

/// Something the shell must do as a consequence of a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Evaluate this request (on a worker, or inline) and report back to finish_run.
    RunRequested(RunRequest),
    /// A document was written to this path.
    Saved(PathBuf),
    /// The command failed; show this message.
    Error(String),
    /// Repaint the panes from the current state.
    Repaint,
}

/// A user-level command. The shell maps menus, accelerators and buttons onto these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Add a new empty document and select it.
    NewDocument,
    /// Open a file in a new document.
    Open(PathBuf),
    /// Save the active document to its path.
    Save,
    /// Save the active document to a new path.
    SaveAs(PathBuf),
    /// Close the active document (refused when it has unsaved changes).
    CloseDocument,
    /// Select a tab by index.
    SelectTab(usize),
    /// Request a run of the active document.
    Run,
    /// Cancel the run that is in flight, if any.
    Cancel,
    /// Set the active document's display precision.
    SetPrecision(usize),
    /// Discard the active document's variables.
    ClearVariables,
    /// Evaluate one expression immediately, without touching the buffer.
    #[allow(dead_code)] // part of the command surface; a quick-eval field can bind it later
    QuickEval(String),
    /// Undo the last buffer change.
    Undo,
    /// Redo the last undone change.
    Redo,
    /// Insert text at the cursor.
    InsertText(String),
}

/// One script buffer plus the engine instance that belongs to it.
pub struct Document {
    /// File path, when the document has been saved or opened.
    pub path: Option<PathBuf>,
    /// UTF-8 script text.
    pub buffer: String,
    /// True when the buffer differs from the last saved text.
    pub dirty: bool,
    /// Display precision, in decimal places.
    pub precision: usize,
    /// Increments on every buffer change and every completed run.
    pub revision: u64,
    /// Cursor offset in bytes, always on a character boundary.
    pub cursor: usize,
    /// Variables published by this document's engine.
    pub vars: Vec<(String, String)>,
    /// Functions published by this document's engine.
    pub funcs: Vec<(String, Vec<String>)>,
    /// The most recent plot produced by this document's engine.
    pub last_plot: Option<PlotCapture>,
    engine: Box<dyn Evaluator>,
    saved_buffer: String,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

#[derive(Clone, Debug)]
struct Snapshot {
    buffer: String,
    cursor: usize,
}

impl Document {
    /// Creates an empty document owning the given engine.
    pub fn new(engine: Box<dyn Evaluator>, precision: usize) -> Document {
        let mut engine = engine;
        let precision = precision.clamp(MIN_PRECISION, MAX_PRECISION);
        engine.set_precision(precision);
        Document {
            path: None,
            buffer: String::new(),
            dirty: false,
            precision,
            revision: 0,
            cursor: 0,
            vars: Vec::new(),
            funcs: Vec::new(),
            last_plot: None,
            engine,
            saved_buffer: String::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The document's engine.
    pub fn engine(&self) -> &dyn Evaluator {
        &*self.engine
    }

    /// The document's engine, mutably.
    pub fn engine_mut(&mut self) -> &mut dyn Evaluator {
        &mut *self.engine
    }

    /// Sets the display precision and forwards it to the engine.
    pub fn set_precision(&mut self, decimals: usize) {
        self.precision = decimals.clamp(MIN_PRECISION, MAX_PRECISION);
        self.engine.set_precision(self.precision);
    }

    /// Moves the cursor, clamping it into the buffer and onto a character boundary.
    pub fn set_cursor(&mut self, position: usize) {
        self.cursor = floor_boundary(&self.buffer, position);
    }

    /// The cursor position as 1-based line and column.
    pub fn line_col(&self) -> (usize, usize) {
        line_col_at(&self.buffer, self.cursor)
    }

    /// Inserts text at the cursor as one undoable edit.
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.record_undo();
        self.buffer.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.bump();
    }

    /// Replaces the whole buffer with text typed into the native editor, as one undoable edit.
    pub fn set_text_from_editor(&mut self, text: &str) {
        if text == self.buffer {
            return;
        }
        self.record_undo();
        self.buffer.clear();
        self.buffer.push_str(text);
        self.cursor = floor_boundary(&self.buffer, self.cursor.min(self.buffer.len()));
        self.bump();
    }

    /// Undoes the last edit. Returns false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        let current = Snapshot {
            buffer: std::mem::take(&mut self.buffer),
            cursor: self.cursor,
        };
        self.redo.push(current);
        if self.redo.len() > UNDO_CAP {
            self.redo.remove(0);
        }
        self.buffer = snapshot.buffer;
        self.cursor = floor_boundary(&self.buffer, snapshot.cursor);
        self.bump();
        true
    }

    /// Redoes the last undone edit. Returns false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        let current = Snapshot {
            buffer: std::mem::take(&mut self.buffer),
            cursor: self.cursor,
        };
        self.undo.push(current);
        if self.undo.len() > UNDO_CAP {
            self.undo.remove(0);
        }
        self.buffer = snapshot.buffer;
        self.cursor = floor_boundary(&self.buffer, snapshot.cursor);
        self.bump();
        true
    }

    /// Marks the current buffer as the saved text and clears the dirty flag.
    pub fn mark_saved(&mut self) {
        self.saved_buffer.clear();
        self.saved_buffer.push_str(&self.buffer);
        self.dirty = false;
    }

    fn record_undo(&mut self) {
        self.undo.push(Snapshot { buffer: self.buffer.clone(), cursor: self.cursor });
        if self.undo.len() > UNDO_CAP {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn bump(&mut self) {
        self.revision = self.revision.saturating_add(1);
        self.dirty = self.buffer != self.saved_buffer;
    }
}

/// The whole application state that the shell drives.
pub struct AppState {
    /// Open documents; never empty.
    pub documents: Vec<Document>,
    /// Index of the active document.
    pub active: usize,
    /// Current pane sizes.
    pub layout: LayoutSizes,
    /// The logs pane model, oldest first.
    pub logs: Vec<LogEntry>,
    /// Elapsed time of the last completed run, in milliseconds.
    pub last_run_ms: u64,
    /// Whether a run is in flight.
    pub run_state: RunState,
    /// Persisted settings.
    pub settings: Settings,
    pending: Option<RunRequest>,
    factory: EngineFactory,
}

impl Default for AppState {
    fn default() -> AppState {
        AppState::new()
    }
}

impl AppState {
    /// Creates an application with the default engine and default settings.
    pub fn new() -> AppState {
        AppState::with_factory(engine::default_engine_factory())
    }

    /// Creates an application with the given engine factory.
    pub fn with_factory(factory: EngineFactory) -> AppState {
        AppState::with_factory_and_settings(factory, Settings::new())
    }

    /// Creates an application with the default engine and the given settings.
    pub fn with_settings(settings: Settings) -> AppState {
        AppState::with_factory_and_settings(engine::default_engine_factory(), settings)
    }

    /// Creates an application with the given engine factory and settings.
    pub fn with_factory_and_settings(factory: EngineFactory, mut settings: Settings) -> AppState {
        let precision = settings.active_precision.clamp(MIN_PRECISION, MAX_PRECISION);
        settings.active_precision = precision;
        let document = Document::new(factory(), precision);
        AppState {
            documents: vec![document],
            active: 0,
            layout: LayoutSizes::default(),
            logs: Vec::new(),
            last_run_ms: 0,
            run_state: RunState::Idle,
            settings,
            pending: None,
            factory,
        }
    }

    /// The active document, if any.
    pub fn active_doc(&self) -> Option<&Document> {
        self.documents.get(self.active)
    }

    /// The active document, mutably.
    pub fn active_doc_mut(&mut self) -> Option<&mut Document> {
        self.documents.get_mut(self.active)
    }

    /// The title of the tab at index, with an unsaved-changes marker left to the caller.
    pub fn tab_title(&self, index: usize) -> String {
        match self.documents.get(index) {
            Some(document) => match &document.path {
                Some(path) => path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string()),
                None => format!("Untitled {}", index + 1),
            },
            None => "(no document)".to_string(),
        }
    }

    /// The window caption for the active document.
    pub fn window_title(&self) -> String {
        let dirty = self.active_doc().map(|document| document.dirty).unwrap_or(false);
        format!(
            "Lovelace Desktop - {}{}",
            self.tab_title(self.active),
            if dirty { " *" } else { "" }
        )
    }

    /// The status bar text: revision, precision, cursor and run state.
    pub fn status_text(&self, line: usize, column: usize) -> String {
        match self.active_doc() {
            Some(document) => format!(
                "rev {} | precision {} | Ln {}, Col {} | {} | last run {} ms | engine {}",
                document.revision,
                document.precision,
                line,
                column,
                self.run_state.label(),
                self.last_run_ms,
                document.engine().name()
            ),
            None => "no document".to_string(),
        }
    }

    /// The right-hand pane text: variables above functions.
    pub fn variables_text(&self) -> String {
        let mut text = String::new();
        text.push_str("VARIABLES\n");
        match self.active_doc() {
            Some(document) if !document.vars.is_empty() => {
                for (name, value) in &document.vars {
                    text.push_str("  ");
                    text.push_str(name);
                    text.push_str(" = ");
                    text.push_str(value);
                    text.push('\n');
                }
            }
            _ => text.push_str("  (none)\n"),
        }
        text.push_str("\nFUNCTIONS\n");
        match self.active_doc() {
            Some(document) if !document.funcs.is_empty() => {
                for (name, parameters) in &document.funcs {
                    text.push_str("  ");
                    text.push_str(name);
                    text.push('(');
                    text.push_str(&parameters.join(", "));
                    text.push_str(")\n");
                }
            }
            _ => text.push_str("  (none)\n"),
        }
        text
    }

    /// The logs pane text, one entry per line, with severity prefixes.
    pub fn logs_text(&self) -> String {
        let mut text = String::new();
        for entry in &self.logs {
            let tag = entry.level.tag();
            if tag.is_empty() {
                text.push_str(&entry.text);
            } else {
                text.push('[');
                text.push_str(tag);
                text.push_str("] ");
                text.push_str(&entry.text);
            }
            text.push('\n');
        }
        text
    }

    /// Appends a log entry, evicting the oldest when the cap is reached.
    pub fn push_log(&mut self, level: LogLevel, text: impl Into<String>) {
        if self.logs.len() >= LOG_CAP {
            self.logs.remove(0);
        }
        self.logs.push(LogEntry { level, text: text.into() });
    }

    /// Copies the settings loader's warnings into the log.
    pub fn adopt_settings_warnings(&mut self) {
        for warning in self.settings.take_warnings() {
            self.push_log(LogLevel::Warn, warning);
        }
    }

    /// Recomputes the pane sizes for a client area.
    pub fn apply_layout(&mut self, client_w: i32, client_h: i32) {
        self.layout = compute_layout(client_w, client_h, &self.settings.layout);
    }

    /// The run request that is in flight, if any.
    pub fn pending_run(&self) -> Option<&RunRequest> {
        self.pending.as_ref()
    }

    /// Applies a command, returning the effects the shell must perform.
    pub fn apply(&mut self, command: Command) -> Vec<Effect> {
        match command {
            Command::NewDocument => self.new_document(),
            Command::Open(path) => self.open(path),
            Command::Save => self.save(),
            Command::SaveAs(path) => self.save_as(path),
            Command::CloseDocument => self.close_document(),
            Command::SelectTab(index) => self.select_tab(index),
            Command::Run => self.request_run(),
            Command::Cancel => self.cancel_run(),
            Command::SetPrecision(decimals) => self.set_precision(decimals),
            Command::ClearVariables => self.clear_variables(),
            Command::QuickEval(source) => self.quick_eval(&source),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
            Command::InsertText(text) => self.insert_text(&text),
        }
    }

    /// Synchronises the buffer with the native editor's text. This is a view callback rather
    /// than a user command, so it is not part of Command.
    pub fn set_active_text(&mut self, text: &str) -> Vec<Effect> {
        if let Some(document) = self.documents.get_mut(self.active) {
            document.set_text_from_editor(text);
        }
        vec![Effect::Repaint]
    }

    /// Requests a run, evaluates it immediately on this thread and publishes the result. The
    /// shell uses the two halves separately (apply, then finish_run) so that a worker could sit
    /// between them; the headless tests and --selftest use this convenience.
    pub fn run_blocking(&mut self) -> Vec<Effect> {
        let mut effects = self.apply(Command::Run);
        let request = effects.iter().find_map(|effect| match effect {
            Effect::RunRequested(request) => Some(request.clone()),
            _ => None,
        });
        let Some(request) = request else {
            return effects;
        };
        let started = std::time::Instant::now();
        let outcome = match self.documents.get_mut(request.document) {
            Some(document) => document.engine_mut().eval_full(&request.source),
            None => return effects,
        };
        let elapsed = started.elapsed().as_millis() as u64;
        effects.extend(self.finish_run(request.document, outcome, elapsed));
        effects
    }

    /// Publishes the outcome of a run requested earlier. Results for a document that is no
    /// longer running, or for a run that was cancelled, are discarded.
    pub fn finish_run(&mut self, document: usize, outcome: RunOutput, elapsed_ms: u64) -> Vec<Effect> {
        let expected = matches!(
            (&self.pending, self.run_state),
            (Some(pending), RunState::Running) if pending.document == document
        );
        if !expected {
            return Vec::new();
        }
        self.pending = None;
        self.run_state = RunState::Idle;
        self.last_run_ms = elapsed_ms;

        let RunOutput { result, output, variables, functions, plot } = outcome;
        let mut effects = Vec::new();
        if let Some(active) = self.documents.get_mut(document) {
            active.vars = variables;
            active.funcs = functions;
            if let Some(plot) = plot {
                active.last_plot = Some(plot);
            }
            active.revision = active.revision.saturating_add(1);
        }
        for line in output {
            self.push_log(LogLevel::Output, line);
        }
        match result {
            Ok(display) => self.push_log(LogLevel::Result, format!("= {display}")),
            Err(message) => {
                self.push_log(LogLevel::Error, format!("error: {message}"));
                effects.push(Effect::Error(message));
            }
        }
        effects.push(Effect::Repaint);
        effects
    }

    fn new_document(&mut self) -> Vec<Effect> {
        let precision = self
            .active_doc()
            .map(|document| document.precision)
            .unwrap_or(self.settings.active_precision)
            .clamp(MIN_PRECISION, MAX_PRECISION);
        let document = Document::new((self.factory)(), precision);
        self.documents.push(document);
        self.active = self.documents.len() - 1;
        self.push_log(LogLevel::Info, format!("New document (precision {precision})."));
        vec![Effect::Repaint]
    }

    fn open(&mut self, path: PathBuf) -> Vec<Effect> {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let precision = self.settings.active_precision.clamp(MIN_PRECISION, MAX_PRECISION);
                let mut document = Document::new((self.factory)(), precision);
                document.buffer = text;
                document.mark_saved();
                document.path = Some(path.clone());
                self.documents.push(document);
                self.active = self.documents.len() - 1;
                self.settings.add_recent(&path.to_string_lossy());
                self.push_log(LogLevel::Info, format!("Opened {}", path.display()));
                vec![Effect::Repaint]
            }
            Err(error) => {
                let message = format!("Cannot open {}: {error}", path.display());
                self.push_log(LogLevel::Error, message.clone());
                vec![Effect::Error(message), Effect::Repaint]
            }
        }
    }

    fn save(&mut self) -> Vec<Effect> {
        match self.active_doc().and_then(|document| document.path.clone()) {
            Some(path) => self.write_active(path),
            None => {
                let message = "This document has no path yet; use Save As.".to_string();
                self.push_log(LogLevel::Warn, message.clone());
                vec![Effect::Error(message), Effect::Repaint]
            }
        }
    }

    fn save_as(&mut self, path: PathBuf) -> Vec<Effect> {
        self.write_active(path)
    }

    fn write_active(&mut self, path: PathBuf) -> Vec<Effect> {
        let outcome = match self.documents.get_mut(self.active) {
            None => Err("No document is open.".to_string()),
            Some(document) => match std::fs::write(&path, document.buffer.as_bytes()) {
                Ok(()) => {
                    document.path = Some(path.clone());
                    document.mark_saved();
                    Ok(())
                }
                Err(error) => Err(format!("Cannot save {}: {error}", path.display())),
            },
        };
        match outcome {
            Ok(()) => {
                self.settings.add_recent(&path.to_string_lossy());
                self.push_log(LogLevel::Info, format!("Saved {}", path.display()));
                vec![Effect::Saved(path), Effect::Repaint]
            }
            Err(message) => {
                self.push_log(LogLevel::Error, message.clone());
                vec![Effect::Error(message), Effect::Repaint]
            }
        }
    }

    fn close_document(&mut self) -> Vec<Effect> {
        let Some(document) = self.documents.get(self.active) else {
            return vec![Effect::Repaint];
        };
        if document.dirty {
            let message = format!(
                "Cannot close {}: it has unsaved changes. Save it first.",
                self.tab_title(self.active)
            );
            self.push_log(LogLevel::Warn, message.clone());
            return vec![Effect::Error(message), Effect::Repaint];
        }
        let closed = self.active;
        self.documents.remove(closed);
        if self.pending.as_ref().map(|pending| pending.document) == Some(closed) {
            self.pending = None;
            self.run_state = RunState::Idle;
        }
        if self.documents.is_empty() {
            let precision = self.settings.active_precision.clamp(MIN_PRECISION, MAX_PRECISION);
            self.documents.push(Document::new((self.factory)(), precision));
            self.active = 0;
        } else if self.active >= self.documents.len() {
            self.active = self.documents.len() - 1;
        }
        self.sync_active_precision();
        self.push_log(LogLevel::Info, "Document closed.");
        vec![Effect::Repaint]
    }

    fn select_tab(&mut self, index: usize) -> Vec<Effect> {
        if index >= self.documents.len() {
            let message = format!("No tab at index {index}.");
            self.push_log(LogLevel::Warn, message.clone());
            return vec![Effect::Error(message), Effect::Repaint];
        }
        self.active = index;
        self.sync_active_precision();
        vec![Effect::Repaint]
    }

    fn request_run(&mut self) -> Vec<Effect> {
        if self.run_state == RunState::Running {
            self.push_log(LogLevel::Warn, "A run is already in progress.");
            return vec![Effect::Repaint];
        }
        let Some(document) = self.active_doc() else {
            return vec![Effect::Repaint];
        };
        if document.buffer.trim().is_empty() {
            self.push_log(LogLevel::Warn, "Nothing to run: the document is empty.");
            return vec![Effect::Repaint];
        }
        let request = RunRequest {
            document: self.active,
            source: document.buffer.clone(),
            precision: document.precision,
        };
        let revision = document.revision;
        self.pending = Some(request.clone());
        self.run_state = RunState::Running;
        self.push_log(LogLevel::Info, format!("Run started (revision {revision})."));
        vec![Effect::RunRequested(request), Effect::Repaint]
    }

    fn cancel_run(&mut self) -> Vec<Effect> {
        if self.run_state == RunState::Running {
            self.pending = None;
            self.run_state = RunState::Idle;
            self.push_log(LogLevel::Info, "Run cancelled.");
        } else {
            self.push_log(LogLevel::Info, "Nothing to cancel.");
        }
        vec![Effect::Repaint]
    }

    fn set_precision(&mut self, decimals: usize) -> Vec<Effect> {
        let decimals = decimals.clamp(MIN_PRECISION, MAX_PRECISION);
        let Some(document) = self.documents.get_mut(self.active) else {
            return vec![Effect::Repaint];
        };
        document.set_precision(decimals);
        self.settings.active_precision = decimals;
        self.push_log(LogLevel::Info, format!("Precision set to {decimals}."));
        vec![Effect::Repaint]
    }

    fn clear_variables(&mut self) -> Vec<Effect> {
        let Some(document) = self.documents.get_mut(self.active) else {
            return vec![Effect::Repaint];
        };
        document.engine_mut().clear();
        document.vars.clear();
        let functions = document.engine().functions();
        document.funcs = functions;
        document.revision = document.revision.saturating_add(1);
        self.push_log(LogLevel::Info, "Variables cleared.");
        vec![Effect::Repaint]
    }

    fn quick_eval(&mut self, source: &str) -> Vec<Effect> {
        let source = source.trim();
        if source.is_empty() {
            return vec![Effect::Repaint];
        }
        self.push_log(LogLevel::Info, format!("> {source}"));
        let outcome = match self.documents.get_mut(self.active) {
            Some(document) => document.engine_mut().eval_full(source),
            None => return vec![Effect::Repaint],
        };
        let RunOutput { result, output, variables, functions, plot } = outcome;
        let mut effects = Vec::new();
        if let Some(document) = self.documents.get_mut(self.active) {
            document.vars = variables;
            document.funcs = functions;
            if let Some(plot) = plot {
                document.last_plot = Some(plot);
            }
        }
        for line in output {
            self.push_log(LogLevel::Output, line);
        }
        match result {
            Ok(display) => self.push_log(LogLevel::Result, format!("= {display}")),
            Err(message) => {
                self.push_log(LogLevel::Error, format!("error: {message}"));
                effects.push(Effect::Error(message));
            }
        }
        effects.push(Effect::Repaint);
        effects
    }

    fn undo(&mut self) -> Vec<Effect> {
        let undone = match self.documents.get_mut(self.active) {
            Some(document) => document.undo(),
            None => false,
        };
        if !undone {
            self.push_log(LogLevel::Warn, "Nothing to undo.");
        }
        vec![Effect::Repaint]
    }

    fn redo(&mut self) -> Vec<Effect> {
        let redone = match self.documents.get_mut(self.active) {
            Some(document) => document.redo(),
            None => false,
        };
        if !redone {
            self.push_log(LogLevel::Warn, "Nothing to redo.");
        }
        vec![Effect::Repaint]
    }

    fn insert_text(&mut self, text: &str) -> Vec<Effect> {
        if let Some(document) = self.documents.get_mut(self.active) {
            document.insert_text(text);
        }
        vec![Effect::Repaint]
    }

    fn sync_active_precision(&mut self) {
        if let Some(document) = self.active_doc() {
            self.settings.active_precision = document.precision;
        }
    }
}

/// The largest character boundary at or below position.
fn floor_boundary(text: &str, mut position: usize) -> usize {
    if position >= text.len() {
        return text.len();
    }
    while position > 0 && !text.is_char_boundary(position) {
        position -= 1;
    }
    position
}

/// Converts a byte offset into a 1-based line and column.
fn line_col_at(text: &str, position: usize) -> (usize, usize) {
    let upto = floor_boundary(text, position);
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in text[..upto].chars() {
        match ch {
            '\n' => {
                line += 1;
                column = 1;
            }
            '\r' => {}
            _ => column += 1,
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{FakeEngine, RunOutput};
    use crate::settings::LayoutFractions;

    fn app() -> AppState {
        AppState::with_factory(|| Box::new(FakeEngine::new()))
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lovelace-app-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        dir
    }

    fn empty_output() -> RunOutput {
        RunOutput {
            result: Ok("Void".to_string()),
            output: Vec::new(),
            variables: Vec::new(),
            functions: Vec::new(),
            plot: None,
        }
    }

    #[test]
    fn starts_with_one_empty_document_at_the_default_precision() {
        let app = app();
        assert_eq!(app.documents.len(), 1);
        assert_eq!(app.active, 0);
        assert!(app.documents[0].buffer.is_empty());
        assert!(!app.documents[0].dirty);
        assert_eq!(app.documents[0].precision, engine::DEFAULT_PRECISION);
        assert_eq!(app.run_state, RunState::Idle);
        assert!(app.logs.is_empty());
        assert_eq!(app.last_run_ms, 0);
        assert_eq!(app.tab_title(0), "Untitled 1");
    }

    #[test]
    fn new_document_appends_and_selects_a_fresh_tab() {
        let mut app = app();
        let effects = app.apply(Command::NewDocument);
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.active, 1);
        assert!(effects.contains(&Effect::Repaint));
        assert_eq!(app.tab_title(1), "Untitled 2");
        assert_eq!(app.documents[1].precision, app.documents[0].precision);
    }

    #[test]
    fn insert_text_marks_dirty_and_bumps_the_revision() {
        let mut app = app();
        app.apply(Command::InsertText("1 + 1".to_string()));
        let document = &app.documents[0];
        assert_eq!(document.buffer, "1 + 1");
        assert_eq!(document.revision, 1);
        assert!(document.dirty);
        assert_eq!(document.cursor, 5);
        assert_eq!(document.line_col(), (1, 6));
    }

    #[test]
    fn run_publishes_one_coherent_update_to_logs_and_variables() {
        let mut app = app();
        app.apply(Command::InsertText("a = 6\nprint(\"hi\")\na * 7".to_string()));
        let effects = app.run_blocking();
        assert!(effects.iter().any(|effect| matches!(effect, Effect::RunRequested(_))));
        assert!(effects.contains(&Effect::Repaint));
        assert_eq!(app.run_state, RunState::Idle);
        assert_eq!(
            app.documents[0].vars,
            vec![("a".to_string(), "6".to_string())]
        );
        assert!(app.logs_text().contains("hi"));
        assert!(app.logs_text().contains("= 42"));
        assert_eq!(app.documents[0].revision, 2);
        assert!(app.active_doc().unwrap().last_plot.is_none());
    }

    #[test]
    fn apply_run_only_requests_the_effect() {
        let mut app = app();
        app.apply(Command::InsertText("1".to_string()));
        let effects = app.apply(Command::Run);
        let request = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::RunRequested(request) => Some(request.clone()),
                _ => None,
            })
            .expect("a run request");
        assert_eq!(request.document, 0);
        assert_eq!(request.source, "1");
        assert_eq!(request.precision, engine::DEFAULT_PRECISION);
        assert_eq!(app.run_state, RunState::Running);
        assert!(app.pending_run().is_some());
        assert!(app.documents[0].vars.is_empty());
    }

    #[test]
    fn cancel_discards_the_pending_run_and_ignores_a_late_result() {
        let mut app = app();
        app.apply(Command::InsertText("x = 1".to_string()));
        app.apply(Command::Run);
        let effects = app.apply(Command::Cancel);
        assert!(effects.contains(&Effect::Repaint));
        assert_eq!(app.run_state, RunState::Idle);
        assert!(app.pending_run().is_none());
        assert!(app.logs_text().contains("Run cancelled."));

        let revision = app.documents[0].revision;
        let late = RunOutput {
            result: Ok("1".to_string()),
            output: vec!["late output".to_string()],
            variables: vec![("x".to_string(), "1".to_string())],
            functions: Vec::new(),
            plot: None,
        };
        let stale = app.finish_run(0, late, 5);
        assert!(stale.is_empty(), "a cancelled run must not publish");
        assert_eq!(app.documents[0].revision, revision);
        assert!(app.documents[0].vars.is_empty());
        assert!(!app.logs_text().contains("late output"));
        assert_eq!(app.last_run_ms, 0);
    }

    #[test]
    fn cancel_without_a_run_says_there_was_nothing_to_cancel() {
        let mut app = app();
        app.apply(Command::Cancel);
        assert!(app.logs_text().contains("Nothing to cancel."));
        assert_eq!(app.run_state, RunState::Idle);
    }

    #[test]
    fn tab_isolation_running_in_one_document_does_not_touch_the_other() {
        let mut app = app();
        app.apply(Command::NewDocument);
        app.apply(Command::InsertText("a = 1".to_string()));
        app.run_blocking();
        assert_eq!(
            app.documents[1].vars,
            vec![("a".to_string(), "1".to_string())]
        );

        assert!(app.documents[0].buffer.is_empty());
        assert_eq!(app.documents[0].revision, 0);
        assert!(app.documents[0].vars.is_empty());
        assert!(app.documents[0].last_plot.is_none());
        assert!(!app.documents[0].dirty);

        app.apply(Command::SelectTab(0));
        app.apply(Command::InsertText("a".to_string()));
        let effects = app.run_blocking();
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Error(_))));
        assert!(app.logs_text().contains("Undefined variable 'a'."));
        assert!(app.documents[0].vars.is_empty());
        assert_eq!(
            app.documents[1].vars,
            vec![("a".to_string(), "1".to_string())]
        );
        assert_eq!(app.active, 0);
    }

    #[test]
    fn precision_is_document_scoped() {
        let mut app = app();
        app.apply(Command::NewDocument);
        app.apply(Command::SetPrecision(250));
        assert_eq!(app.documents[1].precision, 250);
        assert_eq!(app.documents[1].engine().precision(), 250);
        assert_eq!(app.documents[0].precision, engine::DEFAULT_PRECISION);
        assert_eq!(app.settings.active_precision, 250);
        app.apply(Command::SelectTab(0));
        assert_eq!(app.settings.active_precision, engine::DEFAULT_PRECISION);
        assert_eq!(app.documents[0].precision, engine::DEFAULT_PRECISION);
    }

    #[test]
    fn precision_is_clamped_to_the_supported_range() {
        let mut app = app();
        app.apply(Command::SetPrecision(0));
        assert_eq!(app.documents[0].precision, MIN_PRECISION);
        app.apply(Command::SetPrecision(usize::MAX));
        assert_eq!(app.documents[0].precision, MAX_PRECISION);
        assert_eq!(app.settings.active_precision, MAX_PRECISION);
    }

    #[test]
    fn save_round_trip_writes_reads_and_tracks_recent_files() {
        let dir = scratch("save");
        let path = dir.join("script.ls");
        let mut app = app();
        app.apply(Command::InsertText("1 + 2\n".to_string()));
        let effects = app.apply(Command::SaveAs(path.clone()));
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Saved(saved) if saved == &path)));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "1 + 2\n");
        assert!(!app.documents[0].dirty);
        assert_eq!(app.documents[0].path.as_deref(), Some(path.as_path()));
        assert!(app.settings.recent_files.iter().any(|recent| recent == &path.to_string_lossy()));

        let effects = app.apply(Command::Open(path.clone()));
        assert!(effects.contains(&Effect::Repaint));
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.documents[1].buffer, "1 + 2\n");
        assert_eq!(app.documents[1].path.as_deref(), Some(path.as_path()));
        assert!(!app.documents[1].dirty);
        assert_eq!(app.active, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_without_a_path_reports_an_error() {
        let mut app = app();
        let effects = app.apply(Command::Save);
        assert!(matches!(effects.first(), Some(Effect::Error(_))));
        assert!(app.logs_text().contains("no path"));
        assert!(app.documents[0].dirty == false);
    }

    #[test]
    fn opening_a_missing_file_reports_an_error_and_keeps_the_document_count() {
        let mut app = app();
        let effects = app.apply(Command::Open(PathBuf::from("Z:/lovelace/does-not-exist.ls")));
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Error(_))));
        assert_eq!(app.documents.len(), 1);
        assert!(app.logs_text().contains("Cannot open"));
    }

    #[test]
    fn closing_a_dirty_document_is_refused_until_it_is_saved() {
        let dir = scratch("close");
        let path = dir.join("a.ls");
        let mut app = app();
        app.apply(Command::InsertText("x = 1".to_string()));
        let effects = app.apply(Command::CloseDocument);
        assert!(matches!(effects.first(), Some(Effect::Error(_))));
        assert_eq!(app.documents.len(), 1);

        app.apply(Command::SaveAs(path));
        let effects = app.apply(Command::CloseDocument);
        assert!(effects.contains(&Effect::Repaint));
        assert_eq!(app.documents.len(), 1, "closing the last tab leaves a fresh one");
        assert!(app.documents[0].buffer.is_empty());
        assert_eq!(app.active, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undo_and_redo_restore_the_buffer_and_the_clean_state() {
        let mut app = app();
        app.apply(Command::InsertText("abc".to_string()));
        app.apply(Command::InsertText("def".to_string()));
        assert_eq!(app.documents[0].buffer, "abcdef");
        app.apply(Command::Undo);
        assert_eq!(app.documents[0].buffer, "abc");
        app.apply(Command::Undo);
        assert_eq!(app.documents[0].buffer, "");
        assert!(!app.documents[0].dirty, "back at the saved text, so clean");
        app.apply(Command::Redo);
        assert_eq!(app.documents[0].buffer, "abc");
        assert!(app.documents[0].dirty);
        app.apply(Command::Redo);
        assert_eq!(app.documents[0].buffer, "abcdef");
    }

    #[test]
    fn undo_with_an_empty_history_logs_a_warning() {
        let mut app = app();
        app.apply(Command::Undo);
        assert!(app.logs_text().contains("Nothing to undo."));
    }

    #[test]
    fn the_log_cap_evicts_oldest_entries_first() {
        let mut app = app();
        for index in 0..(LOG_CAP + 25) {
            app.push_log(LogLevel::Info, format!("line {index}"));
        }
        assert_eq!(app.logs.len(), LOG_CAP);
        assert_eq!(app.logs[0].text, "line 25");
        assert_eq!(app.logs[LOG_CAP - 1].text, format!("line {}", LOG_CAP + 24));
    }

    #[test]
    fn quick_eval_updates_variables_without_touching_the_buffer() {
        let mut app = app();
        let effects = app.apply(Command::QuickEval("n = 3 * 4".to_string()));
        assert!(effects.contains(&Effect::Repaint));
        assert!(app.documents[0].buffer.is_empty());
        assert_eq!(app.documents[0].vars, vec![("n".to_string(), "12".to_string())]);
        assert!(app.logs_text().contains("> n = 3 * 4"));
        assert!(app.logs_text().contains("= 12"));
        assert_eq!(app.documents[0].revision, 0);
    }

    #[test]
    fn quick_eval_reports_errors_as_effects() {
        let mut app = app();
        let effects = app.apply(Command::QuickEval("1 / 0".to_string()));
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Error(_))));
        assert!(app.logs_text().contains("Cannot divide by zero."));
    }

    #[test]
    fn clear_variables_empties_the_workspace_but_keeps_functions() {
        let mut app = app();
        app.apply(Command::InsertText("x = 1".to_string()));
        app.run_blocking();
        assert!(!app.documents[0].vars.is_empty());
        app.apply(Command::ClearVariables);
        assert!(app.documents[0].vars.is_empty());
        assert!(!app.documents[0].funcs.is_empty());
        assert!(app.variables_text().contains("(none)"));
    }

    #[test]
    fn layout_sizes_follow_the_saved_fractions() {
        let sizes = compute_layout(1280, 800, &LayoutFractions::default());
        assert_eq!(sizes.logs_h, 176);
        assert_eq!(sizes.editor_w, 921);
        assert_eq!(sizes.right_w, 1280 - 921 - PANE_GAP);
        assert_eq!(sizes.plot_h, 284);
        assert_eq!(sizes.editor_h, 618 - 284 - PANE_GAP);
        assert_eq!(
            sizes.editor_h + sizes.plot_h + PANE_GAP,
            800 - sizes.logs_h - PANE_GAP
        );
    }

    #[test]
    fn tiny_layouts_never_produce_negative_or_overlapping_sizes() {
        for (width, height) in [(1, 1), (60, 40), (100, 80), (320, 240), (900, 600), (3840, 2160)] {
            let sizes = compute_layout(width, height, &LayoutFractions::default());
            assert!(sizes.editor_w >= 0 && sizes.right_w >= 0);
            assert!(sizes.editor_h >= 0 && sizes.plot_h >= 0 && sizes.logs_h >= 0);
            assert!(sizes.editor_w + sizes.right_w <= width.max(1));
            assert!(sizes.editor_h + sizes.plot_h + sizes.logs_h <= height.max(1));
        }
    }

    #[test]
    fn window_title_marks_unsaved_changes() {
        let dir = scratch("title");
        let path = dir.join("t.ls");
        let mut app = app();
        app.apply(Command::InsertText("x".to_string()));
        assert_eq!(app.window_title(), "Lovelace Desktop - Untitled 1 *");
        app.apply(Command::SaveAs(path));
        assert_eq!(app.window_title(), "Lovelace Desktop - t.ls");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn insert_text_clamps_multibyte_cursors_to_character_boundaries() {
        let mut app = app();
        app.apply(Command::InsertText("µ≈".to_string()));
        assert_eq!(app.documents[0].buffer, "µ≈");
        assert_eq!(app.documents[0].cursor, "µ≈".len());
        app.documents[0].set_cursor(1);
        assert_eq!(app.documents[0].cursor, 0, "1 is inside the two-byte µ");
        app.apply(Command::InsertText("x".to_string()));
        assert_eq!(app.documents[0].buffer, "xµ≈");
    }

    #[test]
    fn status_text_reports_revision_precision_cursor_and_run_state() {
        let mut app = app();
        app.apply(Command::InsertText("ab\ncd".to_string()));
        app.documents[0].set_cursor(4);
        let (line, column) = app.documents[0].line_col();
        assert_eq!((line, column), (2, 2));
        let status = app.status_text(line, column);
        assert!(status.contains("rev 1"), "status: {status}");
        assert!(status.contains("precision 1000"), "status: {status}");
        assert!(status.contains("Ln 2, Col 2"), "status: {status}");
        assert!(status.contains("idle"), "status: {status}");
    }

    #[test]
    fn run_on_an_empty_document_is_rejected_with_a_warning() {
        let mut app = app();
        let effects = app.apply(Command::Run);
        assert!(effects.contains(&Effect::Repaint));
        assert!(!effects.iter().any(|effect| matches!(effect, Effect::RunRequested(_))));
        assert!(app.logs_text().contains("Nothing to run"));
    }

    #[test]
    fn a_second_run_while_running_is_rejected() {
        let mut app = app();
        app.apply(Command::InsertText("1".to_string()));
        app.apply(Command::Run);
        let effects = app.apply(Command::Run);
        assert!(!effects.iter().any(|effect| matches!(effect, Effect::RunRequested(_))));
        assert!(app.logs_text().contains("already in progress"));
        app.finish_run(0, empty_output(), 1);
        assert_eq!(app.run_state, RunState::Idle);
    }

    #[test]
    fn a_plot_produced_by_a_run_reaches_the_document() {
        let mut app = app();
        app.apply(Command::InsertText("plot(\"p\", 1, 2, 3)".to_string()));
        app.run_blocking();
        let plot = app.documents[0].last_plot.as_ref().expect("a captured plot");
        assert_eq!(plot.title, "p");
        assert!(plot.svg.contains("<polyline"));
        assert!(plot.path.ends_with(".svg"));
    }

    #[test]
    fn selecting_a_missing_tab_reports_an_error() {
        let mut app = app();
        let effects = app.apply(Command::SelectTab(9));
        assert!(matches!(effects.first(), Some(Effect::Error(_))));
        assert_eq!(app.active, 0);
    }

    #[test]
    fn settings_warnings_are_adopted_into_the_log() {
        let settings = Settings::from_json("{ broken");
        let mut app = AppState::with_factory_and_settings(|| Box::new(FakeEngine::new()), settings);
        app.adopt_settings_warnings();
        assert!(app.logs_text().contains("not valid JSON"));
        assert!(app.settings.warnings.is_empty());
    }
}
