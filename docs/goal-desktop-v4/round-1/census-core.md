# Headless Desktop Core — As-Built Census (Round 1)

**Subject:** `rust/lovelace-desktop` headless core as it exists in the working tree.
**Method:** static read-only inspection of the seven in-scope files. No `cargo` was run (the build lock is held by another process), no code was executed, and no file other than this deliverable was written. All line numbers are 1-based and refer to the file snapshot whose SHA-256 is recorded in the appendix. No statement below relies on an out-of-scope file.
**Scope:** `src/app.rs`, `src/engine.rs`, `src/engine/fake.rs`, `src/engine/real.rs`, `src/settings.rs`, `src/main.rs`, `Cargo.toml`. `src/win32/`, `src/svg.rs`, `src/selftest.rs`, the other crates, and all other files were not inspected as sources.

**Counts:** OBS rows: **238** · `Command` variants: **14** · public non-`Command` state transitions: **15** · UI-facing fields/values: **59**.

**Confidence legend.** Every row is `High`: the fact is either stated directly by the cited line(s), or is a negative observation over an in-scope file that was read in full. Scope limits on what the whole application may do are listed in section 4, not encoded as lower-confidence rows.

## 1. OBS

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| B-01 | Cargo.toml:14 | The real engine is an optional path dependency: `lovelace-lang = { path = "../lovelace-lang", optional = true }`. | High |
| B-02 | Cargo.toml:17 | The default feature set includes the real engine: `default = ["real-engine"]`. | High |
| B-03 | Cargo.toml:19 | The feature that enables the adapter is `real-engine = ["dep:lovelace-lang"]`. | High |
| B-04 | main.rs:34-36 | `--selftest` is handled before the shell by `if args.iter().any(|arg| arg == "--selftest") {`, which exits via `std::process::exit(selftest::run());`. | High |
| B-05 | main.rs:46-50 | Unknown `--` options are rejected with exit code 2: `eprintln!("unknown option {}\n\n{}", unknown[0], usage()); std::process::exit(2);`. | High |
| B-06 | main.rs:51-52 | The first non-`--` argument is passed as the initial path: `let initial = args.iter().find(\|arg\| !arg.starts_with("--")).cloned();` and `std::process::exit(win32::shell::run(initial.as_deref()));`. | High |
| B-07 | main.rs:56-62 | The non-Windows `main` runs `--selftest` when asked, otherwise writes `this build targets Windows; the native shell is unavailable` and exits 2. | High |
| B-08 | engine.rs:130-133 | Without the `real-engine` feature the factory body is `fake::boxed` (signature `pub fn default_engine_factory() -> EngineFactory`). | High |
| B-09 | engine.rs:136-139 | With `real-engine` the factory body is `real::boxed` (signature `pub fn default_engine_factory() -> EngineFactory`). | High |
| D-01 | app.rs:204 | A document is declared as `pub struct Document {`. | High |
| D-02 | app.rs:206 | Document file path: `pub path: Option<PathBuf>,`. | High |
| D-03 | app.rs:208 | Document text: `pub buffer: String,`. | High |
| D-04 | app.rs:210 | Unsaved-change flag: `pub dirty: bool,`. | High |
| D-05 | app.rs:212 | Per-document precision: `pub precision: usize,`. | High |
| D-06 | app.rs:214 | Per-document revision: `pub revision: u64,`. | High |
| D-07 | app.rs:216 | Cursor as a byte offset: `pub cursor: usize,` with the doc comment `/// Cursor offset in bytes, always on a character boundary.` (app.rs:215). | High |
| D-08 | app.rs:218 | Workspace variables: `pub vars: Vec<(String, String)>,`. | High |
| D-09 | app.rs:220 | Workspace functions: `pub funcs: Vec<(String, Vec<String>)>,`. | High |
| D-10 | app.rs:222 | Last plot: `pub last_plot: Option<PlotCapture>,`. | High |
| D-11 | app.rs:223 | The engine is a private owned trait object: `engine: Box<dyn Evaluator>,`. | High |
| D-12 | app.rs:224-226 | Saved text and edit history are private: `saved_buffer: String,`, `undo: Vec<Snapshot>,`, `redo: Vec<Snapshot>,`. | High |
| D-13 | app.rs:237 | Constructor signature: `pub fn new(engine: Box<dyn Evaluator>, precision: usize) -> Document {`. | High |
| D-14 | app.rs:239-240 | Construction clamps precision and forwards it: `let precision = precision.clamp(MIN_PRECISION, MAX_PRECISION);` then `engine.set_precision(precision);`. | High |
| D-15 | app.rs:241-255 | A new document starts as `path: None`, `buffer: String::new()`, `dirty: false`, `revision: 0`, `cursor: 0`, empty `vars`/`funcs`, `last_plot: None`, empty `saved_buffer`/`undo`/`redo`. | High |
| D-16 | app.rs:259-266 | The engine is exposed through `pub fn engine(&self) -> &dyn Evaluator` and `pub fn engine_mut(&mut self) -> &mut dyn Evaluator`. | High |
| D-17 | app.rs:269-272 | `Document::set_precision` clamps and forwards: `self.precision = decimals.clamp(MIN_PRECISION, MAX_PRECISION); self.engine.set_precision(self.precision);`. | High |
| D-18 | app.rs:275-277 | `Document::set_cursor` clamps onto a character boundary: `self.cursor = floor_boundary(&self.buffer, position);`. | High |
| D-19 | app.rs:280-282 | `Document::line_col` returns the cursor's 1-based line and column: `line_col_at(&self.buffer, self.cursor)`. | High |
| D-20 | app.rs:285-293 | `Document::insert_text` returns immediately for empty input (`if text.is_empty() { return; }`), otherwise records undo, inserts at the cursor, advances the cursor by `text.len()`, and calls `bump()`. | High |
| D-21 | app.rs:296-305 | `Document::set_text_from_editor` returns immediately when `text == self.buffer`; otherwise one undo snapshot is recorded, the whole buffer is replaced, the cursor is clamped, and `bump()` is called. | High |
| D-22 | app.rs:308-324 | `Document::undo` pops one snapshot, pushes the current state onto `redo`, restores buffer and boundary-clamped cursor, and calls `bump()`; it returns `false` when `undo` is empty. | High |
| D-23 | app.rs:327-343 | `Document::redo` is the symmetric operation, popping `redo`, pushing onto `undo`, restoring buffer and cursor, and calling `bump()`; it returns `false` when `redo` is empty. | High |
| D-24 | app.rs:346-350 | `Document::mark_saved` copies the buffer into `saved_buffer` and sets `dirty = false`; it does not change `revision`. | High |
| D-25 | app.rs:352-361 | `record_undo` pushes the current buffer/cursor, evicts the oldest entry when `undo.len() > UNDO_CAP`, and clears `redo`. | High |
| D-26 | app.rs:363-366 | `bump` does `self.revision = self.revision.saturating_add(1);` and `self.dirty = self.buffer != self.saved_buffer;`. | High |
| D-27 | app.rs:927-935 | `floor_boundary` returns the largest character boundary at or below a byte position, clamped to `text.len()`. | High |
| D-28 | app.rs:938-953 | `line_col_at` counts `'\n'` as a new line, ignores `'\r'`, and counts other characters as columns starting at 1. | High |
| C-01 | app.rs:171 | The command type is `pub enum Command {`. | High |
| C-02 | app.rs:173 | `NewDocument,` — add and select a new empty document. | High |
| C-03 | app.rs:175 | `Open(PathBuf),` — open a file in a new document. | High |
| C-04 | app.rs:177 | `Save,` — save the active document to its path. | High |
| C-05 | app.rs:179 | `SaveAs(PathBuf),` — save the active document to a supplied path. | High |
| C-06 | app.rs:181 | `CloseDocument,` — close the active document. | High |
| C-07 | app.rs:183 | `SelectTab(usize),` — select a tab by index. | High |
| C-08 | app.rs:185 | `Run,` — request a run of the active document. | High |
| C-09 | app.rs:187 | `Cancel,` — cancel the run in flight. | High |
| C-10 | app.rs:189 | `SetPrecision(usize),` — set the active document's display precision. | High |
| C-11 | app.rs:191 | `ClearVariables,` — discard the active document's variables. | High |
| C-12 | app.rs:193-194 | `QuickEval(String),` is annotated `#[allow(dead_code)] // part of the command surface; a quick-eval field can bind it later`. | High |
| C-13 | app.rs:196 | `Undo,` — undo the last buffer change. | High |
| C-14 | app.rs:198 | `Redo,` — redo the last undone change. | High |
| C-15 | app.rs:200 | `InsertText(String),` — insert text at the cursor. | High |
| C-16 | app.rs:564-581 | `AppState::apply` is a single exhaustive match that calls one private handler per variant (`Command::NewDocument => self.new_document()`, and so on) and returns `Vec<Effect>`. | High |
| S-01 | app.rs:147-154 | `RunRequest` carries exactly three fields: `pub document: usize`, `pub source: String`, `pub precision: usize`. | High |
| S-02 | app.rs:158-167 | `Effect` has exactly four variants: `RunRequested(RunRequest)`, `Saved(PathBuf)`, `Error(String)`, `Repaint`. | High |
| S-03 | app.rs:159 | The run effect is documented as `/// Evaluate this request (on a worker, or inline) and report back to finish_run.`. | High |
| S-04 | app.rs:128-133 | `RunState` has exactly two variants: `Idle` and `Running`. | High |
| S-05 | app.rs:137-142 | `RunState::label` returns `"idle"` for `Idle` and `"running"` for `Running`. | High |
| S-06 | app.rs:370-387 | `AppState` declares seven public fields (`documents`, `active`, `layout`, `logs`, `last_run_ms`, `run_state`, `settings`) plus private `pending` and `factory`. | High |
| S-07 | app.rs:385-386 | The pending run slot is `pending: Option<RunRequest>,` and the engine construction function is `factory: EngineFactory,`. | High |
| S-08 | app.rs:397-404 | `AppState::new()` uses `engine::default_engine_factory()`; `with_factory` accepts an explicit factory. | High |
| S-09 | app.rs:407-409 | `with_settings` uses the default factory and the supplied settings. | High |
| S-10 | app.rs:412-416 | `with_factory_and_settings` clamps `settings.active_precision` into `MIN_PRECISION..=MAX_PRECISION` and writes the clamped value back before creating the first document. | High |
| S-11 | app.rs:432-439 | `active_doc` and `active_doc_mut` index `self.documents` with `self.active` and return `Option`. | High |
| S-12 | app.rs:442-453 | `tab_title(index)` returns the path's `file_name` when a path exists, else `format!("Untitled {}", index + 1)`, and `"(no document)"` for a missing index. | High |
| S-13 | app.rs:456-466 | `window_title()` returns `format!("Lovelace Desktop - {}{}", self.tab_title(self.active), if dirty { " *" } else { "" })`. | High |
| S-14 | app.rs:469-483 | `status_text(line, column)` formats revision, precision, line, column, run state, last run milliseconds, and engine name; with no active document it returns `"no document"`. | High |
| S-15 | app.rs:486-515 | `variables_text()` renders `VARIABLES` then `  name = value` lines, then `FUNCTIONS` then `  name(params)` lines, using `  (none)` for empty lists. | High |
| S-16 | app.rs:518-533 | `logs_text()` writes one entry per line; entries whose `LogLevel::tag()` is empty are written raw, others as `[tag] text`. | High |
| S-17 | app.rs:536-544 | `push_log` evicts the oldest entry when `self.logs.len() >= LOG_CAP`, then pushes the new entry. | High |
| S-18 | app.rs:547-551 | `adopt_settings_warnings` drains `self.settings.take_warnings()` into `LogLevel::Warn` entries. | High |
| S-19 | app.rs:554-556 | `apply_layout` stores `compute_layout(client_w, client_h, &self.settings.layout)` in `self.layout`. | High |
| S-20 | app.rs:559-561 | `pending_run()` returns `self.pending.as_ref()` as `Option<&RunRequest>`. | High |
| R-01 | app.rs:585-590 | `set_active_text(text)` is declared as a view callback, not a command; it calls `document.set_text_from_editor(text)` when an active document exists and always returns `vec![Effect::Repaint]`. | High |
| R-02 | app.rs:592-594 | The `run_blocking` doc comment states: `/// Requests a run, evaluates it immediately on this thread and publishes the result. The`, and says the shell uses `apply` then `finish_run` separately `so that a worker could sit between them`. | High |
| R-03 | app.rs:596-610 | `run_blocking` applies `Command::Run`, finds `Effect::RunRequested`, calls `document.engine_mut().eval_full(&request.source)`, measures `started.elapsed()`, and passes the outcome to `finish_run`. | High |
| R-04 | app.rs:604-609 | The only timing instrumentation is `std::time::Instant::now()` before evaluation and `started.elapsed().as_millis() as u64` after it. | High |
| R-05 | app.rs:616-621 | `finish_run` signature: `pub fn finish_run(&mut self, document: usize, outcome: RunOutput, elapsed_ms: u64) -> Vec<Effect>`. | High |
| R-06 | app.rs:622-628 | A result is accepted only when `(Some(pending), RunState::Running)` matches and `pending.document == document`; otherwise `finish_run` returns an empty vector and publishes nothing. | High |
| R-07 | app.rs:629-631 | An accepted result clears `self.pending = None`, sets `self.run_state = RunState::Idle`, and stores `self.last_run_ms = elapsed_ms`. | High |
| R-08 | app.rs:633-648 | The outcome is destructured into `result`, `output`, `variables`, `functions`, `plot`; `active.vars`, `active.funcs` are replaced and `active.last_plot` is set only when `plot` is `Some`. | High |
| R-09 | app.rs:644-646 | A run that produces no plot leaves the previous `last_plot` in place: `if let Some(plot) = plot { active.last_plot = Some(plot); }`. | High |
| R-10 | app.rs:647 | A published run increments the document revision: `active.revision = active.revision.saturating_add(1);`. | High |
| R-11 | app.rs:649-651 | Each engine output line is appended as `LogLevel::Output` with no prefix. | High |
| R-12 | app.rs:652-658 | `Ok(display)` logs `LogLevel::Result` with text `format!("= {display}")`; `Err(message)` logs `LogLevel::Error` with `format!("error: {message}")` and appends `Effect::Error(message)`. | High |
| R-13 | app.rs:659 | Every accepted run appends `Effect::Repaint` to the effect list. | High |
| R-14 | app.rs:789-793 | `request_run` rejects a second run while `run_state == RunState::Running` with the warning `"A run is already in progress."` and returns only `Repaint`. | High |
| R-15 | app.rs:794-800 | `request_run` returns only `Repaint` when there is no active document; when the active buffer `trim()` is empty it logs `"Nothing to run: the document is empty."` and returns only `Repaint`. | High |
| R-16 | app.rs:801-805 | An accepted request clones the buffer and records the document index and precision: `RunRequest { document: self.active, source: document.buffer.clone(), precision: document.precision }`. | High |
| R-17 | app.rs:806-813 | An accepted request stores `self.pending = Some(request.clone());`, sets `self.run_state = RunState::Running;`, logs `Run started (revision N).`, and returns `vec![Effect::RunRequested(request), Effect::Repaint]`. | High |
| R-18 | app.rs:816-825 | `cancel_run` only resets `pending = None` and `run_state = Idle` when running (logging `"Run cancelled."`), or logs `"Nothing to cancel."`; it makes no engine call and always returns `Repaint`. | High |
| R-19 | app.rs:838-849 | `clear_variables` calls `document.engine_mut().clear()`, clears `document.vars`, re-reads `document.funcs = document.engine().functions()`, and increments `document.revision`. | High |
| R-20 | app.rs:851-855 | `quick_eval` trims its input and returns only `Repaint` when the result is empty. | High |
| R-21 | app.rs:856-860 | `quick_eval` logs the source as `Info` with prefix `"> "`, then calls `document.engine_mut().eval_full(source)` inline; with no active document it returns `Repaint`. | High |
| R-22 | app.rs:861-887 | `quick_eval` publishes `vars`, `funcs`, and `last_plot` the same way as `finish_run`, logs output/result/error, and appends `Repaint`; it makes no `revision` assignment. | High |
| R-23 | app.rs:890-910 | `undo` and `redo` call the active document's method, log `"Nothing to undo."` or `"Nothing to redo."` on failure, and always return `Repaint`. | High |
| R-24 | app.rs:912-917 | `insert_text` calls `document.insert_text(text)` when an active document exists and always returns `Repaint`. | High |
| E-01 | app.rs:564-581 | `apply` performs each state transition inline on its caller's thread; no thread spawn, channel, or task API appears in the function. | High |
| E-02 | app.rs:159 | Whether evaluation actually runs on a worker or inline is left to the effect consumer by the `RunRequested` doc comment. | High |
| E-03 | app.rs:680 | `Command::Open` performs file I/O inline inside `apply`: `match std::fs::read_to_string(&path)`. | High |
| E-04 | app.rs:722 | `Save`/`SaveAs` perform file I/O inline inside `apply`: `match std::fs::write(&path, document.buffer.as_bytes())`. | High |
| E-05 | app.rs:605-606 | The run path reads only `request.document` and `request.source`; `request.precision` is not read by `run_blocking`. | High |
| E-06 | engine.rs:67 | The trait is declared `pub trait Evaluator {` with no `Send` or `Sync` supertrait bound. | High |
| E-07 | app.rs:816-825 | `cancel_run` contains no call into `Evaluator`; cancellation is represented only by clearing `pending`/`run_state`. | High |
| E-08 | app.rs:1-1439 | No autosave, timer, file-watcher, or periodic-save code appears anywhere in `app.rs`; the file was read in full. | High |
| P-01 | engine.rs:34 | Default precision is `pub const DEFAULT_PRECISION: usize = 1000;`. | High |
| P-02 | settings.rs:26 | Minimum accepted precision is `pub const MIN_PRECISION: usize = 1;`. | High |
| P-03 | settings.rs:28 | Maximum accepted precision is `pub const MAX_PRECISION: usize = 100_000;`. | High |
| P-04 | app.rs:239 | `Document::new` clamps: `let precision = precision.clamp(MIN_PRECISION, MAX_PRECISION);`. | High |
| P-05 | app.rs:269-271 | `Document::set_precision` clamps and forwards: `self.precision = decimals.clamp(MIN_PRECISION, MAX_PRECISION); self.engine.set_precision(self.precision);`. | High |
| P-06 | app.rs:827-832 | The `SetPrecision` handler clamps: `let decimals = decimals.clamp(MIN_PRECISION, MAX_PRECISION);` and calls `document.set_precision(decimals);`. | High |
| P-07 | app.rs:833-834 | `SetPrecision` writes the setting and logs `Precision set to {decimals}.`: `self.settings.active_precision = decimals;`. | High |
| P-08 | app.rs:413-416 | Startup clamps the persisted precision and writes it back: `settings.active_precision = precision;`. | High |
| P-09 | app.rs:664-668 | `NewDocument` inherits the active document's precision, falling back to `settings.active_precision`, then clamps. | High |
| P-10 | app.rs:682-685 | `Open` takes precision from `self.settings.active_precision`, not from the active document. | High |
| P-11 | app.rs:919-923 | `sync_active_precision` copies the active document's precision into `self.settings.active_precision`. | High |
| P-12 | app.rs:773,785 | `close_document` and `select_tab` both call `self.sync_active_precision();`. | High |
| P-13 | engine/fake.rs:192-194 | The fake engine's setter only enforces a lower bound: `self.precision = decimals.max(1);`. | High |
| P-14 | engine/fake.rs:30-36 | The fake engine's `Value::display` formats `Value::Int(value)` with `value.to_string()` and never reads `precision`. | High |
| P-15 | engine/real.rs:49-55 | `RealEngine` delegates both directions: `self.engine.set_precision(decimals);` and `self.engine.precision()`. | High |
| P-16 | app.rs:804 | Every `RunRequest` records the document precision in `precision: document.precision`. | High |
| P-17 | app.rs:471-480 | The status bar includes precision: `document.precision` is formatted into `precision {}`. | High |
| P-18 | app.rs:827-836 | The `SetPrecision` handler contains no `revision` assignment. | High |
| W-01 | engine.rs:98-100 | The default workspace is empty: `fn variables(&self) -> Vec<(String, String)> { Vec::new() }`. | High |
| W-02 | engine.rs:103-105 | The default function list is empty: `fn functions(&self) -> Vec<(String, Vec<String>)> { Vec::new() }`. | High |
| W-03 | engine/fake.rs:50 | Fake engine variables are stored as `vars: BTreeMap<String, Value>,`. | High |
| W-04 | engine/fake.rs:204-209 | `FakeEngine::variables` maps every `BTreeMap` entry to `(name, value.display())`. | High |
| W-05 | engine/fake.rs:211-213 | `FakeEngine::functions` returns `builtin_functions()` and nothing else. | High |
| W-06 | engine/fake.rs:235-244 | The builtin list is exactly `abs(x)`, `clear()`, `max(a, b)`, `min(a, b)`, `plot(values)`, `print(values)`. | High |
| W-07 | engine/fake.rs:162-165 | The fake engine's `clear` builtin clears variables only: `self.vars.clear();`. | High |
| W-08 | engine/fake.rs:223-227 | `Evaluator::clear` on the fake engine clears `vars`, clears `plot`, and increments `revision`. | High |
| W-09 | engine/real.rs:61-67 | `RealEngine` forwards `variables()` and `functions()` directly to `lovelace_lang::Engine`. | High |
| W-10 | engine/real.rs:81-85 | `RealEngine::clear` rebuilds the engine and restores precision: `self.engine = lovelace_lang::Engine::new(); self.engine.set_precision(precision);`. | High |
| W-11 | app.rs:641-645 | A published run replaces the document workspace: `active.vars = variables; active.funcs = functions;`. | High |
| W-12 | engine/fake.rs:528-553 | Every fake-engine call is dispatched to `engine.builtin(name, args)`; the parser has no user-function definition path. | High |
| L-01 | app.rs:19 | The log cap is `pub const LOG_CAP: usize = 500;`. | High |
| L-02 | app.rs:91-102 | Log levels are exactly `Info`, `Warn`, `Error`, `Output`, `Result`. | High |
| L-03 | app.rs:106-114 | Tags are `info`, `warn`, `error`, and the empty string for both `Output` and `Result`. | High |
| L-04 | app.rs:536-544 | `push_log` evicts with `self.logs.remove(0)` when at capacity and appends `LogEntry { level, text: text.into() }`. | High |
| L-05 | app.rs:518-533 | `logs_text` omits the bracket prefix when `entry.level.tag().is_empty()` and otherwise writes `[tag] text`. | High |
| L-06 | app.rs:649-658 | A completed run logs output lines as `Output`, a successful display as `Result` (`= ...`), and a failure as `Error` (`error: ...`). | High |
| L-07 | app.rs:876-885 | `QuickEval` uses the same output/result/error mapping as a completed run, after logging the source line. | High |
| L-08 | engine/fake.rs:100-110 | `print` joins argument displays with single spaces, pushes the line into `self.output`, and returns `Value::Void`. | High |
| L-09 | engine/fake.rs:215-217 | `take_output` drains the buffer: `std::mem::take(&mut self.output)`. | High |
| L-10 | engine.rs:72-80 | The default `eval_full` calls `self.eval(src)` and then snapshots `take_output`, `variables`, `functions`, and `last_plot`; the `RunOutput` is built even when the result is `Err`. | High |
| L-11 | app.rs:547-551 | Settings load warnings are copied into the log by `adopt_settings_warnings`. | High |
| L-12 | app.rs:809-812,817-823 | Run start logs `Run started (revision N).`; cancel logs `Run cancelled.` or `Nothing to cancel.`. | High |
| L-13 | app.rs:672-675,693,698,734,738,774,834,847 | Document/command info and error lines include `New document (precision N).`, `Opened ...`, `Cannot open ...`, `Saved ...`, `Cannot save ...`, `Document closed.`, `Precision set to N.`, `Variables cleared.`. | High |
| SET-01 | settings.rs:15 | The settings schema is `pub const SCHEMA_VERSION: u32 = 1;`. | High |
| SET-02 | settings.rs:18-22 | Window limits are `MIN_WINDOW_WIDTH = 900`, `MIN_WINDOW_HEIGHT = 600`, `MAX_WINDOW_DIMENSION = 10_000`. | High |
| SET-03 | settings.rs:24 | `pub const MAX_RECENT_FILES: usize = 20;`. | High |
| SET-04 | settings.rs:31-37 | `WindowGeometry` has `pub width: i32` and `pub height: i32`. | High |
| SET-05 | settings.rs:39-46 | Its default is `WindowGeometry { width: 1280, height: 800 }`. | High |
| SET-06 | settings.rs:52-60 | `LayoutFractions` has three `u32` permille fields: `editor_width_permille`, `logs_height_permille`, `plot_height_permille`. | High |
| SET-07 | settings.rs:62-70 | Its default is `720`, `220`, `460` permille. | High |
| SET-08 | settings.rs:73-87 | `Settings` has public fields `schema_version`, `window`, `layout`, `recent_files`, `active_precision`, `warnings`. | High |
| SET-09 | settings.rs:89-100 | `Settings::default()` uses schema 1, default geometry, default layout, empty `recent_files`, `DEFAULT_PRECISION`, empty `warnings`. | High |
| SET-10 | settings.rs:110-147 | `to_json` serialises the keys `schema_version`, `window` (`width`, `height`), `layout` (three permille keys), `recent_files`, and `active_precision`. | High |
| SET-11 | settings.rs:112 | The serialised schema is the constant, not the field: `("schema_version", Json::Num(SCHEMA_VERSION.to_string())),`. | High |
| SET-12 | settings.rs:110-147 | `warnings` is not serialised; no `warnings` key is emitted. | High |
| SET-13 | settings.rs:151-167 | `from_json` has the return type `-> Settings` (no error type); a non-object root or invalid JSON pushes a warning and returns the default value. | High |
| SET-14 | settings.rs:169-193 | A missing/non-numeric `schema_version` returns defaults; a version newer than 1 returns defaults; an older version warns and continues. | High |
| SET-15 | settings.rs:195-217 | `window.width` is read in `900..=10_000` and `window.height` in `600..=10_000`. | High |
| SET-16 | settings.rs:218-247 | Layout fields are read in `300..=900`, `100..=600`, and `150..=850` permille. | High |
| SET-17 | settings.rs:248-255 | `active_precision` is read in `1..=100_000`. | High |
| SET-18 | settings.rs:256-274 | `recent_files` is read only as an array of strings, trimmed, de-duplicated in file order, and truncated to `MAX_RECENT_FILES`. | High |
| SET-19 | settings.rs:280-293 | `load` returns `Settings::new()` with no warning for `NotFound`, and defaults with a warning for any other read error. | High |
| SET-20 | settings.rs:296-305 | `save` creates parent directories, appends a newline to `to_json()`, and calls `std::fs::write(path, text)`. | High |
| SET-21 | settings.rs:308-316 | `add_recent` trims, ignores empty paths, removes duplicates, inserts at index 0, and truncates to `MAX_RECENT_FILES`. | High |
| SET-22 | settings.rs:319-321 | `take_warnings` uses `std::mem::take(&mut self.warnings)`. | High |
| SET-23 | settings.rs:326-358 | `read_i64` clamps out-of-range numbers with a warning, uses the default with a warning for wrong types, and warns for missing fields. | High |
| SET-24 | settings.rs:362-376 | The config path is `%APPDATA%\Lovelace\desktop.json` when `APPDATA` exists, else an XDG path, else `$HOME/.config/lovelace/desktop.json`. | High |
| SET-25 | settings.rs:151-277 | The parsed version is validated but never assigned to `settings.schema_version`; the field keeps the `Settings::default()` value of 1. | High |
| SET-26 | app.rs:1-1439 | `app.rs` contains no call to `Settings::load` or `Settings::save`; `AppState` only stores and mutates a `Settings` value. | High |
| F-01 | app.rs:680 | `Open` reads the whole file as UTF-8 text: `match std::fs::read_to_string(&path)`. | High |
| F-02 | app.rs:681-694 | On a successful read, `Open` creates a new document with `Document::new((self.factory)(), precision)`, sets `document.buffer = text`, `document.mark_saved()`, and `document.path = Some(path.clone())`. | High |
| F-03 | app.rs:682-685 | The new document's precision comes from `self.settings.active_precision`. | High |
| F-04 | app.rs:690-691 | The new document is appended and selected: `self.documents.push(document); self.active = self.documents.len() - 1;`. | High |
| F-05 | app.rs:692-693 | `Open` records the path with `self.settings.add_recent(&path.to_string_lossy());` and logs `format!("Opened {}", path.display())`. | High |
| F-06 | app.rs:681-694 | An opened document's `revision` remains 0; the success path assigns `buffer` and calls `mark_saved()` but never `bump()`. | High |
| F-07 | app.rs:686-689 | An opened document's `cursor` remains 0; the success path never sets the cursor. | High |
| F-08 | app.rs:686-691 | `Open` appends a new document without checking whether the path is already open in another document. | High |
| F-09 | app.rs:696-700 | A failed open builds `format!("Cannot open {}: {error}", path.display())`, logs it as `Error`, and returns `[Effect::Error(message), Effect::Repaint]` without adding a document. | High |
| F-10 | app.rs:704-712 | `Save` with no path pushes a warning and returns `[Effect::Error("This document has no path yet; use Save As."), Effect::Repaint]`. | High |
| F-11 | app.rs:715-717 | `SaveAs(path)` delegates directly to `self.write_active(path)`. | High |
| F-12 | app.rs:719-730 | `write_active` writes `document.buffer.as_bytes()` to the supplied path, then sets `document.path` and calls `mark_saved()`; it does not change `revision`. | High |
| F-13 | app.rs:722 | The write is a direct `std::fs::write` to the target path, inside the state transition. | High |
| F-14 | app.rs:723-726 | On write success, the path is replaced and the document is marked saved: `document.path = Some(path.clone()); document.mark_saved();`. | High |
| F-15 | app.rs:733-736 | On write success, `add_recent` is called, the save is logged, and the effects are `[Effect::Saved(path), Effect::Repaint]`. | High |
| F-16 | app.rs:737-740 | On write failure, an error is logged and the effects are `[Effect::Error(message), Effect::Repaint]`; path, buffer, and dirty flag are unchanged. | High |
| F-17 | app.rs:719-742 | `write_active` contains no temporary-file, rename, fsync, or backup step; it writes directly to the destination. | High |
| F-18 | app.rs:719-730 | `write_active` does not test `document.dirty` before writing. | High |
| F-19 | app.rs:744-755 | `CloseDocument` refuses a dirty active document with the message `"Cannot close {}: it has unsaved changes. Save it first."` (formatted with the tab title) and returns `Error` plus `Repaint`. | High |
| F-20 | app.rs:756-761 | A successful close removes the document at `self.active`; if the pending run's `document` equals the removed index, `pending` is cleared and `run_state = Idle`. | High |
| F-21 | app.rs:762-769 | Closing the last document creates a replacement with a fresh engine and `settings.active_precision`, then sets `active = 0`. | High |
| F-22 | app.rs:770-773 | After a close, `active` is clamped to `len - 1` and `sync_active_precision()` is called. | High |
| F-23 | app.rs:778-787 | `SelectTab(index)` rejects `index >= documents.len()` with `No tab at index {index}.`; otherwise it sets `active = index` and syncs precision. | High |
| F-24 | app.rs:1-1439 | No autosave, autosave interval, draft file, or crash-recovery write appears anywhere in `app.rs`. | High |
| F-25 | app.rs:1-1439 | No file-watcher, modification-time check, or external-change detection appears anywhere in `app.rs`. | High |
| F-26 | app.rs:744-776 | `close_document` does not rebase `pending.document` after removing a document; the only `pending` write in the function is `self.pending = None;` inside the branch where the pending index equals the closed index. | High |
| F-27 | app.rs:778-787 | `select_tab` changes only `self.active` and `settings.active_precision`; it contains no read or write of `pending`, so an in-flight run keeps its original document index. | High |
| Y-01 | app.rs:25 | Pane gap is `pub const PANE_GAP: i32 = 6;`. | High |
| Y-02 | app.rs:29-52 | `LayoutSizes` has public `i32` fields `editor_w`, `editor_h`, `right_w`, `plot_h`, `logs_h`; defaults are `800`, `420`, `340`, `260`, `180`. | High |
| Y-03 | app.rs:56-58 | Signature: `pub fn compute_layout(client_w: i32, client_h: i32, fractions: &LayoutFractions) -> LayoutSizes`, with `let width = client_w.max(1); let height = client_h.max(1);`. | High |
| Y-04 | app.rs:60-62 | Log height is `logs_raw.max(60).min((height / 2).max(60)).min(height)`, then `top_h = (height - logs_h - PANE_GAP).max(0)`. | High |
| Y-05 | app.rs:64-68 | Editor width is clamped to `min_editor = 240.min(width)` and `max_editor = (width - 240 - PANE_GAP).max(min_editor).min(width)`, then `right_w = (width - editor_w - PANE_GAP).max(0)`. | High |
| Y-06 | app.rs:70-78 | Plot height is `raw.clamp(low, high.max(low))` with `low = 80.min(top_h)` and `high = (top_h - 80).max(low).min(top_h)`; `editor_h = (top_h - plot_h - PANE_GAP).max(0)`. | High |
| Y-07 | app.rs:554-556 | `apply_layout` recomputes the whole layout from the saved fractions on every call. | High |
| EN-01 | engine.rs:67-69 | The trait's only required method is `fn eval(&mut self, src: &str) -> Result<String, String>;`. | High |
| EN-02 | engine.rs:72-81 | `eval_full` is a default trait method that calls `eval` once and snapshots output, variables, functions, and plot. | High |
| EN-03 | engine.rs:84 | `set_precision` defaults to a no-op: `fn set_precision(&mut self, _decimals: usize) {}`. | High |
| EN-04 | engine.rs:87-89 | `precision()` defaults to `DEFAULT_PRECISION`. | High |
| EN-05 | engine.rs:93-95 | `revision()` defaults to `0`. | High |
| EN-06 | engine.rs:98-105 | `variables()` and `functions()` default to empty vectors. | High |
| EN-07 | engine.rs:108-115 | `take_output()` defaults to empty; `last_plot()` defaults to `None`. | High |
| EN-08 | engine.rs:118-123 | `clear()` defaults to a no-op and `name()` defaults to `"evaluator"`. | High |
| EN-09 | engine.rs:127 | The factory type is `pub type EngineFactory = fn() -> Box<dyn Evaluator>;`. | High |
| EN-10 | engine.rs:37-45 | `PlotCapture` has `pub path: String`, `pub title: String`, `pub svg: String`. | High |
| EN-11 | engine.rs:48-60 | `RunOutput` has `pub result: Result<String, String>`, `pub output: Vec<String>`, `pub variables: Vec<(String, String)>`, `pub functions: Vec<(String, Vec<String>)>`, `pub plot: Option<PlotCapture>`. | High |
| EN-12 | engine/fake.rs:49-56 | `FakeEngine` stores `vars`, `precision`, `revision`, `output`, `plot`, and a `plots` counter. | High |
| EN-13 | engine/fake.rs:66-75 | `FakeEngine::new()` starts with empty vars/output, `precision: DEFAULT_PRECISION`, `revision: 0`, `plot: None`, `plots: 0`. | High |
| EN-14 | engine/fake.rs:172-190 | `FakeEngine::eval` splits the source into statements, evaluates each in order, remembers the last display, and returns it or `"Void"`. | High |
| EN-15 | engine/fake.rs:182-188 | `FakeEngine` increments its revision once on an error return and once on a successful return. | High |
| EN-16 | engine/fake.rs:98-168 | Builtin dispatch handles `print`, `plot`, `abs`, `min`, `max`, and `clear`; any other name returns `Unknown function '{other}'.`. | High |
| EN-17 | engine/fake.rs:111-134 | `plot` takes text as the title, integer arguments as values, requires at least two values, increments `plots`, builds `svg::make_plot_svg(&title, &ys)`, and stores a `PlotCapture`. | High |
| EN-18 | engine/fake.rs:128-132 | The fake plot path is the string `format!("plots/plot-{}.svg", self.plots)`; no filesystem write occurs in `fake.rs`. | High |
| EN-19 | engine/fake.rs:248-282 | `split_statements` splits at `;` and newline only outside strings and brackets. | High |
| EN-20 | engine/fake.rs:285-322 | `split_assignment` recognises `name = expression` at depth 0 and rejects `==`. | High |
| EN-21 | engine/fake.rs:431-474 | Expressions are parsed with `+`/`-` at the outer level and `*`/`/`/`%` in `term`, so multiplication binds tighter than addition. | High |
| EN-22 | engine/real.rs:17-33 | `boxed()` returns `Box::new(RealEngine::new())`; `RealEngine` owns one `lovelace_lang::Engine` created by `Engine::new()`. | High |
| EN-23 | engine/real.rs:42-47 | `RealEngine::eval` maps `Ok(value)` through `self.engine.format(&value)` and `Err(error)` to `Err(error.message)`. | High |
| EN-24 | engine/real.rs:49-59 | `RealEngine` forwards `set_precision`, `precision`, and `revision` to the wrapped engine. | High |
| EN-25 | engine/real.rs:61-71 | `RealEngine` forwards `variables`, `functions`, and `take_output`. | High |
| EN-26 | engine/real.rs:73-79 | `RealEngine::last_plot` maps the engine plot into `PlotCapture { path, title, svg }`. | High |
| EN-27 | engine/real.rs:81-85 | `RealEngine::clear` rebuilds the engine and restores the previous precision. | High |
| EN-28 | engine/real.rs:87-89 | `RealEngine::name` returns `"lovelace-lang"`. | High |
| EN-29 | engine/fake.rs:229-231 | `FakeEngine::name` returns `"fake"`. | High |
| EN-30 | engine/fake.rs:171-232; engine/real.rs:41-89 | Neither `FakeEngine` nor `RealEngine` overrides `eval_full`; both `impl Evaluator` blocks end after `name`, so both use the default composition at engine.rs:72-81. | High |

## 2. Commands and state transitions

`AppState::apply(Command)` is the single command dispatcher (app.rs:564-581). It runs synchronously on the caller's thread. `Thread` below records where the handler body executes; only `Run` defers evaluation to the consumer of `Effect::RunRequested`.

### 2.1 `Command` variants (14)

| Command | Inputs | Effect on document state | Thread it runs on | Rejection/queue rule (if any) | Citation |
|---|---|---|---|---|---|
| `NewDocument` | none | Appends a fresh `Document` (new engine, precision inherited from the active document, else `settings.active_precision`), selects it, and logs `New document (precision N).` | Calling thread (handler inline) | None | app.rs:566,663-677 |
| `Open(path)` | `PathBuf` | On success: appends a new `Document` whose buffer is the file text, path is `path`, state is clean, precision is `settings.active_precision`, cursor 0, revision 0; selects it; records it in `recent_files`; logs `Opened ...`. On error: no document change. | Calling thread; `std::fs::read_to_string` runs inline | I/O error only; no duplicate-path check and no dialog. | app.rs:566,679-702 |
| `Save` | none | Writes the active buffer to its existing path; sets `path` to the same path; `mark_saved()` clears `dirty`; `revision` unchanged; logs `Saved ...`; emits `Effect::Saved(path)`. | Calling thread; `std::fs::write` runs inline | No path: warning + `Effect::Error`, no write. Write error: `Effect::Error`, state unchanged. | app.rs:568,704-713,719-742 |
| `SaveAs(path)` | `PathBuf` | Writes the active buffer to `path`; sets `document.path = Some(path)`; `mark_saved()` clears `dirty`; `revision` unchanged; logs and emits `Effect::Saved(path)`. | Calling thread; `std::fs::write` runs inline | No active document: `Effect::Error("No document is open.")`; write error: `Effect::Error`, state unchanged. | app.rs:569,715-717,719-742 |
| `CloseDocument` | none | Removes the active document. If the list becomes empty, creates a replacement document with `settings.active_precision`. Clamps `active`, syncs `settings.active_precision`, logs `Document closed.` | Calling thread (handler inline) | Dirty active document: warning + `Effect::Error`, no removal. If the removed index equals `pending.document`, the pending run is cleared and `run_state = Idle`. | app.rs:570,744-776 |
| `SelectTab(index)` | `usize` | Sets `active = index` and syncs `settings.active_precision`; document contents, vars, funcs, plot, and revision unchanged. | Calling thread (handler inline) | `index >= documents.len()`: warning + `Effect::Error`, `active` unchanged. | app.rs:571,778-787 |
| `Run` | none | No engine call. Captures a `RunRequest` with `document: self.active`, `source: document.buffer.clone()`, and `precision: document.precision`; sets `self.pending = Some(request.clone())` and `self.run_state = RunState::Running`; logs `Run started (revision N).`; emits `RunRequested(request)` and `Repaint`. | Calling thread for the state change; evaluation is deferred to the effect consumer (`app.rs:159`) | If already `Running`: warning `A run is already in progress.`, no request. If `buffer.trim().is_empty()`: warning `Nothing to run: the document is empty.`, no request. Only one `pending` slot; no queue. | app.rs:572,789-814 |
| `Cancel` | none | If `Running`: `pending = None`, `run_state = Idle`, log `Run cancelled.`. Otherwise log `Nothing to cancel.`. No engine call. | Calling thread (handler inline) | None; always returns `Repaint`. | app.rs:573,816-825 |
| `SetPrecision(decimals)` | `usize` | Clamps to `1..=100_000`; sets active `document.precision` and forwards to the engine; sets `settings.active_precision = decimals`; logs `Precision set to N.`; `revision` unchanged. | Calling thread (handler inline) | None; out-of-range values are clamped. No active document: `Repaint` only. | app.rs:574,827-836 |
| `ClearVariables` | none | Calls `engine.clear()`; clears `document.vars`; replaces `document.funcs` with `engine.functions()`; increments `document.revision`; logs `Variables cleared.`. | Calling thread (handler inline) | None. No active document: `Repaint` only. | app.rs:575,838-849 |
| `QuickEval(source)` | `String` | Trims the text; logs `> source`; evaluates inline via `engine.eval_full`; replaces `vars`/`funcs` and conditional `last_plot`; logs output/result/error; appends `Error` effect on failure. Buffer, path, dirty, cursor, and revision unchanged. | Calling thread; `eval_full` runs inline | Empty after trim: `Repaint` only, no log. Command is marked `#[allow(dead_code)]` at app.rs:193. | app.rs:576,851-888 |
| `Undo` | none | Pops one undo snapshot, moves current state to redo, restores buffer and cursor, increments `revision`, recomputes `dirty`; logs `Nothing to undo.` when empty. | Calling thread (handler inline) | Empty undo stack: warning only. No queue. | app.rs:577,890-899; app.rs:308-324 |
| `Redo` | none | Pops one redo snapshot, moves current state to undo, restores buffer and cursor, increments `revision`, recomputes `dirty`; logs `Nothing to redo.` when empty. | Calling thread (handler inline) | Empty redo stack: warning only. No queue. | app.rs:578,901-910; app.rs:327-343 |
| `InsertText(text)` | `String` | If non-empty: records one undo snapshot, inserts at the cursor, advances cursor by `text.len()`, increments `revision`, recomputes `dirty`; no log. | Calling thread (handler inline) | Empty string: no-op (`Document::insert_text` returns immediately). | app.rs:579,912-917; app.rs:285-293 |

### 2.2 Public non-`Command` state transitions (15)

| Command / method | Inputs | Effect on document state | Thread it runs on | Rejection/queue rule (if any) | Citation |
|---|---|---|---|---|---|
| `AppState::set_active_text(text)` | `&str` | Calls `active_document.set_text_from_editor(text)`; full-buffer replacement, one undo entry when the text differs; always `Repaint`. | Calling thread (view callback) | No active document: no state change, still `Repaint`. | app.rs:585-590 |
| `AppState::run_blocking()` | none | Applies `Command::Run`, finds the `RunRequest`, evaluates it inline with `eval_full`, measures elapsed ms, and publishes through `finish_run`. | Calling thread (`app.rs:592-594`) | If `Run` is rejected, returns its effects without evaluating. | app.rs:595-612 |
| `AppState::finish_run(document, outcome, elapsed_ms)` | `usize`, `RunOutput`, `u64` | Accepts only a matching pending run; publishes `vars`/`funcs`/conditional plot, increments document revision, appends output/result/error logs, clears pending, sets `Idle`, sets `last_run_ms`. | Calling thread; the `RunOutput` value may be produced elsewhere | Returns empty `Vec` unless `pending.document == document` and `run_state == Running`; stale/cancelled results are discarded. | app.rs:616-661 |
| `AppState::apply_layout(client_w, client_h)` | `i32`, `i32` | Replaces `self.layout` from `compute_layout`; no document change. | Calling thread | None | app.rs:554-556 |
| `AppState::push_log(level, text)` | `LogLevel`, `Into<String>` | Appends to `self.logs`, evicting the oldest entry at `LOG_CAP`; no document change. | Calling thread | At capacity, oldest entry is removed first. | app.rs:536-544 |
| `AppState::adopt_settings_warnings()` | none | Drains `settings.warnings` into `LogLevel::Warn` log entries; no document change. | Calling thread | None | app.rs:547-551 |
| `Document::set_text_from_editor(text)` | `&str` | If text differs: records undo, replaces the whole buffer, clamps cursor, `bump()` (revision +1, dirty recomputed). | Calling thread | Identical text: no-op (no undo entry, no revision change). | app.rs:296-305 |
| `Document::insert_text(text)` | `&str` | If non-empty: records undo, inserts at cursor, advances cursor by byte length, `bump()`. | Calling thread | Empty text: no-op. | app.rs:285-293 |
| `Document::undo()` | none | Restores the previous buffer/cursor and bumps revision; returns `bool`. | Calling thread | Empty stack: returns `false`, no change. | app.rs:308-324 |
| `Document::redo()` | none | Restores the next buffer/cursor and bumps revision; returns `bool`. | Calling thread | Empty stack: returns `false`, no change. | app.rs:327-343 |
| `Document::mark_saved()` | none | Copies `buffer` into `saved_buffer`, sets `dirty = false`; revision unchanged. | Calling thread | None | app.rs:346-350 |
| `Document::set_cursor(position)` | `usize` | Sets `cursor` to `floor_boundary(buffer, position)`; no revision or dirty change. | Calling thread | Position is clamped to `buffer.len()` and onto a character boundary. | app.rs:275-277 |
| `Document::set_precision(decimals)` | `usize` | Clamps precision and forwards to the engine; does not update `settings` and does not bump revision. | Calling thread | Values outside `1..=100_000` are clamped. | app.rs:269-272 |
| `Settings::add_recent(path)` | `&str` | Trims; if non-empty removes an existing equal string, inserts at index 0, truncates to 20; no document change. | Calling thread | Empty path: no-op. | settings.rs:308-316 |
| `Settings::take_warnings()` | none | Returns and empties `settings.warnings`; no document change. | Calling thread | None | settings.rs:319-321 |

## 3. Fields exposed to a UI

| Field | Type | Where computed | Citation |
|---|---|---|---|
| `AppState.documents` | `Vec<Document>` | Stored; appended by `NewDocument`/`Open`, removed by `CloseDocument` | app.rs:372 |
| `AppState.active` | `usize` | Stored; set on new/open/select/close | app.rs:374 |
| `AppState.layout` | `LayoutSizes` | `apply_layout` → `compute_layout` | app.rs:376,554-556 |
| `AppState.logs` | `Vec<LogEntry>` | Appended by `push_log` and every logging handler | app.rs:378 |
| `AppState.last_run_ms` | `u64` | Set by `finish_run` from `elapsed_ms` | app.rs:380,631 |
| `AppState.run_state` | `RunState` | `Idle` initially; changed by `Run`, `Cancel`, `finish_run`, `CloseDocument` | app.rs:382 |
| `AppState.settings` | `Settings` | Stored; mutated by `SetPrecision`, `Open`, `Save`, `CloseDocument`, `SelectTab` | app.rs:384 |
| `AppState::pending_run()` | `Option<&RunRequest>` | `self.pending.as_ref()` | app.rs:559-561 |
| `Document.path` | `Option<PathBuf>` | `None` on creation; set by `Open`/`write_active` | app.rs:206 |
| `Document.buffer` | `String` | Set by creation, `Open`, `insert_text`, `set_text_from_editor`, undo/redo | app.rs:208 |
| `Document.dirty` | `bool` | `bump()` comparison; `mark_saved()` clears | app.rs:210,363-366,346-350 |
| `Document.precision` | `usize` | Creation, `set_precision`, `SetPrecision` | app.rs:212 |
| `Document.revision` | `u64` | `bump()`, accepted `finish_run`, `clear_variables` | app.rs:214 |
| `Document.cursor` | `usize` | Creation, `insert_text`, `set_text_from_editor`, undo/redo, `set_cursor` | app.rs:216 |
| `Document.vars` | `Vec<(String, String)>` | `finish_run`, `QuickEval`, `ClearVariables` | app.rs:218 |
| `Document.funcs` | `Vec<(String, Vec<String>)>` | `finish_run`, `QuickEval`, `ClearVariables` | app.rs:220 |
| `Document.last_plot` | `Option<PlotCapture>` | `finish_run`/`QuickEval` only when a new plot exists; `clear_variables` does not clear it | app.rs:222 |
| `Document::engine()` / `Document::engine_mut()` | `&dyn Evaluator` / `&mut dyn Evaluator` | Direct borrow of the private engine | app.rs:259-266 |
| `Document::line_col()` | `(usize, usize)` | `line_col_at(buffer, cursor)` | app.rs:280-282 |
| `AppState::tab_title(index)` | `String` | `path.file_name()` or `Untitled N` | app.rs:442-453 |
| `AppState::window_title()` | `String` | `"Lovelace Desktop - {tab_title}{ * if dirty}"` | app.rs:456-466 |
| `AppState::status_text(line, column)` | `String` | `format!` of revision, precision, line, column, run state, last run ms, engine name | app.rs:469-483 |
| `AppState::variables_text()` | `String` | `VARIABLES`/`FUNCTIONS` sections from `Document.vars`/`.funcs` | app.rs:486-515 |
| `AppState::logs_text()` | `String` | Iterates `AppState.logs`, applies `LogLevel::tag` | app.rs:518-533 |
| `LayoutSizes.editor_w` | `i32` | `compute_layout` | app.rs:31,56-86 |
| `LayoutSizes.editor_h` | `i32` | `compute_layout` | app.rs:33,56-86 |
| `LayoutSizes.right_w` | `i32` | `compute_layout` | app.rs:35,56-86 |
| `LayoutSizes.plot_h` | `i32` | `compute_layout` | app.rs:37,56-86 |
| `LayoutSizes.logs_h` | `i32` | `compute_layout` | app.rs:39,56-86 |
| `LogEntry.level` | `LogLevel` | Set by the logging call | app.rs:121 |
| `LogEntry.text` | `String` | Set by the logging call | app.rs:123 |
| `LogLevel::tag()` | `&'static str` | `info`/`warn`/`error`/`""`/`""` | app.rs:106-114 |
| `RunState::label()` | `&'static str` | `idle`/`running` | app.rs:137-142 |
| `RunRequest.document` | `usize` | Captured by `request_run` | app.rs:149,804 |
| `RunRequest.source` | `String` | Clone of the buffer at request time | app.rs:151,803 |
| `RunRequest.precision` | `usize` | Copy of the document precision at request time | app.rs:153,804 |
| `Effect::RunRequested` | `RunRequest` | Produced by `request_run` | app.rs:160 |
| `Effect::Saved` | `PathBuf` | Produced by `write_active` on success | app.rs:162 |
| `Effect::Error` | `String` | Produced by failing commands and failed runs | app.rs:164 |
| `Effect::Repaint` | unit | Produced by almost every handler | app.rs:166 |
| `RunOutput.result` | `Result<String, String>` | Set by `Evaluator::eval` / `eval_full` | engine.rs:51 |
| `RunOutput.output` | `Vec<String>` | Set by `take_output` in `eval_full` | engine.rs:53 |
| `RunOutput.variables` | `Vec<(String, String)>` | Set by `variables()` in `eval_full` | engine.rs:55 |
| `RunOutput.functions` | `Vec<(String, Vec<String>)>` | Set by `functions()` in `eval_full` | engine.rs:57 |
| `RunOutput.plot` | `Option<PlotCapture>` | Set by `last_plot()` in `eval_full` | engine.rs:59 |
| `PlotCapture.path` | `String` | Fake engine format string; real engine mapping | engine.rs:40; engine/fake.rs:129; engine/real.rs:74-78 |
| `PlotCapture.title` | `String` | Fake `plot` argument or real engine mapping | engine.rs:42 |
| `PlotCapture.svg` | `String` | `svg::make_plot_svg` in fake; real engine mapping | engine.rs:44 |
| `Settings.schema_version` | `u32` | `Settings::default()`; parsed version is not stored | settings.rs:76,169-181 |
| `Settings.window` | `WindowGeometry` | `from_json` clamps loaded values | settings.rs:78 |
| `Settings.layout` | `LayoutFractions` | `from_json` clamps loaded values | settings.rs:80 |
| `Settings.recent_files` | `Vec<String>` | `add_recent` and `from_json` | settings.rs:82 |
| `Settings.active_precision` | `usize` | `from_json` clamp, `with_factory_and_settings` clamp, `SetPrecision`, `sync_active_precision` | settings.rs:84 |
| `Settings.warnings` | `Vec<String>` | Pushed by `from_json`/`load`; drained by `take_warnings` | settings.rs:86 |
| `WindowGeometry.width` | `i32` | Default 1280; loaded from `window.width` | settings.rs:33,198-205 |
| `WindowGeometry.height` | `i32` | Default 800; loaded from `window.height` | settings.rs:35,206-213 |
| `LayoutFractions.editor_width_permille` | `u32` | Default 720; loaded in `300..=900` | settings.rs:55,220-227 |
| `LayoutFractions.logs_height_permille` | `u32` | Default 220; loaded in `100..=600` | settings.rs:57,228-235 |
| `LayoutFractions.plot_height_permille` | `u32` | Default 460; loaded in `150..=850` | settings.rs:59,236-243 |

## 4. What could NOT be determined within SCOPE, and why

1. **Which thread the shipped Win32 application evaluates a run on.** The core exposes `run_blocking` (evaluates inline on the caller's thread, app.rs:592-610) and produces `Effect::RunRequested` for a consumer that may run it `on a worker, or inline` (app.rs:159). The actual consumer is `src/win32/shell.rs`, which is out of scope, so the shipped threading choice, scheduling, and any worker lifetime cannot be established from these files.
2. **Whether evaluation is interruptible.** `Evaluator` provides only `eval(&mut self, src: &str)` (engine.rs:69) and `cancel_run` makes no engine call (app.rs:816-825); no cancel token or interrupt handle crosses the trait. Whether the engine crates implement interruption internally cannot be determined because `lovelace-lang` is out of scope.
3. **Autosave.** No autosave, timer, draft, or periodic-save code exists in `app.rs` or any other in-scope file. Because the Win32 shell is out of scope, a shell-level timer or autosave cannot be ruled out; only the core's absence is established.
4. **When and how settings are loaded and saved in the application.** `Settings::load` and `Settings::save` are implemented (settings.rs:280-305) and `default_config_path` is defined (settings.rs:362-376), but no in-scope file calls them; `AppState` only holds a `Settings` value. The call sites, timing, path selection, and failure handling in the running application are in `src/win32/`, out of scope.
5. **Real-engine formatting and precision semantics.** `RealEngine` delegates `format`, `precision`, `variables`, `functions`, `take_output`, `last_plot`, and `revision` to `lovelace_lang::Engine` (engine/real.rs:42-89). What precision does to the displayed text, how variables/functions are ordered, and what `error.message` contains are determined inside `lovelace-lang`, which is out of scope.
6. **Whether `Settings.window` is applied.** The geometry is parsed, clamped, stored, and serialised, but no in-scope code applies it to a window; window creation and `SetWindowPos`-style use are in `src/win32/`, out of scope.
7. **File dialogs and path acquisition.** The core receives `PathBuf` values in `Command::Open` and `Command::SaveAs`; it never opens a dialog or derives a default path. The UI's dialog and filename policy are in `src/win32/`, out of scope.
8. **Thread-safety of moving an engine between threads.** `Evaluator` has no `Send`/`Sync` bound and `Document` stores `Box<dyn Evaluator>` (engine.rs:67; app.rs:223). Whether the shell can or does move engine state across threads also depends on the concrete `lovelace_lang::Engine` type, which is out of scope.
9. **Behaviour of the engine behind `Evaluator::eval_full` on failure.** The default composition is readable and snapshots state after `eval` returns (engine.rs:72-80), and the fake engine's post-error state is readable (statements before the error keep their effects, revision increments once). How the real `lovelace_lang::Engine` mutates its own state when `eval` returns an error is out of scope.
10. **Any command/state-transition callers outside the core.** The census describes the public surface and bodies; it cannot establish which subset the shell exposes through menus/accelerators, nor whether any UI uses `Document::engine_mut`, `QuickEval`, or direct `Settings` mutation. Those call sites are out of scope.

## Appendix — source snapshot

SHA-256 of the exact files read:

| File | SHA-256 |
|---|---|
| `src/app.rs` | `DFADCA47527A79BD8023585BFEE29824D9B2841670101BD62B24E09AD603A1F8` |
| `src/engine.rs` | `DD69FC1BAA49FAB37289336FF46A7DADAC77C504DE8D79F15040BAF4557923F9` |
| `src/engine/fake.rs` | `A726F08174DDB10DC66C0011AE319606D1D455CEC4A5CA9F47FDD027FE6FF50A` |
| `src/engine/real.rs` | `4D147E6A51DBACA5FBA7D796DEA34FC2C7C4C4FCC5CE5673B2028C2BDA0465C4` |
| `src/settings.rs` | `455332405A9BB08792E68CBA9065D3394548AB5CF02BA9C4D04CBF4BCEC80006` |
| `src/main.rs` | `184D47E4760703454D9DF545A1A25F00B65029E8C93529E1D3A753B9A7B79D02` |
| `Cargo.toml` | `BE40CEED5BD63EA4179BDD595A6BA4B71F27DD820D767FB90D353706475F99A5` |
