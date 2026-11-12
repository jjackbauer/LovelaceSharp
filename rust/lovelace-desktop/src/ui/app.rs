//! The native workbench window: toolbar, tab strip, four panes and the status strip.
//!
//! The window is a thin view over [`crate::workbench::Workbench`]. It never evaluates a script:
//! Run only submits to the `ExecutionHost`, every frame polls the typed event stream, and every
//! value in the panes comes from the snapshot or the engine's plot capture. Panes with nothing
//! to show render their empty state instead of sample data.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::{
    Align, Align2, Color32, CursorIcon, FontId, Id, Key, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

use crate::engine::FunctionOrigin;
use crate::execution::{Diagnostic, DocumentId};
use crate::inspector::{Inspection, InspectorExpansion, Section, TREE_ROOT_PATH, tree_path};
use crate::keyboard::{self, FileCommand};
use crate::log_kind::LogKind;
use crate::quick_eval::QuickEval;
use crate::recovery::{self, RecoveryJournal, RecoveryRecord, RecoveryScan};
use crate::results_tree::{
    ArrayNode, ExpansionState, ResultProjection, StructuredNode, element_path, field_path,
};
use crate::settings::{self, MAX_PRECISION, MIN_PRECISION, Settings};
use crate::workbench::{DocumentView, SaveOutcome, Workbench};
use crate::workspace_table::{
    FunctionColumn, FunctionTable, VariableColumn, VariableTable, cell_text, ordered_functions,
    ordered_variables,
};

use super::editor::{TextEditor, surface_id};
use super::file_dialogs::{self, FileDialogs, NativeFileDialogs};
use super::plot::{self, Plot, PngBackground};
/// Pane background.
pub const PANE_BG: Color32 = Color32::from_rgb(0x17, 0x1b, 0x21);
/// Pane header background.
pub const HEADER_BG: Color32 = Color32::from_rgb(0x22, 0x28, 0x30);
/// Pane border.
pub const BORDER: Color32 = Color32::from_rgb(0x33, 0x3a, 0x44);
/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(0xd8, 0xde, 0xe9);
/// Secondary text.
pub const DIM: Color32 = Color32::from_rgb(0x8b, 0x98, 0xa9);
/// Accent (names, caret, status).
pub const ACCENT: Color32 = Color32::from_rgb(0x4e, 0xc9, 0xb0);
/// Splitter background.
pub const SPLIT_BG: Color32 = Color32::from_rgb(0x0d, 0x10, 0x14);
/// Splitter hover colour.
pub const SPLIT_HOT: Color32 = Color32::from_rgb(0x2b, 0x4a, 0x47);
/// Warning text.
pub const WARN: Color32 = Color32::from_rgb(0xd7, 0xba, 0x7d);
/// Error text.
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x7b, 0x72);
/// Echo marker text (the quiet, low-contrast lavender of a prompt echo).
pub const LOG_ECHO: Color32 = Color32::from_rgb(0xc8, 0xb6, 0xff);
/// Progress marker text (a calm blue, distinct from the accent teal).
pub const LOG_PROGRESS: Color32 = Color32::from_rgb(0x7f, 0xb0, 0xd8);

/// Width of a splitter, in pixels.
pub const SPLIT: f32 = 6.0;
const MIN_EDITOR: f32 = 320.0;
const MIN_PLOT: f32 = 240.0;
const MIN_WORKSPACE: f32 = 180.0;
const MIN_LOGS: f32 = 100.0;
const MIN_TOP: f32 = 240.0;

/// Rectangles of every pane and splitter for one pass.
#[derive(Clone, Copy, Debug)]
pub struct PaneLayout {
    /// Editor pane.
    pub editor: Rect,
    /// Plot pane.
    pub plot: Rect,
    /// Workspace pane.
    pub workspace: Rect,
    /// Logs pane.
    pub logs: Rect,
    /// Editor / plot splitter.
    pub split_a: Rect,
    /// Plot / workspace splitter.
    pub split_b: Rect,
    /// Top / logs splitter.
    pub split_c: Rect,
}

/// Computes the four-pane layout from the saved fractions, honouring minimum sizes whenever the
/// window is large enough and compressing the minimums proportionally when it is not.
pub fn compute_layout(
    full: Rect,
    editor_frac: f32,
    workspace_frac: f32,
    logs_frac: f32,
) -> PaneLayout {
    let usable_w = (full.width() - 2.0 * SPLIT).max(0.0);
    let min_sum = MIN_EDITOR + MIN_PLOT + MIN_WORKSPACE;
    let shrink_w = if usable_w >= min_sum {
        1.0
    } else {
        usable_w / min_sum
    };
    let min_editor = MIN_EDITOR * shrink_w;
    let min_plot = MIN_PLOT * shrink_w;
    let min_workspace = MIN_WORKSPACE * shrink_w;

    let max_editor = (usable_w - min_plot - min_workspace).max(min_editor);
    let editor_w = (usable_w * editor_frac).clamp(min_editor, max_editor);
    let remaining = (usable_w - editor_w).max(0.0);
    let max_workspace = (remaining - min_plot).max(min_workspace);
    let workspace_w = (usable_w * workspace_frac).clamp(min_workspace, max_workspace);
    let plot_w = (remaining - workspace_w).max(0.0);

    let usable_h = (full.height() - SPLIT).max(0.0);
    let height_min_sum = MIN_TOP + MIN_LOGS;
    let shrink_h = if usable_h >= height_min_sum {
        1.0
    } else {
        usable_h / height_min_sum
    };
    let min_top = MIN_TOP * shrink_h;
    let min_logs = MIN_LOGS * shrink_h;
    let max_logs = (usable_h - min_top).max(min_logs);
    let logs_h = (full.height() * logs_frac).clamp(min_logs, max_logs);
    let top_h = (usable_h - logs_h).max(0.0);

    let top_rect = Rect::from_min_size(full.min, vec2(full.width(), top_h));
    let editor = Rect::from_min_size(top_rect.min, vec2(editor_w, top_h));
    let split_a = Rect::from_min_size(pos2(editor.right(), top_rect.top()), vec2(SPLIT, top_h));
    let plot = Rect::from_min_size(pos2(split_a.right(), top_rect.top()), vec2(plot_w, top_h));
    let split_b = Rect::from_min_size(pos2(plot.right(), top_rect.top()), vec2(SPLIT, top_h));
    let workspace = Rect::from_min_size(
        pos2(split_b.right(), top_rect.top()),
        vec2(workspace_w, top_h),
    );
    let split_c = Rect::from_min_size(
        pos2(full.left(), top_rect.bottom()),
        vec2(full.width(), SPLIT),
    );
    let logs = Rect::from_min_size(
        pos2(full.left(), split_c.bottom()),
        vec2(full.width(), logs_h),
    );

    PaneLayout {
        editor,
        plot,
        workspace,
        logs,
        split_a,
        split_b,
        split_c,
    }
}

/// The in-window PNG export options (PLT-006): a chosen scale and background.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PngExport {
    /// Pixels per SVG point.
    scale: f32,
    /// Opaque white or transparent.
    background: PngBackground,
}

impl Default for PngExport {
    fn default() -> PngExport {
        PngExport {
            scale: 2.0,
            background: PngBackground::Opaque,
        }
    }
}

/// What the user is answering when the unsaved-changes prompt is open (DOC-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsavedAnswer {
    /// Write the document, then continue closing.
    Save,
    /// Close without writing; the edits are lost.
    Discard,
    /// Leave everything exactly as it was: the tab stays open, the buffer and the caret unchanged.
    Cancel,
}

impl UnsavedAnswer {
    /// The button label, which is also the prompt's accessible name in a UIA capture.
    fn label(self) -> &'static str {
        match self {
            UnsavedAnswer::Save => "Save",
            UnsavedAnswer::Discard => "Discard",
            UnsavedAnswer::Cancel => "Cancel",
        }
    }
}

/// What closing is being decided for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseTarget {
    /// One tab, identified by its stable document identity.
    Tab(DocumentId),
    /// The whole window (the title-bar close button, Quit and the OS close request).
    Window,
}

/// The pending Save/Discard/Cancel question (DOC-003).
///
/// Documents are identified by [`DocumentId`], not by tab index, so a tab change during the flow
/// cannot make the prompt name the wrong document.
#[derive(Clone, Debug)]
struct ClosePrompt {
    /// What is being closed once every question is answered.
    target: CloseTarget,
    /// Documents with unsaved changes still to ask about, in tab order.
    pending: Vec<DocumentId>,
    /// How many documents the flow started with, so the prompt can say "1 of 2".
    total: usize,
    /// The widget that had focus when the prompt opened; restored on Cancel.
    focus_before: Option<Id>,
    /// Why the last Save in the prompt failed, shown in the prompt itself.
    error: Option<String>,
}

impl ClosePrompt {
    /// The document this prompt is asking about.
    fn current(&self) -> DocumentId {
        self.pending[0]
    }
}

/// Where the window persists its state (DOC-004) and journals unsaved buffers (DOC-005).
///
/// The shipped window resolves both paths from the application directory
/// (`%APPDATA%\Lovelace` on Windows); the window tests supply scratch paths through
/// [`WorkbenchApp::with_storage`], so no test touches the user profile and no product code writes
/// beside the executable or in the process working directory (PLF-006).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AppStorage {
    /// The settings file, or `None` when this window must not persist anything.
    settings_path: Option<PathBuf>,
    /// The recovery directory, or `None` when this window must not journal anything.
    recovery_root: Option<PathBuf>,
}

impl AppStorage {
    /// The paths the shipped window uses.
    fn from_application_dir() -> AppStorage {
        AppStorage {
            settings_path: settings::default_config_path(),
            recovery_root: settings::default_recovery_dir(),
        }
    }
}

/// What pressing a button in the DOC-005 recovery prompt means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecoveryAnswer {
    /// Put the record's text back into a tab; the file on disk is not touched.
    Recover,
    /// Delete the record without restoring it.
    Discard,
}

impl RecoveryAnswer {
    /// The button label, which is also the button's accessible name in a UIA capture.
    fn label(self) -> &'static str {
        match self {
            RecoveryAnswer::Recover => "Recover",
            RecoveryAnswer::Discard => "Discard",
        }
    }
}

/// The recovery prompt shown at launch when journaled buffers survived (DOC-005).
///
/// It is answered record by record. While it is open no keystroke reaches an editor and no pane
/// accepts a pointer, so the journaled text is offered before any save can overwrite a file.
#[derive(Clone, Debug)]
struct RecoveryPrompt {
    /// The records still to answer, oldest document identity first.
    records: Vec<RecoveryRecord>,
    /// Why the last answer failed, shown in the prompt itself.
    error: Option<String>,
    /// The widget that had focus when the prompt opened; restored when it is dismissed.
    focus_before: Option<Id>,
}

/// The window application.
pub struct WorkbenchApp {
    workbench: Workbench,
    /// DOC-004: the settings value this run loaded, updated as the session changes.
    settings: Settings,
    /// DOC-004: where the settings file and the recovery directory live.
    storage: AppStorage,
    /// DOC-005: the debounced writer that keeps one recovery record per dirty document.
    recovery: RecoveryJournal,
    /// DOC-005: the launch recovery prompt, while one is open.
    recovery_prompt: Option<RecoveryPrompt>,
    /// DOC-005 test seam: the recovery prompt buttons painted in the last pass.
    recovery_buttons: Vec<(usize, RecoveryAnswer, Rect)>,
    /// DOC-004: the session signature last written, so an unchanged session is not rewritten.
    persisted_session: Option<String>,
    /// The last settings write failure, so it is logged once instead of every frame.
    settings_error: Option<String>,
    /// The one assembled state this frame renders (RUN-003).
    ///
    /// Rebuilt in [`WorkbenchApp::sync`] immediately after `poll()`, so every pane draws the
    /// same committed snapshot, log, revision and diagnostic even though it is a separate egui
    /// closure. No pane calls back into the workbench for pane data.
    view: DocumentView,
    editors: Vec<TextEditor>,
    /// The identity of the document each editor holds, parallel to `editors`.
    ///
    /// DOC-002: New, Open and Close change the tab list, so the editors are reconciled by document
    /// identity instead of by position. A surviving tab keeps its editor (caret, selection,
    /// history); a new tab gets a fresh one.
    editor_ids: Vec<DocumentId>,
    loaded_revisions: Vec<u64>,
    /// DOC-002: the native dialogs. A headless test replaces this with a scripted answer.
    dialogs: Box<dyn FileDialogs>,
    /// DOC-003: the open Save/Discard/Cancel question, if any.
    close: Option<ClosePrompt>,
    /// True once the close flow approved closing the window, so the OS request that follows is
    /// allowed instead of starting the flow again.
    window_close_approved: bool,
    /// The title last sent to the native window, so it is only sent when it changes.
    window_title: String,
    /// A widget to focus on the next pass (DOC-003's focus restoration).
    ///
    /// A focus request made in the pass that dismisses the modal cannot stick: the modal's layer
    /// is still the top layer while the panes paint, so the editor is not interactable in that
    /// pass. Queueing the id for the next pass is what makes Cancel leave the focus where it was.
    pending_focus: Option<Id>,
    /// DOC-003 test seam: the prompt buttons painted in the last pass and their rectangles.
    prompt_buttons: Vec<(UnsavedAnswer, Rect)>,
    /// DOC-002 test seam: the File bar buttons painted in the last pass and their rectangles.
    file_buttons: Vec<(&'static str, Rect)>,
    plot: Plot,
    loaded_plot_svg: Option<String>,
    /// PLT-006: the open PNG export options window, if any.
    plot_export: Option<PngExport>,
    /// PLT-006: the last export's short result and whether it is an error, shown in the plot
    /// header so the action has feedback even when the file dialog closes without a trace.
    plot_export_status: Option<(String, bool)>,
    editor_frac: f32,
    workspace_frac: f32,
    logs_frac: f32,
    filter: String,
    /// The two Workspace tables, rebuilt from the frame's committed snapshot in [`Self::sync`].
    variables_table: VariableTable,
    functions_table: FunctionTable,
    /// RES-004: the RESULT tree's per-path collapse state. It is bound to the committed result
    /// in `paint_result_section`, so repaints of the same value keep every decision and a new
    /// result clears the map.
    expansion: ExpansionState,
    /// SYM-001: true while the Workspace pane's Inspector view is selected. The view is a tab
    /// beside the Result view; it renders only the committed snapshot's inspection payload.
    inspector_open: bool,
    /// SYM-001: the expression tree's per-path collapse state, scoped to the inspection payload.
    inspector_expansion: InspectorExpansion,
    /// SYM-001: expression-tree headers painted in the last pass (path and rectangle), the same
    /// test seam as `result_toggle_rects`.
    inspector_toggle_rects: Vec<(String, Rect)>,
    /// SYM-001: the Result and Inspector tab rectangles painted in the last pass. The pointer
    /// tests click these centres; UIA reaches the same buttons by their names.
    workspace_view_tabs: [Option<Rect>; 2],
    /// RESULT tree headers painted in the last pass: their node path and rectangle. The pointer
    /// tests click these centres, the same seam as `variable_header_rects`.
    result_toggle_rects: Vec<(String, Rect)>,
    /// Header rectangles from the last painted pass; the pointer tests click these centres and
    /// the pane uses them to decide which column a click belongs to.
    variable_header_rects: [Option<Rect>; 3],
    function_header_rects: [Option<Rect>; 3],
    /// The one-line quick-eval input's text and submitted-expression history.
    quick_eval: QuickEval,
    /// The quick-eval field's last painted rectangle, used by the pointer-focus test.
    quick_eval_rect: Option<Rect>,
    layout: Option<PaneLayout>,
    /// True once the window has moved keyboard focus into the editor on its first frame.
    focus_requested: bool,
    /// True between handling Ctrl+Enter and the end of that frame (RUN-001 trace only).
    trace_pending: bool,
    /// True until the frame that follows the painted Running frame starts (trace only).
    trace_after_paint: bool,
}

impl WorkbenchApp {
    /// Creates the window over a fresh workbench, optionally opening a script path in tab one.
    ///
    /// DOC-004/005: the launch reads the settings file, restores the last session's tabs (by
    /// reading their files again, never by executing them) and offers any recovery records the
    /// previous run left behind.
    pub fn new(initial_path: Option<PathBuf>) -> WorkbenchApp {
        WorkbenchApp::launch(AppStorage::from_application_dir(), initial_path)
    }

    /// Launches over an explicit storage location (the test seam for DOC-004/005).
    fn launch(storage: AppStorage, initial_path: Option<PathBuf>) -> WorkbenchApp {
        let mut settings = match &storage.settings_path {
            Some(path) => Settings::load(path),
            None => Settings::new(),
        };
        let settings_warnings = settings.take_warnings();
        let mut workbench = Workbench::new();
        // DOC-004: a recent path whose file was deleted between two launches is pruned before the
        // File bar can offer it.
        let before_recent = settings.recent_files.len();
        let pruned = settings.prune_missing_recent();
        workbench.set_recent_files(settings.recent_files.clone());
        settings.recent_files = workbench.recent_files().to_vec();

        // DOC-005: whatever an interrupted run journaled is read before anything else, so the
        // prompt can offer it and no save can slip in first.
        let scan = match &storage.recovery_root {
            Some(root) => recovery::scan(root),
            None => RecoveryScan::default(),
        };
        let mut consumed: Vec<u64> = Vec::new();
        if let Some(session) = settings.session.clone() {
            let report = workbench.restore_session(&session, &scan.records);
            consumed = report.recovered;
        }
        for warning in settings_warnings {
            workbench.log_active(LogKind::Diagnostic, format!("Settings: {warning}"));
        }
        if pruned > 0 {
            workbench.log_active(
                LogKind::Info,
                format!("Pruned {pruned} recent file(s) that no longer exist."),
            );
        }
        let _ = before_recent;
        for warning in &scan.warnings {
            workbench.log_active(LogKind::Diagnostic, format!("Recovery: {warning}"));
        }
        // A path named on the command line wins over the restored session: it is selected when the
        // session already had it, otherwise it opens in its own tab.
        if let Some(path) = &initial_path {
            match workbench
                .documents()
                .iter()
                .position(|document| document.path.as_deref() == Some(path.as_path()))
            {
                Some(index) => workbench.set_active(index),
                None => {
                    if let Err(message) = workbench.open(path) {
                        workbench.log_active(LogKind::Diagnostic, message);
                    }
                }
            }
        }

        let pending: Vec<RecoveryRecord> = scan
            .records
            .iter()
            .filter(|record| !consumed.contains(&record.document_id))
            .cloned()
            .collect();

        let mut app = WorkbenchApp::with_storage(workbench, storage);
        app.settings = settings;
        // A record the session restore consumed is exactly the text now in its tab, so the journal
        // keeps tracking it instead of rewriting it on the first frame; it is deleted when the
        // document is saved, closed or the window exits cleanly.
        for document_id in consumed {
            if let Some(index) = app.workbench.document_index(DocumentId(document_id)) {
                let revision = app.workbench.documents()[index].revision;
                app.recovery.track(document_id, revision, Instant::now());
            }
        }
        if !pending.is_empty() {
            app.recovery_prompt = Some(RecoveryPrompt {
                records: pending,
                error: None,
                focus_before: None,
            });
        }
        // The first frame writes the settings file, so the session is current from the moment the
        // window appears: a crash later in the run restores the tabs that were actually open.
        app.persisted_session = None;
        app
    }

    /// Creates the window over an existing workbench (the test seam).
    #[cfg_attr(not(test), allow(dead_code))]
    fn with_workbench(workbench: Workbench) -> WorkbenchApp {
        WorkbenchApp::with_storage(workbench, AppStorage::default())
    }

    /// Creates the window over an existing workbench and an explicit storage location.
    fn with_storage(workbench: Workbench, storage: AppStorage) -> WorkbenchApp {
        let editors = workbench
            .documents()
            .iter()
            .map(|document| {
                // DOC-005: a restored buffer comes back with the caret and selection the journal
                // recorded, so recovery does not merely restore the text.
                let mut editor =
                    TextEditor::from_source_with_newline(&document.source, document.newline);
                editor.set_caret(document.caret.0, document.caret.1);
                editor.selection = document.selection.map(|(anchor, _end)| anchor);
                editor
            })
            .collect();
        let editor_ids = workbench.documents().iter().map(|doc| doc.id).collect();
        let loaded_revisions = workbench
            .documents()
            .iter()
            .map(|document| document.revision)
            .collect();
        let view = workbench.active_view();
        let variables_table = VariableTable::new(&view.state);
        let functions_table = FunctionTable::new(&view.state);
        let recovery = match &storage.recovery_root {
            Some(root) => RecoveryJournal::new(root.clone()),
            None => RecoveryJournal::disabled(),
        };
        WorkbenchApp {
            workbench,
            settings: Settings::new(),
            storage,
            recovery,
            recovery_prompt: None,
            recovery_buttons: Vec::new(),
            persisted_session: None,
            settings_error: None,
            view,
            editors,
            editor_ids,
            loaded_revisions,
            dialogs: Box::new(NativeFileDialogs),
            close: None,
            window_close_approved: false,
            window_title: String::new(),
            pending_focus: None,
            prompt_buttons: Vec::new(),
            file_buttons: Vec::new(),
            plot: Plot::new(),
            loaded_plot_svg: None,
            plot_export: None,
            plot_export_status: None,
            editor_frac: 0.40,
            workspace_frac: 0.20,
            logs_frac: 0.18,
            filter: String::new(),
            variables_table,
            functions_table,
            expansion: ExpansionState::new(),
            inspector_open: false,
            inspector_expansion: InspectorExpansion::new(),
            inspector_toggle_rects: Vec::new(),
            workspace_view_tabs: [None; 2],
            result_toggle_rects: Vec::new(),
            variable_header_rects: [None; 3],
            function_header_rects: [None; 3],
            quick_eval: QuickEval::new(),
            quick_eval_rect: None,
            layout: None,
            focus_requested: false,
            trace_pending: false,
            trace_after_paint: false,
        }
    }

    /// Applies queued events and re-loads the editors whose document changed elsewhere.
    fn sync(&mut self) {
        self.workbench.poll();
        // DOC-002: New, Open and Close change the tab list; keep one editor per open document.
        self.sync_editors();
        // DOC-005: the caret and selection belong to the editor, so they are copied into the
        // document before the journal pass that writes them into the recovery record.
        for (index, editor) in self.editors.iter().enumerate() {
            let selection = editor
                .selection
                .map(|anchor| (anchor, (editor.caret_row, editor.caret_col)));
            self.workbench
                .set_caret(index, (editor.caret_row, editor.caret_col), selection);
        }
        // DOC-005: journal every dirty document, at most once per debounce window. A write failure
        // is a diagnostic line, never a crash and never a lost buffer. Once the window close has
        // been approved nothing may journal or persist again: the clean exit has already recorded
        // the final state on disk.
        if !self.window_close_approved {
            let report = self.recovery.sync(self.workbench.documents(), Instant::now());
            for failure in report.failed {
                self.workbench
                    .log_active(LogKind::Diagnostic, format!("Recovery journal: {failure}"));
            }
            // DOC-004: a changed tab list, active tab, recent list or precision is persisted at
            // once, so a crash leaves a current session rather than the one from the last exit.
            self.persist_state(false);
        }
        // RUN-003: assemble the frame's pane state once, from the same committed snapshot the
        // poll above applied. Every pane below renders this value, not a fresh workbench lookup.
        self.view = self.workbench.active_view();
        // WKS-001/WKS-002: the tables are rebuilt from the same committed capture the frame
        // renders, keeping the user's chosen sort across runs and tab switches.
        self.variables_table.rebuild(&self.view.state);
        self.functions_table.rebuild(&self.view.state);
        // The workbench gates a diagnostic on the revision it was produced for; copying it here
        // makes every editor decoration appear with the failure and disappear on the next edit.
        for (index, editor) in self.editors.iter_mut().enumerate() {
            editor.diagnostic = self.workbench.diagnostic(index).cloned();
        }
        let current_svg = self.view.plot.as_ref().map(|plot| plot.svg.clone());
        if current_svg != self.loaded_plot_svg {
            match current_svg {
                Some(svg) => {
                    self.plot.set_svg(&svg);
                    self.loaded_plot_svg = Some(svg);
                }
                None => {
                    self.plot.clear();
                    self.loaded_plot_svg = None;
                }
            }
        }
    }

    /// Keeps `editors`, `loaded_revisions` and `editor_ids` in step with the workbench's tabs.
    ///
    /// A tab that is still open keeps its editor object, so its caret, selection and undo history
    /// survive another tab opening or closing. A document whose revision advanced elsewhere (a
    /// file load, a reload) gets its text and newline style re-loaded; the fresh tab of a document
    /// created since the last frame gets a new editor at the start of its text.
    fn sync_editors(&mut self) {
        let ids: Vec<DocumentId> = self
            .workbench
            .documents()
            .iter()
            .map(|document| document.id)
            .collect();
        if self.editor_ids != ids {
            let mut previous: Vec<(DocumentId, TextEditor, u64)> = self
                .editor_ids
                .iter()
                .copied()
                .zip(self.editors.drain(..))
                .zip(self.loaded_revisions.drain(..))
                .map(|((id, editor), revision)| (id, editor, revision))
                .collect();
            let mut editors = Vec::with_capacity(ids.len());
            let mut revisions = Vec::with_capacity(ids.len());
            for (index, id) in ids.iter().enumerate() {
                let document = &self.workbench.documents()[index];
                let (caret, selection) = (document.caret, document.selection);
                match previous.iter().position(|(existing, _, _)| existing == id) {
                    Some(position) => {
                        let (_, mut editor, loaded) = previous.remove(position);
                        if loaded != document.revision {
                            editor.load_with_newline(&document.source, document.newline);
                        }
                        // DOC-005: a loaded or recovered buffer puts the journaled caret back.
                        editor.set_caret(caret.0, caret.1);
                        editor.selection = selection.map(|(anchor, _end)| anchor);
                        editors.push(editor);
                        revisions.push(document.revision);
                    }
                    None => {
                        let mut editor = TextEditor::from_source_with_newline(
                            &document.source,
                            document.newline,
                        );
                        editor.set_caret(caret.0, caret.1);
                        editor.selection = selection.map(|(anchor, _end)| anchor);
                        editors.push(editor);
                        revisions.push(document.revision);
                    }
                }
            }
            self.editors = editors;
            self.loaded_revisions = revisions;
            self.editor_ids = ids;
            return;
        }
        let active = self.workbench.active();
        let revision = self.workbench.documents()[active].revision;
        if self.loaded_revisions[active] != revision {
            let document = &self.workbench.documents()[active];
            let (source, newline) = (document.source.clone(), document.newline);
            let (caret, selection) = (document.caret, document.selection);
            self.editors[active].load_with_newline(&source, newline);
            self.editors[active].set_caret(caret.0, caret.1);
            self.editors[active].selection = selection.map(|(anchor, _end)| anchor);
            self.loaded_revisions[active] = revision;
        }
    }

    // -----------------------------------------------------------------------------------------
    // DOC-004 / DOC-005: the persisted session, the recovery journal and the recovery prompt.
    // -----------------------------------------------------------------------------------------

    /// A string that changes exactly when something the settings file stores changes.
    fn session_signature(&self) -> String {
        let session = self.workbench.session_state();
        let recent = self.workbench.recent_files();
        let precision = self.workbench.active_document().precision;
        format!("{session:?}|{recent:?}|{precision}")
    }

    /// Writes the session, the recent list and the active precision when they changed.
    ///
    /// `force` is used at a clean exit. A failure is logged once and never blocks the window: a
    /// read-only or missing application directory must not make the editor unusable.
    fn persist_state(&mut self, force: bool) {
        let Some(path) = self.storage.settings_path.clone() else {
            return;
        };
        let signature = self.session_signature();
        if !force && self.persisted_session.as_deref() == Some(signature.as_str()) {
            return;
        }
        self.settings.set_session(self.workbench.session_state());
        self.settings.recent_files = self.workbench.recent_files().to_vec();
        self.settings.active_precision = self.workbench.active_document().precision;
        match self.settings.save(&path) {
            Ok(()) => {
                self.persisted_session = Some(signature);
                self.settings_error = None;
            }
            Err(error) => {
                let message = format!("Cannot save settings to {}: {error}", path.display());
                if self.settings_error.as_deref() != Some(message.as_str()) {
                    self.workbench
                        .log_active(LogKind::Diagnostic, message.clone());
                    self.settings_error = Some(message);
                }
            }
        }
    }

    /// DOC-005: deletes the records of every open document at a clean exit.
    ///
    /// Called only once the window is actually closing, after every dirty document has been saved
    /// or discarded, so nothing recoverable is left behind by a normal quit.
    fn finish_clean_exit(&mut self) {
        self.persist_state(true);
        let report = self.recovery.clean_exit(self.workbench.documents());
        for failure in report.failed {
            self.workbench
                .log_active(LogKind::Diagnostic, format!("Recovery journal: {failure}"));
        }
    }

    /// DOC-005: puts one record's text back into a tab.
    ///
    /// A record whose document is already open (the session restore usually recreated the tab from
    /// disk) rewrites that tab's buffer; otherwise the text arrives in a new tab. Either way the
    /// document is dirty against what is on disk, nothing is executed and nothing is written.
    fn recover_record(&mut self, record: RecoveryRecord) {
        let report = self.workbench.recover(&record);
        if report.document_id.0 == record.document_id {
            // The record file is now exactly the text in the tab, so the journal tracks it instead
            // of writing it again on this frame.
            let revision = self.workbench.documents()[report.index].revision;
            self.recovery
                .track(record.document_id, revision, Instant::now());
        } else if let Some(message) = self.recovery.forget(record.document_id) {
            // Recovered into a different tab identity: the old record is superseded. The new tab
            // is dirty and untracked, so the next journal pass writes its record immediately.
            self.workbench
                .log_active(LogKind::Diagnostic, format!("Recovery journal: {message}"));
        }
        self.remove_recovery_record(record.document_id);
        self.sync();
        self.pending_focus = Some(surface_id());
    }

    /// DOC-005: deletes one record without restoring it.
    fn discard_record(&mut self, record: RecoveryRecord) {
        if let Some(message) = self.recovery.forget(record.document_id) {
            self.workbench
                .log_active(LogKind::Diagnostic, format!("Recovery journal: {message}"));
        }
        self.remove_recovery_record(record.document_id);
        self.pending_focus = Some(surface_id());
    }

    /// Removes one answered record from the prompt, closing the prompt when none are left.
    fn remove_recovery_record(&mut self, document_id: u64) {
        let Some(prompt) = self.recovery_prompt.as_mut() else {
            return;
        };
        prompt.records.retain(|record| record.document_id != document_id);
        if prompt.records.is_empty() {
            if let Some(prompt) = self.recovery_prompt.take() {
                self.pending_focus = Some(prompt.focus_before.unwrap_or_else(surface_id));
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // DOC-002: the file lifecycle. New / Open / Save / Save As / Close all go through the
    // workbench, which owns the document state and the bytes; the window only asks the native
    // dialogs for a path and renders the result.
    // -----------------------------------------------------------------------------------------

    /// Ctrl+N / File > New: add an empty document tab and select it.
    fn new_document(&mut self) {
        self.workbench.new_document();
    }

    /// Ctrl+O / File > Open: ask the native Open dialog, then open the chosen file in a tab.
    fn open_dialog(&mut self) {
        let Some(path) = self.dialogs.open() else {
            return;
        };
        // A refusal (a non-UTF-8 file, for example) is logged by the workbench with its reason and
        // leaves every tab alone; the editor keeps the focus it already had.
        if let Ok(index) = self.workbench.open(&path) {
            self.sync_after_tab_change(index);
        }
    }

    /// Ctrl+S / File > Save, or Ctrl+Shift+S / File > Save As.
    ///
    /// `ask_path` forces the Save As dialog; Save asks only when the document has no path yet.
    /// A cancelled dialog changes nothing at all.
    fn save_active(&mut self, ask_path: bool) {
        let active = self.workbench.active();
        let _ = self.write_document_through_dialogs(active, ask_path);
    }

    /// Writes one document, asking the native Save As dialog when needed.
    ///
    /// Returns `None` when the user cancelled the dialog (nothing was written and the document is
    /// unchanged), otherwise the workbench's outcome.
    fn write_document_through_dialogs(
        &mut self,
        index: usize,
        ask_path: bool,
    ) -> Option<SaveOutcome> {
        let (has_path, suggested, initial_dir) = {
            let document = self.workbench.document(index)?;
            (
                document.path.is_some(),
                file_dialogs::suggested_name(&document.title),
                document
                    .path
                    .as_ref()
                    .and_then(|path| path.parent().map(PathBuf::from)),
            )
        };
        if !ask_path && has_path {
            return Some(self.workbench.save(index));
        }
        let path = self.dialogs.save_as(&suggested, initial_dir.as_deref())?;
        Some(self.workbench.save_as(index, path))
    }

    /// Makes `index` the active tab and reconciles the editors immediately, before any pane in
    /// this frame indexes them.
    fn sync_after_tab_change(&mut self, index: usize) {
        self.workbench.set_active(index);
        self.sync();
    }

    // -----------------------------------------------------------------------------------------
    // DOC-003: the unsaved-changes prompt.
    // -----------------------------------------------------------------------------------------

    /// Starts closing `target`, or closes it immediately when nothing in it is dirty.
    fn request_close(&mut self, target: CloseTarget, ctx: &egui::Context) {
        // The DOC-005 recovery prompt is answered before any close: a quit must not race the
        // question of whether journaled text should be recovered.
        if self.close.is_some() || self.recovery_prompt.is_some() {
            return;
        }
        let pending: Vec<DocumentId> = match target {
            CloseTarget::Tab(id) => match self.workbench.document_index(id) {
                Some(index) if self.workbench.documents()[index].dirty() => vec![id],
                Some(_) => Vec::new(),
                None => return,
            },
            CloseTarget::Window => self.workbench.dirty_documents(),
        };
        if pending.is_empty() {
            self.perform_close(target, ctx);
            return;
        }
        let total = pending.len();
        let focus_before = ctx.memory(|memory| memory.focused());
        self.close = Some(ClosePrompt {
            target,
            pending,
            total,
            focus_before,
            error: None,
        });
    }

    /// Carries out a close that has been decided: a tab is removed, the window asks the OS to
    /// close and the next close request is allowed through.
    fn perform_close(&mut self, target: CloseTarget, ctx: &egui::Context) {
        match target {
            CloseTarget::Tab(id) => {
                // DOC-005: a tab that is closed (saved or discarded) no longer has unsaved work,
                // so its recovery record must not outlive it.
                if let Some(message) = self.recovery.forget(id.0) {
                    self.workbench
                        .log_active(LogKind::Diagnostic, format!("Recovery journal: {message}"));
                }
                self.workbench.close_document(id);
                // The tab list changed under this frame's editors; reconcile now so no pane draws
                // a document that is gone.
                self.sync();
                self.pending_focus = Some(surface_id());
            }
            CloseTarget::Window => {
                // DOC-004/005: the last session is written and the journal's records of the open
                // documents are deleted; the quit is only approved once both have happened.
                self.finish_clean_exit();
                self.window_close_approved = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Applies one answer to the prompt.
    ///
    /// Cancel is deliberately the cheapest branch: it drops the prompt, restores the focus that
    /// was taken when it opened, and touches no editor, no buffer and no caret.
    fn answer_close(&mut self, answer: UnsavedAnswer, ctx: &egui::Context) {
        let Some(prompt) = self.close.clone() else {
            return;
        };
        match answer {
            UnsavedAnswer::Cancel => {
                self.close = None;
                // Restored on the next pass, when the modal's layer no longer blocks the editor.
                self.pending_focus = Some(prompt.focus_before.unwrap_or_else(surface_id));
            }
            UnsavedAnswer::Discard => self.advance_close(ctx),
            UnsavedAnswer::Save => {
                let Some(index) = self.workbench.document_index(prompt.current()) else {
                    self.advance_close(ctx);
                    return;
                };
                match self.write_document_through_dialogs(index, false) {
                    Some(SaveOutcome::Saved { .. }) => self.advance_close(ctx),
                    Some(SaveOutcome::Failed(message)) => {
                        // The write failed: keep asking, and say why in the prompt.
                        if let Some(prompt) = self.close.as_mut() {
                            prompt.error = Some(message);
                        }
                    }
                    Some(SaveOutcome::NeedsPath) => {
                        // Unreachable: the dialog supplies a path before this outcome can occur.
                        if let Some(prompt) = self.close.as_mut() {
                            prompt.error = Some("This document has no path yet.".to_string());
                        }
                    }
                    None => {
                        // The Save As dialog was cancelled. The prompt stays open so the user can
                        // still choose Discard or Cancel.
                    }
                }
            }
        }
    }

    /// Removes the answered document from the queue and closes when nothing is left to ask about.
    fn advance_close(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.close.as_mut() else {
            return;
        };
        prompt.pending.remove(0);
        prompt.error = None;
        if prompt.pending.is_empty() {
            let target = prompt.target;
            self.close = None;
            self.perform_close(target, ctx);
        }
    }

    /// Interprets the OS close request (title-bar X, Alt+F4).
    ///
    /// A dirty workbench refuses the close for this frame and asks instead; nothing is written and
    /// nothing is discarded until the prompt is answered. A clean workbench lets the close through.
    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.window_close_approved {
            return;
        }
        // DOC-005: the recovery question is answered first; the window stays open until it is.
        if self.recovery_prompt.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        if self.close.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        if self.workbench.dirty_documents().is_empty() {
            self.finish_clean_exit();
            self.window_close_approved = true;
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.request_close(CloseTarget::Window, ctx);
    }

    /// Paints the prompt and applies the answer.
    ///
    /// The caller reads Escape/Enter before this runs and strips every key and text event from the
    /// frame, so no keystroke can reach an editor while the question is open. The modal is drawn in
    /// egui's foreground layer, so it is painted above the panes even though it is built first.
    fn close_prompt_ui(&mut self, ctx: &egui::Context, escape: bool, enter: bool) {
        self.prompt_buttons.clear();
        let Some(prompt) = self.close.clone() else {
            return;
        };
        let Some(index) = self.workbench.document_index(prompt.current()) else {
            // The document disappeared while the prompt was up; there is nothing to save.
            self.close = None;
            return;
        };
        let title = self.workbench.documents()[index].title.clone();
        let total = prompt.total;
        let remaining = prompt.pending.len();
        let error = prompt.error.clone();
        let mut answer: Option<UnsavedAnswer> = None;
        let modal = egui::Modal::new(egui::Id::unique("unsaved-changes-prompt")).show(ctx, |ui| {
            ui.set_max_width(480.0);
            ui.label(RichText::new("Unsaved changes").strong().size(16.0).color(TEXT));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Save changes to {title} before closing?"))
                    .color(TEXT)
                    .size(14.0),
            );
            if total > 1 {
                ui.label(
                    RichText::new(format!(
                        "Document {} of {} with unsaved changes.",
                        total - remaining + 1,
                        total
                    ))
                    .small()
                    .color(DIM),
                );
            }
            ui.label(
                RichText::new("If you discard, what you changed since the last save is lost.")
                    .small()
                    .color(DIM),
            );
            if let Some(error) = &error {
                ui.add_space(2.0);
                ui.label(RichText::new(error).small().color(ERROR));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                for choice in [
                    UnsavedAnswer::Save,
                    UnsavedAnswer::Discard,
                    UnsavedAnswer::Cancel,
                ] {
                    let text = RichText::new(choice.label()).size(14.0);
                    let text = if choice == UnsavedAnswer::Save {
                        text.strong()
                    } else {
                        text
                    };
                    let response = ui.add(egui::Button::new(text).min_size(vec2(96.0, 26.0)));
                    if response.clicked() {
                        answer = Some(choice);
                    }
                    self.prompt_buttons.push((choice, response.rect));
                }
            });
        });
        if answer.is_none() {
            if escape {
                answer = Some(UnsavedAnswer::Cancel);
            } else if enter {
                answer = Some(UnsavedAnswer::Save);
            } else if modal.should_close() {
                answer = Some(UnsavedAnswer::Cancel);
            }
        }
        if let Some(answer) = answer {
            self.answer_close(answer, ctx);
        }
    }

    /// Paints the DOC-005 recovery prompt and applies the answer.
    ///
    /// Like the close prompt it is a foreground modal and the caller strips every key and text
    /// event, so the journaled text is decided before a save can overwrite anything. Escape (or a
    /// click outside) keeps the question for the next launch instead of deleting the records.
    fn recovery_prompt_ui(&mut self, ctx: &egui::Context, escape: bool) {
        self.recovery_buttons.clear();
        let Some(prompt) = self.recovery_prompt.clone() else {
            return;
        };
        let mut answer: Option<(usize, RecoveryAnswer)> = None;
        let modal = egui::Modal::new(egui::Id::unique("recovery-prompt")).show(ctx, |ui| {
            ui.set_max_width(760.0);
            ui.label(RichText::new("Recover unsaved work").strong().size(16.0).color(TEXT));
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "The last session did not close cleanly. These buffers were journaled before \
                     it ended:",
                )
                .color(TEXT)
                .size(14.0),
            );
            ui.label(
                RichText::new(
                    "Recovering puts the text back into a tab before anything is saved, and runs \
                     nothing. Discarding deletes the record.",
                )
                .small()
                .color(DIM),
            );
            if let Some(error) = &prompt.error {
                ui.add_space(2.0);
                ui.label(RichText::new(error).small().color(ERROR));
            }
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (row, record) in prompt.records.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&record.title).monospace().color(ACCENT));
                            ui.label(
                                RichText::new(format!(
                                    "{}  ·  {} bytes",
                                    record.path_label(),
                                    record.buffer_bytes()
                                ))
                                .small()
                                .color(DIM),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                for choice in [RecoveryAnswer::Discard, RecoveryAnswer::Recover] {
                                    let text = RichText::new(choice.label()).size(14.0);
                                    let text = if choice == RecoveryAnswer::Recover {
                                        text.strong()
                                    } else {
                                        text
                                    };
                                    let response = ui.add(
                                        egui::Button::new(text).min_size(vec2(96.0, 26.0)),
                                    );
                                    if response.clicked() {
                                        answer = Some((row, choice));
                                    }
                                    self.recovery_buttons.push((row, choice, response.rect));
                                }
                            });
                        });
                        ui.separator();
                    }
                });
            ui.add_space(6.0);
            ui.label(
                RichText::new("Escape keeps these records for the next launch.")
                    .small()
                    .color(DIM),
            );
        });
        if answer.is_none() && (escape || modal.should_close()) {
            // Keep everything: the records stay on disk and are offered again next launch.
            self.recovery_prompt = None;
            self.pending_focus = Some(prompt.focus_before.unwrap_or_else(surface_id));
            return;
        }
        if let Some((row, choice)) = answer {
            let Some(record) = prompt.records.get(row).cloned() else {
                return;
            };
            match choice {
                RecoveryAnswer::Recover => self.recover_record(record),
                RecoveryAnswer::Discard => self.discard_record(record),
            }
        }
    }

    /// Sends the active document's name and dirty marker to the native window title.
    fn update_window_title(&mut self, ctx: &egui::Context) {
        let title = format!(
            "Lovelace Desktop - {}",
            self.workbench.active_document().title_with_marker()
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    /// Reads this frame's file shortcut, if any (DOC-002's Ctrl+N/O/S, Ctrl+Shift+S, Ctrl+W).
    fn file_shortcut(&self, ui: &Ui) -> Option<FileCommand> {
        ui.input(|input| {
            for event in &input.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    if !(modifiers.ctrl || modifiers.command) || modifiers.alt {
                        continue;
                    }
                    let letter = match key {
                        Key::N => 'n',
                        Key::O => 'o',
                        Key::S => 's',
                        Key::W => 'w',
                        _ => continue,
                    };
                    if let Some(command) =
                        keyboard::file_command_for_shortcut(letter, true, modifiers.shift)
                    {
                        return Some(command);
                    }
                }
            }
            None
        })
    }

    /// Executes one file command through the same entry points the buttons use.
    fn apply_file_command(&mut self, command: FileCommand, ctx: &egui::Context) {
        // Trace mode only: the file command's stable name, beside the run trace marks.
        crate::run_trace::mark("file_command", command.label());
        match command {
            FileCommand::New => self.new_document(),
            FileCommand::Open => self.open_dialog(),
            FileCommand::Save => self.save_active(false),
            FileCommand::SaveAs => self.save_active(true),
            FileCommand::CloseTab => {
                let id = self.workbench.active_document().id;
                self.request_close(CloseTarget::Tab(id), ctx);
            }
        }
    }

    /// The file bar: New / Open / Save / Save As / Recent / Close / Quit and the active
    /// document's encoding and path.
    fn file_bar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.file_buttons.clear();
        ui.horizontal(|ui| {
            ui.label(RichText::new("File").small().color(DIM));
            if self.file_bar_button(ui, "New", "New document (Ctrl+N)") {
                self.new_document();
            }
            if self.file_bar_button(ui, "Open...", "Open a Lovelace script from disk (Ctrl+O)") {
                self.open_dialog();
            }
            if self.file_bar_button(
                ui,
                "Save",
                "Save the active document (Ctrl+S); asks for a path when it has none",
            ) {
                self.save_active(false);
            }
            if self.file_bar_button(
                ui,
                "Save As...",
                "Save the active document under a new name (Ctrl+Shift+S)",
            ) {
                self.save_active(true);
            }
            let recent = self.workbench.recent_files().to_vec();
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Recent", |ui| {
                    ui.set_min_width(360.0);
                    for path in &recent {
                        if ui.button(path).clicked() {
                            if let Ok(index) = self.workbench.open(std::path::Path::new(path)) {
                                self.sync_after_tab_change(index);
                            }
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text("Files opened or saved in this session");
            });
            ui.separator();
            if self.file_bar_button(
                ui,
                "Close",
                "Close the active tab (Ctrl+W); prompts when it has unsaved changes",
            ) {
                let id = self.workbench.active_document().id;
                self.request_close(CloseTarget::Tab(id), &ctx);
            }
            if self.file_bar_button(
                ui,
                "Quit",
                "Quit Lovelace Desktop; prompts for every unsaved document",
            ) {
                self.request_close(CloseTarget::Window, &ctx);
            }
            ui.separator();
            let document = self.workbench.active_document();
            ui.label(
                RichText::new(document.encoding_label())
                    .monospace()
                    .color(DIM),
            )
            .on_hover_text("The encoding and newline style this document saves with");
            match &document.path {
                Some(path) => {
                    ui.label(
                        RichText::new(path.display().to_string())
                            .monospace()
                            .small()
                            .color(DIM),
                    );
                }
                None => {
                    ui.label(RichText::new("no file yet").small().color(DIM));
                }
            }
        });
    }

    /// One File bar button: records its rectangle for the pointer tests and reports a click.
    fn file_bar_button(&mut self, ui: &mut Ui, label: &'static str, hover: &str) -> bool {
        let response = ui.button(label).on_hover_text(hover);
        self.file_buttons.push((label, response.rect));
        response.clicked()
    }

    fn ui_root(&mut self, ui: &mut Ui) {
        // RUN-001 trace: the frame after the Running frame begins only once that frame has been
        // painted and presented, so its start bounds the paint time.
        if self.trace_after_paint {
            crate::run_trace::mark(
                "frame_after_paint_begin",
                format!("pass={}", ui.ctx().cumulative_pass_nr()),
            );
            self.trace_after_paint = false;
        }
        if crate::run_trace::enabled() && !self.workbench.is_running() {
            crate::run_trace::mark(
                "frame_begin",
                format!("pass={}", ui.ctx().cumulative_pass_nr()),
            );
        }

        let ctx = ui.ctx().clone();
        // A focus request made while the modal was up is applied every pass until it takes
        // effect: the modal's layer is still the top layer for one pass after it stops being
        // painted, so the editor is only interactable again on the pass after that.
        if let Some(id) = self.pending_focus {
            ctx.memory_mut(|memory| memory.request_focus(id));
        }
        // DOC-003: the OS close request (title-bar X, Alt+F4) is either refused with a prompt or
        // allowed through. It is handled before anything else so a close can never race a pane.
        self.handle_close_request(&ctx);
        // While a prompt is open nothing in this frame may consume a keystroke: read the keys the
        // prompt itself uses, then strip every key and text event so no editor can be edited
        // behind the modal. The modal's buttons still receive pointer events, and the prompts are
        // drawn in egui's foreground layer even though they are built before the panes.
        if self.close.is_some() || self.recovery_prompt.is_some() {
            let (escape, enter) = ctx.input(|input| {
                (
                    input.key_pressed(Key::Escape),
                    input.key_pressed(Key::Enter),
                )
            });
            ctx.input_mut(|input| {
                input.events.retain(|event| {
                    !matches!(
                        event,
                        egui::Event::Text(_) | egui::Event::Key { .. } | egui::Event::Paste(_)
                    )
                });
            });
            // The recovery prompt is the outermost question (DOC-005): it is asked before a save
            // can offer to overwrite anything.
            if self.recovery_prompt.is_some() {
                self.recovery_prompt_ui(&ctx, escape);
            } else {
                self.close_prompt_ui(&ctx, escape, enter);
            }
        }

        // Start with keyboard focus in the editor so a typed script lands there without a click.
        if !self.focus_requested {
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(surface_id()));
            self.focus_requested = true;
        }

        // LOG-003: Ctrl+Shift+E is the keyboard way into the quick-eval field. The field is also
        // focusable by clicking it (it is an ordinary egui `TextEdit`); this shortcut only moves
        // keyboard focus, it never submits or edits. The editor's completion popup is dismissed
        // so it cannot linger over a field it no longer owns.
        if ui.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        key: Key::E,
                        pressed: true,
                        modifiers,
                        ..
                    } if (modifiers.ctrl || modifiers.command)
                        && modifiers.shift
                        && !modifiers.alt
                )
            })
        }) {
            for editor in &mut self.editors {
                editor.completion.dismiss();
            }
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(quick_eval_id()));
        }

        // While the quick-eval field owns focus, Esc means "leave the field", not "cancel the
        // document's run". The field itself surrenders focus later in this same frame.
        let quick_eval_focused =
            ui.ctx().memory(|memory| memory.focused()) == Some(quick_eval_id());

        // Ctrl+Enter and Esc are window-global shortcuts: they work while any pane has focus.
        // Read the key events themselves as well as the frame's modifier state, so a shortcut
        // works even when the modifier tick and the key press arrive in the same frame.
        let shortcut = ui.input(|input| {
            let mut enter = false;
            let mut escape = false;
            let mut enter_command = false;
            for event in &input.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    match key {
                        Key::Enter => {
                            enter = true;
                            enter_command |= modifiers.command || modifiers.ctrl;
                        }
                        Key::Escape => escape = true,
                        _ => {}
                    }
                }
            }
            // An open completion popup owns Escape for dismissal; it must not also cancel the
            // document's run. The editor closes the popup later in this same frame.
            let completion_open = self.editors[self.workbench.active()].completion.is_open();
            // While the quick-eval field owns focus, a bare Enter belongs to the field, not to
            // Run. The fallback to the frame's modifier state is only for the editor/other
            // panes: a stale Ctrl bit must not turn the field's submit key into Ctrl+Enter.
            let command_modifier = enter_command
                || (!quick_eval_focused && (input.modifiers.command || input.modifiers.ctrl));
            keyboard::command_for_shortcut(
                enter,
                escape && !completion_open && !quick_eval_focused,
                command_modifier,
            )
        });
        if let Some(command) = shortcut {
            if command == keyboard::WorkbenchCommand::Run && crate::run_trace::enabled() {
                // Trace mode only: measure this frame to its painted end.
                crate::run_trace::mark("key_event_handled", "Ctrl+Enter");
                self.trace_pending = true;
            }
            self.workbench.command(command);
        }

        // DOC-002: the file shortcuts are window-level commands because each of them may open a
        // native dialog. Ctrl+N/O/S, Ctrl+Shift+S and Ctrl+W are mapped in `keyboard`.
        if let Some(command) = self.file_shortcut(ui) {
            self.apply_file_command(command, ui.ctx());
        }

        self.sync();
        if self.trace_pending {
            crate::run_trace::mark("sync_done", "event queue polled and document synced");
        }
        // DOC-005: while a dirty document's newest revision is still inside the debounce window,
        // ask for a frame when that window elapses. The trailing write then happens even if the
        // user stops typing, so a forced kill never loses more than the window.
        if let Some(delay) = self
            .recovery
            .pending_delay(self.workbench.documents(), Instant::now())
        {
            ui.ctx().request_repaint_after(delay);
        }

        // F8 is window-global keyboard navigation to the last failure: it works whether the
        // caret is in the editor or focus sits on the toolbar. The jump is revision-gated by the
        // workbench, so it can never move the caret to a line that no longer exists.
        if ui.input(|input| input.key_pressed(Key::F8)) {
            let active = self.workbench.active();
            if self.editors[active].jump_to_diagnostic() {
                ui.ctx()
                    .memory_mut(|memory| memory.request_focus(surface_id()));
            }
        }

        egui::Panel::top("filebar").show(ui, |ui| self.file_bar(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        if self.trace_pending {
            crate::run_trace::mark("toolbar_drawn", "");
        }
        egui::Panel::top("tabstrip").show(ui, |ui| self.tab_strip(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            self.panes(ui, full);
        });
        // PLT-006: the PNG options window sits over every pane, not inside the plot pane, so a
        // click on it cannot also reach the plot canvas underneath.
        self.plot_export_window(ui.ctx());
        // DOC-002: the native window title carries the active file name and its dirty marker.
        self.update_window_title(ui.ctx());
        // A queued focus request is cleared only once the editor actually holds the focus; while
        // the modal's layer is still the top layer the request cannot take effect, so it is kept
        // and re-requested on the following pass.
        if let Some(id) = self.pending_focus {
            if ctx.memory(|memory| memory.focused()) == Some(id) {
                self.pending_focus = None;
            }
        }
        if self.trace_pending {
            crate::run_trace::mark("panes_drawn", "");
        }

        if self.workbench.is_running() {
            // Poll again soon so a fast worker's terminal event is applied promptly.
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        if self.trace_pending {
            let active = self.workbench.active();
            let (line, column) = self.editors[active].line_col_1based();
            let status = self.workbench.status_text(line, column);
            crate::run_trace::mark("frame_end_after_key", format!("status={status:?}"));
            // Mark "drawn": no trace work remains in this frame after this point. Ask for an
            // immediate repaint so the next frame start proves the paint completed.
            self.trace_pending = false;
            self.trace_after_paint = true;
            ui.ctx().request_repaint();
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Lovelace Desktop")
                    .strong()
                    .color(TEXT)
                    .size(15.0),
            );
            ui.label(RichText::new(self.workbench.engine_name()).color(DIM));
            ui.separator();

            let toolbar = self.workbench.toolbar_state();
            let run = ui
                .add_enabled(
                    toolbar.run_enabled,
                    egui::Button::new(RichText::new("Run").strong()),
                )
                .on_hover_text(
                    "Run (Ctrl+Enter): evaluate the active document on a worker (the window never blocks)",
                );
            if run.clicked() {
                self.workbench.run_active();
            }
            if toolbar.cancel_visible {
                let cancel = ui
                    .add_enabled(
                        toolbar.cancel_enabled,
                        egui::Button::new(RichText::new("Cancel").color(ERROR)),
                    )
                    .on_hover_text("Cancel (Esc): stop the active run at the next statement boundary");
                if cancel.clicked() {
                    self.workbench.cancel_active();
                }
            }
            ui.separator();

            ui.label(RichText::new("Precision").color(DIM));
            let mut precision = self.workbench.active_document().precision;
            let response = ui.add(
                egui::DragValue::new(&mut precision)
                    .range(MIN_PRECISION..=MAX_PRECISION)
                    .speed(1.0),
            );
            if response.changed() {
                self.workbench.set_active_precision(precision);
            }
            ui.label(
                RichText::new(format!(
                    "{} digits",
                    self.workbench.active_document().precision
                ))
                .color(DIM),
            );
            ui.separator();

            let active = self.workbench.active();
            let editor = &self.editors[active];
            ui.label(
                RichText::new(format!(
                    "{} lines / {} bytes",
                    editor.line_count(),
                    editor.bytes()
                ))
                .color(DIM),
            );
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "laid out {} / {} visible rows",
                    editor.metrics.laid_out_rows, editor.metrics.visible_rows
                ))
                .color(DIM),
            );
        });
    }

    fn tab_strip(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs: Vec<String> = self
                .workbench
                .documents()
                .iter()
                .map(|document| document.title_with_marker())
                .collect();
            for (index, title) in tabs.iter().enumerate() {
                let selected = index == self.workbench.active();
                if ui.selectable_label(selected, title).clicked() {
                    self.workbench.set_active(index);
                    // A tab switch is a pointer click outside the editor: drop every popup so a
                    // stale one cannot reappear when the user switches back.
                    for editor in &mut self.editors {
                        editor.completion.dismiss();
                    }
                }
            }
            ui.separator();
            let active = self.workbench.active();
            if let Some(document) = self.workbench.documents().get(active) {
                ui.label(
                    RichText::new(format!(
                        "document {} · revision {}",
                        document.id.0, document.revision
                    ))
                    .color(DIM),
                );
            }
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let active = self.workbench.active();
            let (line, column) = self.editors[active].line_col_1based();
            let status = self.workbench.status_text(line, column);
            if self.trace_pending {
                crate::run_trace::mark("status_bar_drawn", format!("status={status:?}"));
            }
            ui.label(RichText::new(status).monospace().color(ACCENT));
            ui.separator();
            if let Some(document) = self.workbench.documents().get(active) {
                ui.label(RichText::new(format!("{} ms last run", document.elapsed_ms)).color(DIM));
            }
            if self.workbench.active_is_running() {
                ui.separator();
                ui.label(RichText::new("Cancel is available in the toolbar.").color(WARN));
            }
        });
    }

    fn panes(&mut self, ui: &mut Ui, full: Rect) {
        let layout = compute_layout(full, self.editor_frac, self.workspace_frac, self.logs_frac);
        self.layout = Some(layout);
        ui.painter()
            .rect_filled(full, 0.0, Color32::from_rgb(0x0b, 0x0e, 0x12));

        // RUN-003: one view for the whole frame. The clone is what a pane closure can borrow
        // while another pane mutates `self`; no pane reads pane data from the workbench.
        let view = self.view.clone();
        self.editor_pane(ui, layout.editor, &view);
        self.plot_pane(ui, layout.plot, &view);
        self.workspace_pane(ui, layout.workspace, &view);
        self.logs_pane(ui, layout.logs, &view);

        let drag_a = splitter(
            ui,
            layout.split_a,
            "split_editor",
            true,
            "Editor / plot splitter",
        );
        let drag_b = splitter(
            ui,
            layout.split_b,
            "split_workspace",
            true,
            "Plot / workspace splitter",
        );
        let drag_c = splitter(
            ui,
            layout.split_c,
            "split_logs",
            false,
            "Workspace / logs splitter",
        );

        if drag_a != 0.0 {
            self.editor_frac =
                (self.editor_frac + drag_a / full.width().max(1.0)).clamp(0.02, 0.95);
        }
        if drag_b != 0.0 {
            self.workspace_frac =
                (self.workspace_frac - drag_b / full.width().max(1.0)).clamp(0.02, 0.95);
        }
        if drag_c != 0.0 {
            self.logs_frac = (self.logs_frac - drag_c / full.height().max(1.0)).clamp(0.02, 0.9);
        }
    }

    fn editor_pane(&mut self, ui: &mut Ui, rect: Rect, view: &DocumentView) {
        let active = self.workbench.active();
        let (title, encoding) = match self.workbench.document(active) {
            Some(document) => (document.title.clone(), document.encoding_label()),
            None => return,
        };
        let lines = self.editors[active].line_count();
        let bytes = self.editors[active].bytes();
        // A run failure is also stated in the pane header, so the position and message are
        // readable (and visible to accessibility clients) without hovering the underline. The
        // frame's assembled view already applied the revision gate.
        let diagnostic = view.diagnostic.clone();
        // The live catalogue: keywords, the engine's registered functions and the active
        // document's last committed variables/user functions. Built fresh each pass so an
        // engine snapshot landing or a tab switch is reflected immediately. The same capture is
        // handed to the editor, whose highlighter uses its builtin flags without the catalogue's
        // shadowing dedup (a variable named like a builtin must not hide that builtin's colour).
        let engine_state = self.workbench.completion_state();
        let catalogue = crate::completion::build_catalogue(
            crate::language::KEYWORDS,
            &engine_state.functions,
            &engine_state.variables,
        );
        let pane_title = format!("Editor - {title}");
        with_pane(ui, rect, &pane_title, |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                ui.label(RichText::new(&title).color(ACCENT).monospace());
                ui.label(RichText::new(format!("{lines} lines / {bytes} bytes")).color(DIM));
                ui.label(RichText::new(&encoding).monospace().small().color(DIM))
                    .on_hover_text(
                        "The document's encoding and newline style; Save writes them back \
                         unchanged unless the file is edited",
                    );
                if let Some(diagnostic) = &diagnostic {
                    ui.separator();
                    ui.label(
                        RichText::new(diagnostic_summary(diagnostic))
                            .monospace()
                            .color(ERROR),
                    )
                    .on_hover_text(diagnostic.render());
                }
                // The popup itself is painted; the header states its selected signature so the
                // completion also exists in the accessibility tree and in a UIA capture.
                if let Some(summary) = self.editors[active].completion_summary() {
                    ui.separator();
                    ui.label(RichText::new(summary).monospace().color(ACCENT));
                }
            });
            let before = self.editors[active].edits;
            let metrics = self.editors[active].ui(ui, body, &catalogue, &engine_state);
            if self.editors[active].edits != before {
                // An edit makes the old position describe text that is gone. Drop it now, not
                // only on the next frame, so the paint pass cannot show a stale underline.
                self.editors[active].diagnostic = None;
                let source = self.editors[active].source();
                self.workbench.set_source(active, source);
                self.loaded_revisions[active] = self.workbench.documents()[active].revision;
            }
            if metrics.laid_out_rows == 0 {
                ui.painter().text(
                    body.center(),
                    Align2::CENTER_CENTER,
                    "This document is empty. Type a Lovelace script.",
                    FontId::proportional(13.0),
                    DIM,
                );
            }
        });
    }

    fn plot_pane(&mut self, ui: &mut Ui, rect: Rect, view: &DocumentView) {
        let plot_title = view.plot.as_ref().map(|plot| plot.title.clone());
        let empty = self.plot.is_empty();
        with_pane(ui, rect, "Plot - engine SVG", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                // PLT-004: Fit restores the whole plot; 100% is the natural-size reset (one SVG
                // point per logical screen pixel) and is a different view on a pane whose fit
                // scale is not 1:1. Both are one click, and the wheel/drag handlers in
                // `plot.rs` apply the same clamped transform.
                if ui
                    .button("Fit")
                    .on_hover_text("Show the whole plot in the pane")
                    .clicked()
                {
                    self.plot.fit();
                }
                if ui
                    .button("100%")
                    .on_hover_text("Natural size: one SVG point per screen pixel")
                    .clicked()
                {
                    self.plot.actual_size();
                }
                if ui
                    .button("Zoom out")
                    .on_hover_text("Zoom out one step around the pointer")
                    .clicked()
                {
                    self.plot.zoom_step(1.0 / plot::WHEEL_STEP);
                }
                if ui
                    .button("Zoom in")
                    .on_hover_text("Zoom in one step around the pointer")
                    .clicked()
                {
                    self.plot.zoom_step(plot::WHEEL_STEP);
                }
                ui.label(
                    RichText::new(format!("{:.0}%", self.plot.zoom_percent()))
                        .monospace()
                        .color(DIM),
                )
                .on_hover_text("Zoom as a percentage of natural size");
                ui.separator();
                match &plot_title {
                    Some(title) => {
                        ui.label(RichText::new(title).color(ACCENT));
                    }
                    None => {
                        ui.label(RichText::new("no engine plot").color(DIM));
                    }
                }
                ui.separator();
                // PLT-006: both exports go through the native save dialog. SVG copies the
                // engine's captured bytes; PNG opens an in-window options step first.
                if ui
                    .add_enabled(!empty, egui::Button::new("Export SVG..."))
                    .on_hover_text("Save the engine's original SVG bytes unchanged")
                    .clicked()
                {
                    self.export_plot_svg();
                }
                if ui
                    .add_enabled(!empty, egui::Button::new("Export PNG..."))
                    .on_hover_text("Rasterise at a chosen scale and background")
                    .clicked()
                {
                    self.plot_export = Some(PngExport::default());
                }
                if let Some((status, error)) = &self.plot_export_status {
                    ui.label(
                        RichText::new(status)
                            .small()
                            .color(if *error { ERROR } else { ACCENT }),
                    );
                }
            });
            if empty {
                self.plot.ui(ui, body);
                return;
            }
            let info = format!(
                "raster {}x{} @ {:.2}x · {:.2} ms raster · {:.2} ms upload",
                self.plot.raster_px[0],
                self.plot.raster_px[1],
                self.plot.raster_scale,
                self.plot.last_raster_ms,
                self.plot.last_upload_ms
            );
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(Rect::from_min_size(body.min, vec2(body.width(), 18.0)))
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.label(RichText::new(info).small().color(DIM));
                },
            );
            let canvas = Rect::from_min_max(pos2(body.left(), body.top() + 18.0), body.max);
            self.plot.ui(ui, canvas);
        });
    }

    /// PLT-006: writes the captured `PlotCapture.svg` through the native save dialog.
    fn export_plot_svg(&mut self) {
        let title = self.plot_title();
        let suggested = plot::export_file_name(title.as_deref(), "svg");
        let Some(path) = plot::save_plot_dialog(
            "Export plot as SVG",
            plot::SVG_FILTER,
            "svg",
            &suggested,
            self.plot_export_dir().as_deref(),
        ) else {
            self.plot_export_status = Some(("SVG export cancelled".to_string(), false));
            return;
        };
        self.plot_export_status = Some(match self.plot.export_svg_to(&path) {
            Ok(bytes) => (
                format!(
                    "SVG: {bytes} bytes -> {}",
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string())
                ),
                false,
            ),
            Err(error) => (error, true),
        });
    }

    /// PLT-006: rasterises at the chosen scale/background and writes it through the dialog.
    fn export_plot_png(&mut self, scale: f32, background: PngBackground) {
        let title = self.plot_title();
        let suggested = plot::export_file_name(title.as_deref(), "png");
        let Some(path) = plot::save_plot_dialog(
            "Export plot as PNG",
            plot::PNG_FILTER,
            "png",
            &suggested,
            self.plot_export_dir().as_deref(),
        ) else {
            self.plot_export_status = Some(("PNG export cancelled".to_string(), false));
            return;
        };
        self.plot_export_status = Some(match self.plot.export_png_to(&path, scale, background) {
            Ok([width, height]) => (
                format!(
                    "PNG: {width}x{height} px {} -> {}",
                    background.label(),
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string())
                ),
                false,
            ),
            Err(error) => (error, true),
        });
    }

    fn plot_title(&self) -> Option<String> {
        self.view.plot.as_ref().map(|plot| plot.title.clone())
    }

    /// The folder the save dialog starts in: where the engine wrote this plot's capture.
    ///
    /// The engine has already created and written that directory, so it is writable; starting
    /// the dialog there avoids a shell default folder this process cannot write (which would
    /// turn every export into a permission prompt) and keeps the exported file next to the
    /// capture it came from.
    fn plot_export_dir(&self) -> Option<PathBuf> {
        self.view
            .plot
            .as_ref()
            .map(|plot| PathBuf::from(&plot.path))
            .and_then(|path| path.parent().map(|directory| directory.to_path_buf()))
            .filter(|directory| directory.is_dir())
    }

    /// PLT-006: the PNG export options window, drawn over the workbench.
    fn plot_export_window(&mut self, ctx: &egui::Context) {
        let Some(mut options) = self.plot_export else {
            return;
        };
        let mut open = true;
        let mut close = false;
        let mut save = false;
        let dimensions = match plot::export_pixels(
            self.plot.svg_w,
            self.plot.svg_h,
            options.scale,
        ) {
            Ok((width, height)) => format!("{width} x {height} px"),
            Err(error) => error,
        };
        egui::Window::new("Export PNG")
            .id(Id::unique("plot_export_png"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new("Scale").color(DIM));
                ui.horizontal(|ui| {
                    for scale in [1.0f32, 2.0, 4.0] {
                        let selected = (options.scale - scale).abs() < f32::EPSILON;
                        let label = format!("{scale:.0}x");
                        if ui
                            .add(egui::Button::new(if selected {
                                RichText::new(label).strong().color(ACCENT)
                            } else {
                                RichText::new(label)
                            }))
                            .clicked()
                        {
                            options.scale = scale;
                        }
                    }
                });
                ui.label(RichText::new("Background").color(DIM));
                ui.horizontal(|ui| {
                    for background in [PngBackground::Opaque, PngBackground::Transparent] {
                        let selected = options.background == background;
                        let label = background.label();
                        if ui
                            .add(egui::Button::new(if selected {
                                RichText::new(label).strong().color(ACCENT)
                            } else {
                                RichText::new(label)
                            }))
                            .clicked()
                        {
                            options.background = background;
                        }
                    }
                });
                ui.separator();
                ui.label(format!("Output: {dimensions}"));
                ui.label(
                    RichText::new(format!(
                        "Selected: {:.0}x, {}",
                        options.scale,
                        options.background.label()
                    ))
                    .color(DIM),
                );
                ui.horizontal(|ui| {
                    if ui.button("Save PNG...").clicked() {
                        save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        self.plot_export = if open && !close && !save {
            Some(options)
        } else {
            None
        };
        if save {
            self.export_plot_png(options.scale, options.background);
        }
    }

    fn workspace_pane(&mut self, ui: &mut Ui, rect: Rect, view: &DocumentView) {
        let has_snapshot = view.has_snapshot;
        // RUN-003: the pane states the revision and committed prefix it is showing, from the
        // same DocumentView as the variables below it. The status strip reports the live buffer
        // revision; this line reports the committed snapshot's revision.
        let state_line = if has_snapshot {
            format!(
                "rev {} · committed {}/{}",
                view.revision.0, view.committed_steps, view.total
            )
        } else {
            format!("rev {} · no committed statement", view.revision.0)
        };
        let mut filter = self.filter.clone();
        // WKS-001/WKS-002: the table rows, their counts and their sort state all come from the
        // frame's one committed capture. The counts are the table row counts, so a displayed
        // count cannot disagree with the rows beside it or with the snapshot's registry capture.
        let variable_count = self.variables_table.rows.len();
        let function_count = self.functions_table.rows.len();
        let variable_rows = self.variables_table.rows.clone();
        let function_rows = self.functions_table.rows.clone();
        let mut variable_sort = self.variables_table.sort;
        let mut function_sort = self.functions_table.sort;
        let mut variable_header_rects: [Option<Rect>; 3] = [None; 3];
        let mut function_header_rects: [Option<Rect>; 3] = [None; 3];
        // RES-004: the RESULT tree's collapse state and the headers this pass paints. The state
        // lives on the window, not in the paint pass, so a repaint or an unrelated update (a
        // quick eval, an edit) cannot reset it; `paint_result_section` clears it only when the
        // committed result changes.
        let expansion = &mut self.expansion;
        let inspector_expansion = &mut self.inspector_expansion;
        let mut inspector_open = self.inspector_open;
        let mut inspector_toggle_rects: Vec<(String, Rect)> = Vec::new();
        let mut workspace_view_tabs: [Option<Rect>; 2] = [None; 2];
        let mut result_toggle_rects: Vec<(String, Rect)> = Vec::new();
        with_pane(ui, rect, "Workspace", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                let label = ui.label("Filter");
                ui.add(
                    egui::TextEdit::singleline(&mut filter)
                        .hint_text("variable or function")
                        .desired_width(130.0),
                )
                .labelled_by(label.id);
                ui.label(
                    RichText::new(format!("{variable_count} vars · {function_count} funcs"))
                        .color(DIM),
                )
                .on_hover_text(format!(
                    "{variable_count} variables · {function_count} functions"
                ));
            });
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(body.shrink(8.0))
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    // RES-001: constrain the result tree to this pane's width. The tree's
                    // symbolic renderings are long, and a ScrollArea left to size itself from
                    // its widest child would let them widen the content and clip at the border.
                    // The 96 px reserve covers the nested record/array indentation: egui's
                    // indents move the cursor without shrinking the wrap width, so a label
                    // wrapping at the full pane width would still end under the border.
                    let result_width = (body.width() - 96.0).max(120.0);
                    // SYM-001: the Workspace pane has two views over the same committed
                    // snapshot. The Inspector tab never reads the editor buffer; it renders
                    // only `view.result_inspection`, which came from the last commit.
                    ui.horizontal(|ui| {
                        let result_tab = ui.selectable_label(!inspector_open, "Result");
                        if result_tab.clicked() {
                            inspector_open = false;
                        }
                        workspace_view_tabs[0] = Some(result_tab.rect);
                        let inspector_tab = ui.selectable_label(inspector_open, "Inspector");
                        if inspector_tab.clicked() {
                            inspector_open = true;
                        }
                        workspace_view_tabs[1] = Some(inspector_tab.rect);
                        ui.label(RichText::new(&state_line).small().color(ACCENT));
                    });
                    ui.add_space(4.0);
                    if inspector_open {
                        egui::ScrollArea::vertical()
                            .id_salt("workspace_inspector_scroll")
                            .show(ui, |ui| {
                                paint_inspector(
                                    ui,
                                    view,
                                    result_width,
                                    inspector_expansion,
                                    &mut inspector_toggle_rects,
                                );
                            });
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("workspace_scroll")
                            .show(ui, |ui| {
                                ui.add_space(4.0);

                                // RES-001: the last committed result's typed tree, built only from
                                // the engine's structured projection. The display text is a caption;
                                // it is never parsed for structure.
                                paint_result_section(
                                    ui,
                                    view,
                                    result_width,
                                    expansion,
                                    &mut result_toggle_rects,
                                );

                            ui.add_space(6.0);
                            ui.separator();
                            ui.label(
                                RichText::new(format!("VARIABLES · {variable_count}"))
                                    .small()
                                    .color(DIM),
                            );
                            let widths = variable_column_widths(ui.available_width());
                            ui.horizontal(|ui| {
                                for column in VariableColumn::ALL {
                                    let response = ui.add_sized(
                                        [widths[column.index()], 18.0],
                                        egui::Button::new(
                                            RichText::new(
                                                variable_sort.header_text(column, column.label()),
                                            )
                                            .small(),
                                        )
                                        .frame(false),
                                    );
                                    let direction = if variable_sort.is_active(column) {
                                        variable_sort.direction_label()
                                    } else {
                                        "not sorted"
                                    };
                                    variable_header_rects[column.index()] = Some(response.rect);
                                    response.clone().on_hover_text(format!(
                                        "Sort variables by {} ({direction})",
                                        column.label().to_lowercase()
                                    ));
                                    if response.clicked() {
                                        variable_sort.clicked(column);
                                    }
                                }
                            });
                            ui.add_space(1.0);

                            let mut shown = 0usize;
                            for index in ordered_variables(&variable_rows, variable_sort) {
                                let row = &variable_rows[index];
                                if !filter.is_empty() && !row.name.contains(filter.as_str()) {
                                    continue;
                                }
                                shown += 1;
                                ui.horizontal(|ui| {
                                    table_cell(
                                        ui,
                                        widths[0],
                                        RichText::new(cell_text(&row.name))
                                            .monospace()
                                            .color(ACCENT),
                                        &row.name,
                                    );
                                    table_cell(
                                        ui,
                                        widths[1],
                                        RichText::new(cell_text(&row.kind)).monospace().color(DIM),
                                        &row.kind,
                                    );
                                    // WKS-001: a large display value is cut in the cell; the
                                    // full value stays available as the hover text.
                                    table_cell(
                                        ui,
                                        widths[2],
                                        RichText::new(row.display_cell()).monospace().color(TEXT),
                                        &row.display,
                                    );
                                });
                            }
                            if shown == 0 {
                                let message = if !has_snapshot {
                                    "No run has published a workspace yet."
                                } else if variable_rows.is_empty() {
                                    "The last run defined no variables."
                                } else {
                                    "No variables match the filter."
                                };
                                ui.label(RichText::new(message).color(DIM));
                            }

                            ui.add_space(8.0);
                            ui.separator();
                            ui.label(
                                RichText::new(format!("FUNCTIONS · {function_count}"))
                                    .small()
                                    .color(DIM),
                            );
                            let widths = function_column_widths(ui.available_width());
                            ui.horizontal(|ui| {
                                for column in FunctionColumn::ALL {
                                    let response = ui.add_sized(
                                        [widths[column.index()], 18.0],
                                        egui::Button::new(
                                            RichText::new(
                                                function_sort.header_text(column, column.label()),
                                            )
                                            .small(),
                                        )
                                        .frame(false),
                                    );
                                    let direction = if function_sort.is_active(column) {
                                        function_sort.direction_label()
                                    } else {
                                        "not sorted"
                                    };
                                    function_header_rects[column.index()] = Some(response.rect);
                                    response.clone().on_hover_text(format!(
                                        "Sort functions by {} ({direction})",
                                        column.label().to_lowercase()
                                    ));
                                    if response.clicked() {
                                        function_sort.clicked(column);
                                    }
                                }
                            });
                            ui.add_space(1.0);

                            let mut shown = 0usize;
                            for index in ordered_functions(&function_rows, function_sort) {
                                let row = &function_rows[index];
                                if !filter.is_empty() && !row.name.contains(filter.as_str()) {
                                    continue;
                                }
                                shown += 1;
                                ui.horizontal(|ui| {
                                    table_cell(
                                        ui,
                                        widths[0],
                                        RichText::new(cell_text(&row.name))
                                            .monospace()
                                            .color(ACCENT),
                                        &row.name,
                                    );
                                    let parameters = row.parameters_text();
                                    table_cell(
                                        ui,
                                        widths[1],
                                        RichText::new(cell_text(&parameters))
                                            .monospace()
                                            .color(TEXT),
                                        &parameters,
                                    );
                                    let origin_hover = match &row.plugin {
                                        Some(plugin) => format!("plugin: {plugin}"),
                                        None => row.origin_label().to_string(),
                                    };
                                    table_cell(
                                        ui,
                                        widths[2],
                                        RichText::new(row.origin_label())
                                            .monospace()
                                            .color(origin_color(row.origin)),
                                        &origin_hover,
                                    );
                                });
                                // WKS-002: the registry's summary, when it publishes one, is a
                                // second line under the function; it is cut like any other cell
                                // and the full summary is the hover text.
                                if let (Some(summary_cell), Some(summary)) =
                                    (row.summary_cell(), row.summary.as_deref())
                                {
                                    ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        ui.allocate_ui_with_layout(
                                            vec2((ui.available_width() - 6.0).max(40.0), 14.0),
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(summary_cell)
                                                            .small()
                                                            .color(DIM),
                                                    )
                                                    .truncate(),
                                                )
                                                .on_hover_text(summary.to_string());
                                            },
                                        );
                                    });
                                }
                            }
                            if shown == 0 {
                                let message = if !has_snapshot {
                                    "No run has published a workspace yet."
                                } else if function_rows.is_empty() {
                                    "The engine published no functions."
                                } else {
                                    "No functions match the filter."
                                };
                                ui.label(RichText::new(message).color(DIM));
                            }
                        });
                    }
                },
            );
        });
        self.filter = filter;
        self.variables_table.sort = variable_sort;
        self.functions_table.sort = function_sort;
        self.variable_header_rects = variable_header_rects;
        self.function_header_rects = function_header_rects;
        self.result_toggle_rects = result_toggle_rects;
        self.inspector_open = inspector_open;
        self.inspector_toggle_rects = inspector_toggle_rects;
        self.workspace_view_tabs = workspace_view_tabs;
    }

    fn logs_pane(&mut self, ui: &mut Ui, rect: Rect, view: &DocumentView) {
        let active = self.workbench.active();
        let legend = log_kind_legend();
        with_pane(ui, rect, "Logs", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                let (line, column) = self.editors[active].line_col_1based();
                ui.label(
                    RichText::new(format!(
                        "run log · caret Ln {line}, Col {column} · engine {}",
                        self.workbench.engine_name()
                    ))
                    .color(DIM),
                )
                .on_hover_text(format!("Log kinds: {legend}"));
            });
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(body.shrink2(vec2(10.0, 4.0)))
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    // RUN-002: the quick-eval one-liner lives above the log list, so it stays
                    // visible with an empty log and cannot be confused with a history row.
                    self.quick_eval_row(ui);
                    let logs = &view.logs;
                    if logs.is_empty() {
                        ui.label(RichText::new("No log entries yet. Run the document.").color(DIM));
                        // The legend names every kind and marker before the first line exists,
                        // so the vocabulary is discoverable even in an empty pane.
                        ui.label(RichText::new(legend.clone()).small().color(DIM));
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("logs_scroll")
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in logs {
                                // LOG-002: the marker is part of the line's rendered model, so
                                // the pane paints exactly the string the unit tests pin.
                                let color = match entry.kind {
                                    LogKind::Echo => LOG_ECHO,
                                    LogKind::Stdout => TEXT,
                                    LogKind::Result => ACCENT,
                                    LogKind::Diagnostic => ERROR,
                                    LogKind::Timing => WARN,
                                    LogKind::Progress => LOG_PROGRESS,
                                    LogKind::Info => DIM,
                                };
                                ui.label(RichText::new(entry.render()).monospace().color(color));
                            }
                        });
                },
            );
        });
    }

    /// The one-line quick-eval field and its keyboard contract (RUN-002 / LOG-003).
    ///
    /// The label is visible and the hint text is the reference placeholder. Enter submits; Esc
    /// leaves the field without submitting and without editing the document; Up/Down restore
    /// submitted expressions newest first. Only an accepted submission clears the field.
    fn quick_eval_row(&mut self, ui: &mut Ui) {
        let mut submit = false;
        let mut navigate: i8 = 0;
        ui.horizontal(|ui| {
            let label = ui.label(RichText::new("Quick eval").strong().color(TEXT));
            let id = quick_eval_id();
            let response = ui
                .add(
                    egui::TextEdit::singleline(self.quick_eval.text_mut())
                        .id(id)
                        .hint_text("quick eval — one-liner, press Enter")
                        .desired_width(340.0),
                )
                .labelled_by(label.id);
            self.quick_eval_rect = Some(response.rect);

            if response.has_focus() {
                // Keep Up/Down and Esc for this field. Otherwise egui's focus navigation would
                // move focus before the history handler below can act, and Esc could surrender
                // focus without the field observing it (the DEF-002 class the editor locks its
                // own arrows for). The filter applies to the NEXT pass, so it is installed as
                // soon as the field holds focus.
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        id,
                        egui::EventFilter {
                            tab: false,
                            escape: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                        },
                    );
                });
                if ui.input(|input| input.key_pressed(Key::Escape)) {
                    // Esc leaves the field without submitting and without clearing its text.
                    response.surrender_focus();
                } else if key_pressed_without_modifiers(ui, Key::ArrowUp) {
                    navigate = -1;
                } else if key_pressed_without_modifiers(ui, Key::ArrowDown) {
                    navigate = 1;
                }
            }

            // A single-line egui TextEdit surrenders focus on a bare Enter, so `lost_focus` plus
            // the Enter key is the submit edge. Ctrl+Enter keeps focus and remains the global
            // Run shortcut, which is why this looks at the key event's own modifiers.
            if response.lost_focus() && key_pressed_without_modifiers(ui, Key::Enter) {
                submit = true;
            }

            ui.label(RichText::new("Ctrl+Shift+E").small().color(DIM))
                .on_hover_text("Focus quick eval");
            ui.label(
                RichText::new("Enter runs · Esc leaves the field")
                    .small()
                    .color(DIM),
            );
        });

        if navigate < 0 {
            self.quick_eval.navigate_up();
        } else if navigate > 0 {
            self.quick_eval.navigate_down();
        }
        if submit {
            self.submit_quick_eval();
            // Enter already surrendered the TextEdit's focus; return it so a sequence of quick
            // evaluations needs no click and a refused expression stays editable.
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(quick_eval_id()));
        }
        ui.add_space(2.0);
    }

    /// Submits the field's expression; clears it only when the workbench accepted the request.
    fn submit_quick_eval(&mut self) {
        let Some(expression) = self.quick_eval.expression().map(str::to_string) else {
            return;
        };
        if self.workbench.quick_eval_active(&expression) {
            self.quick_eval.accept(&expression);
        }
        // On refusal the workbench already logged its [info] advisory; the field is left exactly
        // as typed so the expression can be retried.
    }
}

impl eframe::App for WorkbenchApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.ui_root(ui);
    }
}

/// The INSPECTOR view: the engine's six symbolic sections for the last committed result.
///
/// Every value painted here comes from `view.result_inspection`, the payload the engine adapter
/// captured beside the result when it was committed. The editor buffer is not read, and no
/// section is reconstructed from a display string. A section the engine did not supply paints
/// the engine's (or the adapter's) specific reason instead of an empty box.
fn paint_inspector(
    ui: &mut Ui,
    view: &DocumentView,
    width: f32,
    expansion: &mut InspectorExpansion,
    toggles: &mut Vec<(String, Rect)>,
) {
    ui.set_max_width(width.max(120.0));
    ui.label(RichText::new("INSPECTOR · last committed result").small().color(DIM));
    if !view.has_snapshot {
        ui.label(RichText::new("No run has published a result yet.").color(DIM));
        return;
    }
    let Some(payload) = view.result_inspection.as_ref() else {
        ui.label(
            RichText::new(
                "The engine published no inspection payload for this result. It supplies no \
                 pretty, canonical, tree, assumptions, trace or MathIR for a result it did not \
                 inspect.",
            )
            .small()
            .color(WARN),
        );
        return;
    };
    // SYM-001: bind the tree's collapse map to this payload. The payload is committed data, so
    // an edit to the buffer cannot clear the map while a new committed result can.
    expansion.bind(payload);
    let Some(inspection) = Inspection::from_json(payload) else {
        ui.label(
            RichText::new(
                "The engine's inspection payload was not readable (unexpected desktop schema); \
                 no section is shown.",
            )
            .small()
            .color(WARN),
        );
        return;
    };

    let subject = &inspection.subject;
    let detail = subject
        .detail
        .as_deref()
        .map(|detail| format!(" · {detail}"))
        .unwrap_or_default();
    ui.label(
        RichText::new(format!("subject  {} · {}{detail}", subject.kind, subject.path))
            .monospace()
            .color(ACCENT),
    );

    paint_inspector_heading(ui, "Pretty", "");
    match &inspection.pretty {
        Section::Supplied(text) => paint_inspector_text(ui, text),
        Section::Missing(reason) => paint_inspector_reason(ui, reason),
    }

    paint_inspector_heading(ui, "Canonical", "");
    match &inspection.canonical {
        Section::Supplied(text) => paint_inspector_text(ui, text),
        Section::Missing(reason) => paint_inspector_reason(ui, reason),
    }

    match &inspection.tree {
        Section::Supplied(tree) => {
            paint_inspector_heading(ui, "Expression tree", &count_label(tree.count(), "node", "nodes"));
            paint_inspector_node(ui, tree, TREE_ROOT_PATH, expansion, toggles);
        }
        Section::Missing(reason) => {
            paint_inspector_heading(ui, "Expression tree", "");
            paint_inspector_reason(ui, reason);
        }
    }

    match &inspection.assumptions {
        Section::Supplied(assumptions) => {
            paint_inspector_heading(
                ui,
                "Assumptions",
                &count_label(assumptions.len(), "row", "rows"),
            );
            if assumptions.is_empty() {
                ui.label(
                    RichText::new("No active assumptions constrain the free symbols of this result.")
                        .small()
                        .color(DIM),
                );
            }
            for assumption in assumptions {
                paint_inspector_assumption(ui, assumption);
            }
        }
        Section::Missing(reason) => {
            paint_inspector_heading(ui, "Assumptions", "");
            paint_inspector_reason(ui, reason);
        }
    }

    match &inspection.trace {
        Section::Supplied(steps) => {
            paint_inspector_heading(ui, "Simplification trace", &count_label(steps.len(), "step", "steps"));
            if steps.is_empty() {
                ui.label(
                    RichText::new(
                        "The engine's TransformResult recorded 0 rewrite steps; the expression \
                         was already in its target form.",
                    )
                    .small()
                    .color(DIM),
                );
            }
            for (index, step) in steps.iter().enumerate() {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("{}. {}", index + 1, step.rule_id))
                        .monospace()
                        .color(ACCENT),
                );
                ui.label(
                    RichText::new(format!("classification  {}", step.classification))
                        .small()
                        .color(DIM),
                );
                ui.label(
                    RichText::new(format!("before  {}", step.before))
                        .monospace()
                        .color(TEXT),
                );
                if !step.before_canonical.is_empty() {
                    ui.label(
                        RichText::new(format!("canonical  {}", step.before_canonical))
                            .small()
                            .monospace()
                            .color(DIM),
                    );
                }
                ui.label(
                    RichText::new(format!("after   {}", step.after))
                        .monospace()
                        .color(TEXT),
                );
                if !step.after_canonical.is_empty() {
                    ui.label(
                        RichText::new(format!("canonical  {}", step.after_canonical))
                            .small()
                            .monospace()
                            .color(DIM),
                    );
                }
                if step.conditions.is_empty() {
                    ui.label(
                        RichText::new("requires  none")
                            .small()
                            .monospace()
                            .color(DIM),
                    );
                } else {
                    ui.label(RichText::new("requires").small().color(DIM));
                    for condition in &step.conditions {
                        paint_inspector_assumption(ui, condition);
                    }
                }
            }
        }
        Section::Missing(reason) => {
            paint_inspector_heading(ui, "Simplification trace", "");
            paint_inspector_reason(ui, reason);
        }
    }

    match &inspection.mathir {
        Section::Supplied(mathir) => {
            let result_type = mathir
                .result_type
                .as_deref()
                .map(|kind| format!(" · result {kind}"))
                .unwrap_or_default();
            paint_inspector_heading(
                ui,
                "MathIR",
                &format!(
                    "compile · parameters [{}]{result_type}",
                    mathir.parameters.join(", ")
                ),
            );
            // SYM-007 groundwork: monospaced, selectable, and scrollable both ways. The outer
            // pane scrolls vertically; this inner area bounds the IR's own height and width.
            egui::ScrollArea::both()
                .id_salt("inspector_mathir")
                .max_height(240.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&mathir.text).monospace().color(TEXT))
                            .selectable(true)
                            .extend(),
                    );
                });
        }
        Section::Missing(reason) => {
            paint_inspector_heading(ui, "MathIR", "");
            paint_inspector_reason(ui, reason);
        }
    }
}

/// `1 node` / `5 nodes`, so a one-item section does not read as a count bug.
fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn paint_inspector_heading(ui: &mut Ui, title: &str, detail: &str) {
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(title).small().color(DIM));
        if !detail.is_empty() {
            ui.label(RichText::new(detail).small().color(ACCENT));
        }
    });
}

fn paint_inspector_text(ui: &mut Ui, text: &str) {
    ui.add(
        egui::Label::new(RichText::new(text).monospace().color(TEXT))
            .selectable(true)
            .wrap(),
    );
}

fn paint_inspector_reason(ui: &mut Ui, reason: &str) {
    ui.label(RichText::new(reason).small().color(WARN));
}

fn paint_inspector_assumption(ui: &mut Ui, assumption: &crate::inspector::Assumption) {
    ui.horizontal_wrapped(|ui| {
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("[{}]", assumption.source))
                .small()
                .color(DIM),
        );
        ui.label(
            RichText::new(assumption.type_label())
                .small()
                .color(ACCENT),
        );
        ui.label(RichText::new(&assumption.label).monospace().color(TEXT));
    });
    if let Some(canonical) = &assumption.canonical {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(12.0);
            ui.label(RichText::new("canonical").small().color(DIM));
            ui.label(
                RichText::new(canonical)
                    .small()
                    .monospace()
                    .color(DIM),
            );
        });
    }
}

/// Paints one expression-tree node. A node with children is a collapsible button header whose
/// accessible name carries the node kind, the engine pretty rendering, the node path and the
/// state; a leaf is a single indented row. Child order is the engine's child order.
fn paint_inspector_node(
    ui: &mut Ui,
    node: &crate::inspector::TreeNode,
    path: &str,
    expansion: &mut InspectorExpansion,
    toggles: &mut Vec<(String, Rect)>,
) {
    let label = node.header();
    if node.children.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(label).monospace().color(TEXT));
        });
        return;
    }
    let visible = format!("{} {}", expansion.glyph(path), label);
    let response = ui
        .push_id(path, |ui| {
            ui.add(egui::Button::new(RichText::new(visible).monospace().color(ACCENT)).frame(false))
        })
        .inner;
    if response.clicked() {
        expansion.toggle(path);
        ui.memory_mut(|memory| memory.request_focus(response.id));
        ui.ctx().request_repaint();
    }
    let accessible = expansion.accessible_name(path, &label);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, accessible.clone()));
    let action = if expansion.is_collapsed(path) {
        "expand"
    } else {
        "collapse"
    };
    response.clone().on_hover_text(format!(
        "{path} · {} — click, or press Space/Enter, to {action}",
        expansion.state_word(path)
    ));
    toggles.push((path.to_string(), response.rect));
    if !expansion.is_collapsed(path) {
        ui.indent(path, |ui| {
            for (index, child) in node.children.iter().enumerate() {
                paint_inspector_node(ui, child, &tree_path(path, index), expansion, toggles);
            }
        });
    }
}

/// The RESULT section: the engine's display text and the typed tree beside it.
///
/// The tree is built by [`ResultProjection::tree`] from `view.result_structured` alone; the
/// display string is painted as a caption and is never parsed. An engine that publishes no
/// projection gets an explicit empty state rather than a display-derived guess.
///
/// RES-004: `expansion` is the per-path collapse state of this result. It is bound to the
/// projection before painting, so a repaint of the same committed value keeps every decision
/// while a new result clears the map. `toggles` receives every painted header's path and
/// rectangle; the pointer tests click those rectangles and the pane can use them for focus.
fn paint_result_section(
    ui: &mut Ui,
    view: &DocumentView,
    width: f32,
    expansion: &mut ExpansionState,
    toggles: &mut Vec<(String, Rect)>,
) {
    ui.set_max_width(width.max(120.0));
    ui.label(
        RichText::new("RESULT · structured projection")
            .small()
            .color(DIM),
    );
    if !view.has_snapshot {
        ui.label(RichText::new("No run has published a result yet.").color(DIM));
        return;
    }
    let projection = ResultProjection::new(
        view.result.clone(),
        view.result_structured.clone(),
    );
    // RES-004: the one place the expansion map is scoped to its result. A repaint of the same
    // committed snapshot returns false and keeps every path; a commit that publishes a different
    // value replaces the map instead of applying paths that name the old tree.
    expansion.bind(&projection);
    match projection.tree() {
        Some(tree) => {
            // The display caption stays one line: the full display is in the Logs pane and on
            // hover. It is painted beside the tree, never read by it.
            let full = projection.display.clone();
            let caption: String = full.chars().take(72).collect();
            let caption = if full.chars().count() > 72 {
                format!("{caption}…")
            } else {
                caption
            };
            ui.label(
                RichText::new(format!("display: {caption}"))
                    .small()
                    .monospace()
                    .color(DIM),
            )
            .on_hover_text(full);
            paint_structured_node(ui, &tree, "result", expansion, toggles);
        }
        None => {
            ui.label(
                RichText::new(
                    "The engine published no structured value for this result; only the display \
                     text is available.",
                )
                .small()
                .color(DIM),
            );
        }
    }
}

/// Paints one collapsible header row and returns true when its node is open after the click.
///
/// The header is an egui button, so a pointer click and Space/Enter on a focused header arrive
/// as the same `clicked()` and go through the same [`ExpansionState::toggle`] call. A click also
/// moves keyboard focus onto the header it changed, so the next Space/Enter lands on the node
/// the user just touched instead of typing into the document.
///
/// The visible text carries the ▸/▾ indicator; the accessible name carries the same label, the
/// node's path and the state word, so a UIA client can tell which node is collapsed.
fn paint_toggle(
    ui: &mut Ui,
    expansion: &mut ExpansionState,
    path: &str,
    label: &str,
    toggles: &mut Vec<(String, Rect)>,
) -> bool {
    let visible = expansion.header_text(path, label);
    let response = ui
        .push_id(path, |ui| {
            ui.add(
                egui::Button::new(RichText::new(visible).monospace().color(ACCENT)).frame(false),
            )
        })
        .inner;
    if response.clicked() {
        expansion.toggle(path);
        ui.memory_mut(|memory| memory.request_focus(response.id));
        // The indicator and the state word are painted from the map, so ask for the frame that
        // renders the new state now that the input has been consumed.
        ui.ctx().request_repaint();
    }
    let accessible = expansion.accessible_name(path, label);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, accessible.clone()));
    let action = if expansion.is_collapsed(path) {
        "expand"
    } else {
        "collapse"
    };
    response.clone().on_hover_text(format!(
        "{path} · {} — click, or press Space/Enter, to {action}",
        expansion.state_word(path)
    ));
    toggles.push((path.to_string(), response.rect));
    !expansion.is_collapsed(path)
}

/// Paints one structured node with its toggle header. Scalars and the absent value have no
/// children, so they stay a single row and are never given a toggle.
fn paint_structured_node(
    ui: &mut Ui,
    node: &StructuredNode,
    path: &str,
    expansion: &mut ExpansionState,
    toggles: &mut Vec<(String, Rect)>,
) {
    match node {
        StructuredNode::Scalar(scalar) => paint_scalar(ui, scalar),
        StructuredNode::Null => paint_absent(ui),
        other => {
            if paint_toggle(ui, expansion, path, &other.headline(), toggles) {
                ui.indent(path, |ui| {
                    paint_node_body(ui, other, path, expansion, toggles);
                });
            }
        }
    }
}

/// Paints a node's children without a header of its own: the caller paints the header (a record
/// field, an array row or a matrix cell already names the node).
fn paint_node_body(
    ui: &mut Ui,
    node: &StructuredNode,
    path: &str,
    expansion: &mut ExpansionState,
    toggles: &mut Vec<(String, Rect)>,
) {
    match node {
        StructuredNode::Scalar(scalar) => paint_scalar(ui, scalar),
        StructuredNode::Null => paint_absent(ui),
        StructuredNode::Symbolic(symbolic) => paint_symbolic_body(ui, symbolic),
        StructuredNode::Array(array) => paint_array_body(ui, array, path, expansion, toggles),
        StructuredNode::Record(record) => paint_record_body(ui, record, path, expansion, toggles),
    }
}

/// One scalar row: the projection kind, the value, then the remaining published fields.
fn paint_scalar(ui: &mut Ui, scalar: &crate::results_tree::ScalarNode) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&scalar.kind).monospace().color(DIM));
        ui.label(RichText::new(&scalar.value).monospace().color(TEXT));
        for (key, value) in &scalar.details {
            ui.label(
                RichText::new(format!("{key}={value}"))
                    .small()
                    .color(DIM),
            );
        }
    });
}

/// The absent value, painted where a node would be.
fn paint_absent(ui: &mut Ui) {
    ui.label(RichText::new("Null - no value").monospace().color(DIM));
}

/// Paints one matrix cell. A scalar (the common case) is compact on the cell's row; a symbolic
/// cell is itself a collapsible node, so its pretty and canonical renderings hide behind the
/// same ▸/▾ header every other node uses.
fn paint_cell(
    ui: &mut Ui,
    node: &StructuredNode,
    path: &str,
    column: usize,
    expansion: &mut ExpansionState,
    toggles: &mut Vec<(String, Rect)>,
) {
    match node {
        StructuredNode::Scalar(scalar) => paint_scalar(ui, scalar),
        StructuredNode::Null => {
            ui.label(RichText::new("Null").monospace().color(DIM));
        }
        other => {
            let label = format!("[{column}] {}", other.headline());
            if paint_toggle(ui, expansion, path, &label, toggles) {
                ui.indent(path, |ui| {
                    paint_node_body(ui, other, path, expansion, toggles);
                });
            }
        }
    }
}

/// Paints a symbolic node's published renderings and metadata.
fn paint_symbolic_body(ui: &mut Ui, symbolic: &crate::results_tree::SymbolicNode) {
    ui.add(
        egui::Label::new(
            RichText::new(format!("pretty     {}", symbolic.pretty))
                .monospace()
                .color(TEXT),
        )
        .wrap(),
    );
    ui.add(
        egui::Label::new(
            RichText::new(format!("canonical  {}", symbolic.canonical))
                .monospace()
                .color(TEXT),
        )
        .wrap(),
    );
    if let Some(domain) = &symbolic.domain {
        ui.label(RichText::new(format!("domain     {domain}")).small().color(DIM));
    }
    if !symbolic.free_symbols.is_empty() {
        ui.label(
            RichText::new(format!("free       {}", symbolic.free_symbols.join(", ")))
                .small()
                .color(DIM),
        );
    }
    if let Some(count) = &symbolic.node_count {
        ui.label(RichText::new(format!("nodes      {count}")).small().color(DIM));
    }
    if symbolic.truncated {
        ui.label(
            RichText::new("this rendering was shortened by a print budget")
                .small()
                .color(WARN),
        );
    }
    for (key, value) in &symbolic.details {
        ui.label(
            RichText::new(format!("{key}={value}"))
                .small()
                .color(DIM),
        );
    }
}

/// Paints an array's body. Every row is a collapsible group: a 2-D array's rows hold the cells
/// laid out as columns; a 1-D array's rows hold one element each. A higher-rank array names its
/// shape and lists its row-major elements exactly like a flat array.
fn paint_array_body(
    ui: &mut Ui,
    array: &ArrayNode,
    path: &str,
    expansion: &mut ExpansionState,
    toggles: &mut Vec<(String, Rect)>,
) {
    let rows = array.rows();
    if array.is_matrix() && !rows.is_empty() {
        let columns = array.shape[1].max(1);
        let cell_width = ((ui.available_width() - 44.0) / columns as f32).max(52.0);
        for (row_index, row) in rows.iter().enumerate() {
            let row_path = element_path(path, row_index);
            let label = format!(
                "[{row_index}] {} cell{}",
                row.len(),
                if row.len() == 1 { "" } else { "s" }
            );
            if paint_toggle(ui, expansion, &row_path, &label, toggles) {
                ui.indent(&row_path, |ui| {
                    egui::Grid::new(format!("{row_path}.cells"))
                        .num_columns(columns)
                        .spacing([6.0, 3.0])
                        .show(ui, |ui| {
                            for (column_index, cell) in row.iter().enumerate() {
                                let cell_path = element_path(&row_path, column_index);
                                ui.allocate_ui_with_layout(
                                    vec2(cell_width, 14.0),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        ui.set_width(cell_width);
                                        paint_cell(
                                            ui,
                                            cell,
                                            &cell_path,
                                            column_index,
                                            expansion,
                                            toggles,
                                        );
                                    },
                                );
                            }
                            ui.end_row();
                        });
                });
            }
        }
        return;
    }
    if array.elements.is_empty() {
        ui.label(RichText::new("empty").small().color(DIM));
        return;
    }
    for (index, element) in array.elements.iter().enumerate() {
        let child_path = element_path(path, index);
        let label = match element {
            StructuredNode::Scalar(_) | StructuredNode::Null => format!("[{index}]"),
            other => format!("[{index}] {}", other.headline()),
        };
        if paint_toggle(ui, expansion, &child_path, &label, toggles) {
            ui.indent(&child_path, |ui| {
                paint_node_body(ui, element, &child_path, expansion, toggles);
            });
        }
    }
}

/// Paints a record body: every published field is one collapsible node. Its header names the
/// field and, when the value is structured, the value's headline; its body is the value. A
/// scalar value is a row inside the field's body, so collapsing a scalar field hides its value
/// without dropping it from the model.
fn paint_record_body(
    ui: &mut Ui,
    record: &crate::results_tree::RecordNode,
    path: &str,
    expansion: &mut ExpansionState,
    toggles: &mut Vec<(String, Rect)>,
) {
    if record.fields.is_empty() {
        ui.label(RichText::new("no fields").small().color(DIM));
        return;
    }
    for field in &record.fields {
        let child_path = field_path(path, &field.name);
        let label = match &field.value {
            StructuredNode::Scalar(_) | StructuredNode::Null => format!("{}:", field.name),
            other => format!("{}: {}", field.name, other.headline()),
        };
        if paint_toggle(ui, expansion, &child_path, &label, toggles) {
            ui.indent(&child_path, |ui| {
                paint_node_body(ui, &field.value, &child_path, expansion, toggles);
            });
        }
    }
}

/// The fixed widths of the variables table's three columns, fitted to the pane.
fn variable_column_widths(available: f32) -> [f32; 3] {
    let usable = (available - 16.0).max(96.0);
    let name = (usable * 0.34).max(40.0);
    let kind = (usable * 0.26).max(34.0);
    let display = (usable - name - kind).max(40.0);
    [name, kind, display]
}

/// The fixed widths of the functions table's three columns, fitted to the pane.
fn function_column_widths(available: f32) -> [f32; 3] {
    let usable = (available - 16.0).max(120.0);
    let name = (usable * 0.36).max(44.0);
    let parameters = (usable * 0.32).max(38.0);
    let origin = (usable - name - parameters).max(44.0);
    [name, parameters, origin]
}

/// One aligned table cell: the (already truncated) text, with the full value as its tooltip.
///
/// The cell allocates a fixed width so every row lines up under its header; `Label::truncate`
/// is the second clip for a value that is still wider than the column (the character cut in
/// [`crate::workspace_table::cell_text`] is the deterministic one the tests pin).
fn table_cell(ui: &mut Ui, width: f32, text: RichText, full: &str) {
    ui.allocate_ui_with_layout(
        vec2(width, 16.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.add(egui::Label::new(text).truncate())
                .on_hover_text(full.to_string());
        },
    );
}

/// The visible origin colour: core builtins are quiet, plugin builtins warn-coloured and user
/// definitions take the accent so the three rows are distinguishable at a glance.
fn origin_color(origin: FunctionOrigin) -> Color32 {
    match origin {
        FunctionOrigin::Builtin => DIM,
        FunctionOrigin::Plugin => WARN,
        FunctionOrigin::User => ACCENT,
    }
}

/// The stable egui id of the one-line quick-eval field.
///
/// The window uses it for the Ctrl+Shift+E focus shortcut and for the focus-lock filter that
/// keeps Up/Down in the field for history navigation; the unit tests use it to assert focus.
pub fn quick_eval_id() -> Id {
    Id::unique("quick_eval_input")
}

/// True when this pass contains a press of `key` carrying no modifiers.
///
/// The key event's own modifier mask is used, not the frame's aggregate state: a modifier that
/// was pressed for the focus shortcut and is released in the same pass must not make a later
/// bare Enter look like Ctrl+Enter, and the event mask is what describes the key press itself.
fn key_pressed_without_modifiers(ui: &Ui, key: Key) -> bool {
    ui.input(|input| {
        input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Key {
                    key: pressed_key,
                    pressed: true,
                    modifiers,
                    ..
                } if *pressed_key == key && !modifiers.any()
            )
        })
    })
}

fn header_ui(header: Rect) -> UiBuilder {
    UiBuilder::new()
        .max_rect(header.shrink2(vec2(8.0, 3.0)))
        .layout(Layout::left_to_right(Align::Center))
}

/// The kind vocabulary, as `label: marker` pairs, for the pane header hover and the empty pane.
fn log_kind_legend() -> String {
    LogKind::ALL
        .iter()
        .map(|kind| format!("{}: {}", kind.label(), kind.marker()))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// One-line summary of a failed run's diagnostic for the editor pane header.
fn diagnostic_summary(diagnostic: &Diagnostic) -> String {
    let message = diagnostic.message.lines().next().unwrap_or_default();
    let mut short: String = message.chars().take(70).collect();
    if message.chars().count() > 70 {
        short.push('…');
    }
    format!(
        "Diagnostic Ln {}, Col {}: {}",
        diagnostic.line, diagnostic.column, short
    )
}

fn with_pane<R>(
    ui: &mut Ui,
    rect: Rect,
    title: &str,
    add: impl FnOnce(&mut Ui, Rect, Rect) -> R,
) -> R {
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(egui::Role::Pane)
            .accessibility_label(title),
        |ui| {
            ui.painter().rect_filled(rect, 4.0, PANE_BG);
            ui.painter()
                .rect_stroke(rect, 4.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
            let header = Rect::from_min_size(rect.min, vec2(rect.width(), 24.0));
            ui.painter().rect_filled(header, 4.0, HEADER_BG);
            let body = Rect::from_min_max(
                pos2(rect.left() + 1.0, header.bottom() + 1.0),
                pos2(rect.right() - 1.0, rect.bottom() - 1.0),
            );
            add(ui, header, body)
        },
    )
    .inner
}

fn splitter(ui: &mut Ui, rect: Rect, salt: &str, horizontal_axis: bool, label: &str) -> f32 {
    let id = Id::unique(salt);
    let response = ui.interact(rect, id, Sense::click_and_drag());
    if response.clicked() || response.drag_started() || response.is_pointer_button_down_on() {
        response.request_focus();
    }
    let focused = response.has_focus();
    let hot = response.hovered() || response.dragged() || focused;
    ui.painter()
        .rect_filled(rect, 0.0, if hot { SPLIT_HOT } else { SPLIT_BG });
    let grip = if horizontal_axis {
        Rect::from_center_size(rect.center(), vec2(2.0, rect.height()))
    } else {
        Rect::from_center_size(rect.center(), vec2(rect.width(), 2.0))
    };
    ui.painter()
        .rect_filled(grip, 1.0, if hot { ACCENT } else { BORDER });

    if hot {
        ui.ctx().set_cursor_icon(if horizontal_axis {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        });
    }

    let mut delta = if response.dragged() {
        if horizontal_axis {
            response.drag_delta().x
        } else {
            response.drag_delta().y
        }
    } else {
        0.0
    };

    if focused {
        let step = if ui.input(|input| input.modifiers.shift) {
            32.0
        } else {
            8.0
        };
        if horizontal_axis {
            if ui.input(|input| input.key_pressed(Key::ArrowRight)) {
                delta += step;
            }
            if ui.input(|input| input.key_pressed(Key::ArrowLeft)) {
                delta -= step;
            }
            if ui.input(|input| input.key_pressed(Key::Home)) {
                delta -= 1000.0;
            }
            if ui.input(|input| input.key_pressed(Key::End)) {
                delta += 1000.0;
            }
        } else {
            if ui.input(|input| input.key_pressed(Key::ArrowDown)) {
                delta += step;
            }
            if ui.input(|input| input.key_pressed(Key::ArrowUp)) {
                delta -= step;
            }
        }
    }

    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Splitter, true, label));
    delta
}

/// Opens the native workbench window and runs it until the user closes it.
pub fn run(initial_path: Option<PathBuf>) -> i32 {
    // Printed at startup so a launch with redirected stderr shows where the engine will write.
    eprintln!(
        "lovelace-desktop: plot directory {}",
        crate::engine::default_plot_dir().display()
    );
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Lovelace Desktop"),
        ..Default::default()
    };
    match eframe::run_native(
        "Lovelace Desktop",
        options,
        Box::new(move |creation| {
            creation.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(WorkbenchApp::new(initial_path)))
        }),
    ) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("lovelace-desktop: could not open the workbench window: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::CompletionKind;
    use crate::engine::{EngineFunction, EngineState, Evaluator};
    use crate::log_kind::LogLine;
    use crate::run_state::RunState;
    use crate::workbench::Workbench;
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Instant;

    /// An engine that sleeps in every evaluation, so a run stays in flight long enough for the
    /// window to paint it and for a shortcut to reach it.
    struct SlowEngine {
        delay: Duration,
    }

    impl Evaluator for SlowEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            std::thread::sleep(self.delay);
            Ok("ok".to_string())
        }

        fn variables(&self) -> Vec<(String, String)> {
            vec![("ran".to_string(), "yes".to_string())]
        }
    }

    fn slow_workbench() -> Workbench {
        Workbench::with_factory(Box::new(|| {
            Box::new(SlowEngine {
                delay: Duration::from_millis(30),
            })
        }))
    }

    /// A gate the worker's engine factory waits on, so the first frame after Ctrl+Enter can
    /// deterministically observe `run Queued` before the worker reports `RunStarted`.
    #[derive(Default)]
    struct Gate {
        open: Mutex<bool>,
        condvar: Condvar,
    }

    impl Gate {
        fn wait(&self) {
            let mut open = self.open.lock().expect("gate");
            while !*open {
                open = self.condvar.wait(open).expect("gate wait");
            }
        }

        fn release(&self) {
            let mut open = self.open.lock().expect("gate");
            *open = true;
            self.condvar.notify_all();
        }
    }

    fn gated_workbench(gate: Arc<Gate>) -> Workbench {
        // `with_factory` calls the factory once to learn the engine name; use the named variant
        // so the gate blocks the run worker only, not the test thread.
        Workbench::with_named_factory(
            Box::new(move || {
                gate.wait();
                Box::new(SlowEngine {
                    delay: Duration::from_millis(200),
                })
            }),
            "gated-slow",
        )
    }

    fn key(key: Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn frame(ctx: &egui::Context, app: &mut WorkbenchApp, events: Vec<egui::Event>) {
        frame_sized(ctx, app, vec2(1280.0, 800.0), events);
    }

    /// One headless pass of the whole window at an explicit size. The window-proof layout uses
    /// 2500x1380, the same frame the round-27 captures were taken in, so the Workspace pane lays
    /// the result tree out exactly where the pointer tests click it.
    fn frame_sized(
        ctx: &egui::Context,
        app: &mut WorkbenchApp,
        screen: egui::Vec2,
        events: Vec<egui::Event>,
    ) {
        let _ = frame_capturing(ctx, app, screen, events, Default::default());
    }

    /// One headless pass with an explicit viewport map, returning everything the integration would
    /// apply (DOC-003's close commands are read from the output).
    fn frame_capturing(
        ctx: &egui::Context,
        app: &mut WorkbenchApp,
        screen: egui::Vec2,
        events: Vec<egui::Event>,
        viewports: egui::ViewportIdMap<egui::ViewportInfo>,
    ) -> egui::FullOutput {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
            events,
            ..Default::default()
        };
        if !viewports.is_empty() {
            input.viewports = viewports;
        }
        let mut output = ctx.run_ui(input, |ui| app.ui_root(ui));
        // A headless pass has no painter to upload the font texture to; mark the deltas handled.
        output.textures_delta.clear();
        output
    }

    /// Delivers one OS close request (the title-bar X) to the window, returning the frame output.
    fn close_request(ctx: &egui::Context, app: &mut WorkbenchApp) -> egui::FullOutput {
        let mut viewports = egui::ViewportIdMap::default();
        viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                events: vec![egui::ViewportEvent::Close],
                ..Default::default()
            },
        );
        frame_capturing(ctx, app, vec2(1280.0, 800.0), Vec::new(), viewports)
    }

    /// True when the frame asked the integration to cancel a close (DOC-003's refusal).
    fn cancelled_close(output: &egui::FullOutput) -> bool {
        output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|viewport| {
                viewport
                    .commands
                    .iter()
                    .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
            })
            .unwrap_or(false)
    }

    /// True when the frame asked the integration to close the window (DOC-003's approval).
    fn approved_close(output: &egui::FullOutput) -> bool {
        output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|viewport| {
                viewport
                    .commands
                    .iter()
                    .any(|command| matches!(command, egui::ViewportCommand::Close))
            })
            .unwrap_or(false)
    }

    /// The center of the prompt button the test wants to click.
    fn prompt_button(app: &WorkbenchApp, choice: UnsavedAnswer) -> egui::Pos2 {
        app.prompt_buttons
            .iter()
            .find(|(painted, _)| *painted == choice)
            .map(|(_, rect)| rect.center())
            .unwrap_or_else(|| panic!("the {choice:?} button was not painted"))
    }

    /// The buttons the open prompt painted, in their on-screen order.
    fn prompt_choices(app: &WorkbenchApp) -> Vec<&'static str> {
        app.prompt_buttons
            .iter()
            .map(|(choice, _)| choice.label())
            .collect()
    }

    /// A full pointer click at `pos`: move, press, release (one pass each), the same sequence a
    /// user's mouse produces.
    fn pointer_click(ctx: &egui::Context, app: &mut WorkbenchApp, pos: egui::Pos2) {
        frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(
                ctx,
                app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                }],
            );
        }
    }

    #[test]
    fn focus_starts_in_the_editor_and_typed_text_lands_without_a_click() {
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
        let ctx = egui::Context::default();

        frame(&ctx, &mut app, Vec::new());
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(surface_id()),
            "the editor must own keyboard focus on the first frame"
        );

        let lines_before = app.editors[0].line_count();
        frame(&ctx, &mut app, vec![egui::Event::Text("a = 1".to_string())]);

        assert!(
            app.workbench.documents()[0].source.starts_with("a = 1"),
            "typed text did not reach the document: {:?}",
            &app.workbench.documents()[0].source
                [..20.min(app.workbench.documents()[0].source.len())]
        );
        assert_eq!(
            app.editors[0].line_count(),
            lines_before,
            "plain text typed into the editor must not change the line count"
        );
    }

    /// Builds an app whose editor holds `source`, with the caret placed at `(row, col)`.
    fn app_with_source(source: &str, row: usize, col: usize) -> WorkbenchApp {
        let mut workbench = slow_workbench();
        workbench.set_source(0, source.to_string());
        let mut app = WorkbenchApp::with_workbench(workbench);
        app.editors[0].set_caret(row, col);
        app
    }

    #[test]
    fn backspace_that_joins_two_lines_keeps_the_paint_pass_in_bounds() {
        // DEF-001: pressing Backspace at the start of line 2 joins it with line 1. The line
        // count shrank, and the paint loop used to index `lines[1]` of a one-line buffer.
        let mut app = app_with_source("a\nb", 1, 0);
        let ctx = egui::Context::default();

        frame(&ctx, &mut app, Vec::new()); // gives the editor keyboard focus
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Backspace, egui::Modifiers::default())],
        );

        assert_eq!(
            app.editors[0].source(),
            "ab",
            "Backspace at the start of line 2 must join it with line 1"
        );
        assert_eq!(
            app.editors[0].line_col_1based(),
            (1, 2),
            "the caret must sit after the joined text"
        );
        assert_eq!(
            app.workbench.documents()[0].source,
            "ab",
            "the joined buffer must be the document's committed source"
        );
    }

    #[test]
    fn arrow_up_moves_the_caret_without_surrendering_editor_focus() {
        // DEF-002: egui's cardinal focus navigation moves focus on a bare Up arrow before the
        // editor can consume it; the editor must lock the arrow keys while it has focus.
        let mut app = app_with_source("a\nb", 1, 0);
        let ctx = egui::Context::default();

        frame(&ctx, &mut app, Vec::new()); // gives the editor keyboard focus
        frame(&ctx, &mut app, Vec::new()); // lets the focus lock filter install
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowUp, egui::Modifiers::default())],
        );

        assert_eq!(
            app.editors[0].line_col_1based(),
            (1, 1),
            "Up must move the caret to line 1"
        );
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(surface_id()),
            "Up must not move keyboard focus out of the editor"
        );

        frame(&ctx, &mut app, vec![egui::Event::Text("c".to_string())]);
        assert_eq!(
            app.editors[0].source(),
            "ca\nb",
            "typing immediately after Up must land in the editor"
        );
    }

    /// Builds an app whose editor holds `source` and whose engine is the deterministic fake.
    fn fake_app_with_source(source: &str) -> WorkbenchApp {
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::FakeEngine::new())));
        workbench.set_source(0, source.to_string());
        WorkbenchApp::with_workbench(workbench)
    }

    #[test]
    fn failed_run_decorates_the_reported_line_and_f8_jumps_to_it_and_an_edit_clears_it() {
        let mut app = fake_app_with_source("alpha = 1\n1 / 0\ngamma = 3\n");
        let ctx = egui::Context::default();
        assert!(app.workbench.run_active());
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(app.workbench.documents()[0].state(), RunState::Failed);

        frame(&ctx, &mut app, Vec::new());
        let diagnostic = app.editors[0]
            .diagnostic
            .as_ref()
            .expect("the failed run's diagnostic must reach the editor");
        assert_eq!((diagnostic.line, diagnostic.column), (2, 1));
        let range = app.editors[0]
            .diagnostic_range()
            .expect("the offending line is in the buffer");
        assert_eq!(
            (range.line, range.start_char, range.end_char),
            (2, 0, 5),
            "the underline covers line 2 from the reported column to end-of-line"
        );
        assert_eq!(
            (range.start_byte, range.end_byte),
            (0, 5),
            "the range is byte-exact for the ASCII line"
        );

        // F8 is the keyboard path to the failure: caret on the exact 1-based line and column.
        app.editors[0].set_caret(0, 0);
        frame(
            &ctx,
            &mut app,
            vec![key(Key::F8, egui::Modifiers::default())],
        );
        assert_eq!(
            app.editors[0].line_col_1based(),
            (2, 1),
            "F8 must jump to the reported position"
        );

        // Delete the line break before the offending line: the buffer changes and the old
        // diagnostic must not be painted over the joined text.
        app.editors[0].set_caret(1, 0);
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Backspace, egui::Modifiers::default())],
        );
        assert!(
            app.editors[0].diagnostic.is_none(),
            "an edit clears the decoration in the same pass"
        );
        assert!(
            app.workbench.diagnostic(0).is_none(),
            "the workbench also hides a diagnostic whose revision is no longer current"
        );
    }

    /// RUN-003 through the window: after a four-statement run, the view the frame renders is
    /// the workbench's one assembled object and its variables, revision and log tail agree.
    #[test]
    fn a_frame_renders_the_same_assembled_view_the_workbench_published() {
        let mut app = fake_app_with_source("a = 1\nb = a + 1\nprint(\"p\")\nb\n");
        let ctx = egui::Context::default();
        assert!(app.workbench.run_active());
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
        frame(&ctx, &mut app, Vec::new());

        let view = app.workbench.active_view();
        assert_eq!(
            app.view, view,
            "the frame must render the workbench's assembled view, not a second assembly"
        );
        assert_eq!(view.revision, crate::execution::SourceRevision(1));
        assert_eq!(view.committed_steps, 4);
        assert_eq!(
            view.variables,
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
            ]
        );
        assert_eq!(view.last_committed_log_statement(), Some(4));
        assert_eq!(view.last_log_statement(), Some(4));
    }

    /// WKS-001/WKS-002 through the window: both table headers are real clickable controls. The
    /// first click on a column sorts ascending, the second descending, and the rows the window
    /// paints are the committed snapshot's rows (so the header counts are the snapshot counts).
    #[test]
    fn workspace_header_clicks_sort_both_tables_and_the_counts_are_the_snapshot_counts() {
        let mut app = fake_app_with_source("c = 3\na = 1\nb = 2\n");
        let ctx = egui::Context::default();
        assert!(app.workbench.run_active());
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
        frame(&ctx, &mut app, Vec::new());

        let variable_names = |app: &WorkbenchApp| -> Vec<String> {
            app.variables_table
                .ordered()
                .into_iter()
                .map(|index| app.variables_table.rows[index].name.clone())
                .collect()
        };
        let function_names = |app: &WorkbenchApp| -> Vec<String> {
            app.functions_table
                .ordered()
                .into_iter()
                .map(|index| app.functions_table.rows[index].name.clone())
                .collect()
        };

        // Default: ordinal name ascending for both tables, and the painted rows are exactly the
        // committed snapshot's rows.
        assert_eq!(variable_names(&app), vec!["a", "b", "c"]);
        assert!(app.variables_table.sort.ascending);
        assert!(app.functions_table.sort.ascending);
        assert_eq!(
            app.variables_table.rows.len(),
            app.view.state.variables.len(),
            "the variables table must show every captured variable"
        );
        assert_eq!(
            app.functions_table.rows.len(),
            app.view.functions.len(),
            "the displayed function count must equal the snapshot's function count"
        );
        assert_eq!(
            app.functions_table.rows.len(),
            app.view.state.functions.len(),
            "and that count must be the registry capture the engine published"
        );

        // First click on Name keeps the default ascending order; the second click descends.
        let name_header = app.variable_header_rects[VariableColumn::Name.index()]
            .expect("the variables Name header was painted");
        pointer_click(&ctx, &mut app, name_header.center());
        assert!(
            app.variables_table.sort.ascending,
            "the first header click must sort ascending"
        );
        let name_header = app.variable_header_rects[VariableColumn::Name.index()]
            .expect("the Name header stays painted");
        pointer_click(&ctx, &mut app, name_header.center());
        assert!(!app.variables_table.sort.ascending);
        assert_eq!(variable_names(&app), vec!["c", "b", "a"]);

        // A new column starts ascending again.
        let kind_header = app.variable_header_rects[VariableColumn::Kind.index()]
            .expect("the variables Kind header was painted");
        pointer_click(&ctx, &mut app, kind_header.center());
        assert_eq!(app.variables_table.sort.key, VariableColumn::Kind);
        assert!(app.variables_table.sort.ascending);

        // Functions obey the same click contract: the first click on Name is ascending, the
        // second reverses the whole registry slice without losing a row.
        let function_header = app.function_header_rects[FunctionColumn::Name.index()]
            .expect("the functions Name header was painted");
        pointer_click(&ctx, &mut app, function_header.center());
        assert!(app.functions_table.sort.ascending);
        let ascending = function_names(&app);
        assert_eq!(ascending.len(), app.view.state.functions.len());
        let function_header = app.function_header_rects[FunctionColumn::Name.index()]
            .expect("the Name header stays painted");
        pointer_click(&ctx, &mut app, function_header.center());
        assert!(!app.functions_table.sort.ascending);
        let mut reversed = ascending.clone();
        reversed.reverse();
        assert_eq!(
            function_names(&app),
            reversed,
            "descending reverses every row"
        );
        assert_eq!(app.functions_table.rows.len(), ascending.len());
    }

    #[test]
    fn ctrl_enter_runs_and_escape_cancels_with_the_visible_running_state() {
        let gate = Arc::new(Gate::default());
        let mut app = WorkbenchApp::with_workbench(gated_workbench(Arc::clone(&gate)));
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());

        let source_before = app.workbench.documents()[0].source.clone();
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(ctrl), key(Key::Enter, ctrl)],
        );

        assert!(
            app.workbench.active_is_running(),
            "Ctrl+Enter did not start a run"
        );
        assert_eq!(
            app.workbench.documents()[0].state(),
            RunState::Queued,
            "the frame that pressed Ctrl+Enter must submit and show Queued"
        );
        assert!(
            app.workbench.status_text(1, 1).contains("run Queued"),
            "status strip after submit was {:?}",
            app.workbench.status_text(1, 1)
        );
        assert_eq!(
            app.workbench.documents()[0].source,
            source_before,
            "Ctrl+Enter must not also insert a newline into the document"
        );
        let toolbar = app.workbench.toolbar_state();
        assert!(!toolbar.run_enabled, "Run must be disabled while queued");
        assert!(
            toolbar.cancel_visible,
            "Cancel must be visible while queued"
        );

        // Release the worker and let the window apply the typed RunStarted event.
        gate.release();
        let deadline = Instant::now() + Duration::from_secs(2);
        while app.workbench.documents()[0].state() == RunState::Queued {
            app.workbench.poll();
            assert!(
                Instant::now() < deadline,
                "the worker never reported RunStarted"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            app.workbench.documents()[0].state(),
            RunState::Running,
            "RunStarted must move Queued to Running"
        );
        frame(&ctx, &mut app, Vec::new());
        assert!(
            app.workbench.status_text(1, 1).contains("run Running"),
            "status strip was {:?}",
            app.workbench.status_text(1, 1)
        );

        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::default()),
                key(Key::Escape, egui::Modifiers::default()),
            ],
        );
        assert_eq!(
            app.workbench.documents()[0].state(),
            RunState::Cancelling,
            "Esc must move the visible state to Cancelling immediately"
        );
        assert!(
            app.workbench.status_text(1, 1).contains("run Cancelling"),
            "status strip after Esc was {:?}",
            app.workbench.status_text(1, 1)
        );
        assert!(
            app.workbench.wait_until_idle(Duration::from_secs(2)),
            "the run did not reach a terminal state after Esc"
        );
        assert_eq!(app.workbench.documents()[0].state(), RunState::Cancelled);
        assert!(
            app.workbench.status_text(1, 1).contains("run Cancelled"),
            "status strip was {:?}",
            app.workbench.status_text(1, 1)
        );
        assert_eq!(
            app.workbench.documents()[0]
                .transitions()
                .iter()
                .map(|transition| transition.to)
                .collect::<Vec<_>>(),
            [
                RunState::Queued,
                RunState::Running,
                RunState::Cancelling,
                RunState::Cancelled
            ],
            "the window's own frame loop must log the whole visible sequence"
        );
        let toolbar = app.workbench.toolbar_state();
        assert!(toolbar.run_enabled, "Run must return after the run stops");
        assert!(!toolbar.cancel_visible, "Cancel must leave with the run");
    }

    fn builtin(name: &str, parameters: &[&str]) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin: true,
            plugin: None,
            summary: None,
        }
    }

    fn sqrt_function() -> EngineFunction {
        builtin("sqrt", &["x"])
    }

    /// An app whose editor holds `source`, with the caret at `(row, col)` and a registry that
    /// only contains the functions the test names.
    fn completion_app(
        source: &str,
        row: usize,
        col: usize,
        functions: Vec<EngineFunction>,
    ) -> WorkbenchApp {
        let mut workbench = slow_workbench();
        workbench.set_engine_state(EngineState {
            variables: Vec::new(),
            functions,
        });
        workbench.set_source(0, source.to_string());
        let mut app = WorkbenchApp::with_workbench(workbench);
        app.editors[0].set_caret(row, col);
        app
    }

    #[test]
    fn typing_sq_opens_completion_with_the_engine_function_and_enter_replaces_only_the_prefix() {
        let mut app = completion_app("", 0, 0, vec![sqrt_function()]);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // first frame: focus the editor
        frame(&ctx, &mut app, vec![egui::Event::Text("sq".to_string())]);

        let session = &app.editors[0].completion;
        assert!(session.is_open(), "typing `sq` must open the popup");
        assert_eq!(session.items().len(), 1);
        let item = session.selected().expect("a highlighted entry");
        assert_eq!(item.label, "sqrt");
        assert_eq!(
            item.detail, "sqrt(x)",
            "the signature comes from the engine's own function snapshot"
        );
        assert_eq!(item.kind, CompletionKind::Builtin);
        assert_eq!(
            app.editors[0].completion_summary().as_deref(),
            Some("Completion: sq -> sqrt(x) [builtin] 1/1")
        );
        assert_eq!(app.workbench.documents()[0].source, "sq");

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert_eq!(
            app.editors[0].source(),
            "sqrt",
            "Enter must replace exactly the typed prefix"
        );
        assert_eq!(
            app.editors[0].line_col_1based(),
            (1, 5),
            "the caret must sit after the inserted text"
        );
        assert!(!app.editors[0].completion.is_open());
        assert_eq!(app.workbench.documents()[0].source, "sqrt");
    }

    #[test]
    fn arrow_keys_move_the_completion_selection_and_tab_accepts_it() {
        let mut app = completion_app(
            "",
            0,
            0,
            vec![builtin("sq", &["x"]), builtin("sqrt", &["x"])],
        );
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());
        frame(&ctx, &mut app, vec![egui::Event::Text("sq".to_string())]);
        assert_eq!(
            app.editors[0]
                .completion
                .items()
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["sq", "sqrt"],
            "the exact label is offered before the longer prefix match"
        );
        assert_eq!(app.editors[0].completion.selected_index(), 0);

        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowDown, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].completion.selected_index(), 1);
        assert_eq!(app.editors[0].source(), "sq", "navigation must not edit");
        assert_eq!(app.editors[0].line_col_1based(), (1, 3));

        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowUp, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].completion.selected_index(), 0);
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowUp, egui::Modifiers::default())],
        );
        assert_eq!(
            app.editors[0].completion.selected_index(),
            0,
            "Up on the first entry stays on it"
        );
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowDown, egui::Modifiers::default())],
        );
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Tab, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].source(), "sqrt", "Tab accepts the selection");
        assert_eq!(app.editors[0].line_col_1based(), (1, 5));
        assert!(!app.editors[0].completion.is_open());
    }

    #[test]
    fn escape_dismisses_completion_without_changing_the_buffer_or_accepting() {
        let mut app = completion_app("", 0, 0, vec![sqrt_function()]);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());
        frame(&ctx, &mut app, vec![egui::Event::Text("sq".to_string())]);
        assert!(app.editors[0].completion.is_open());

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Escape, egui::Modifiers::default())],
        );
        assert!(!app.editors[0].completion.is_open(), "Esc must dismiss");
        assert_eq!(
            app.editors[0].source(),
            "sq",
            "Esc must not edit the buffer"
        );
        assert_eq!(app.editors[0].line_col_1based(), (1, 3));
        assert_eq!(app.workbench.documents()[0].source, "sq");

        // A dismissed popup must not accept anything: Enter is an ordinary newline again.
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].source(), "sq\n");
        assert_eq!(app.editors[0].line_col_1based(), (2, 1));
    }

    #[test]
    fn ctrl_space_opens_the_whole_catalogue_and_strings_and_comments_suppress_it() {
        let mut app = completion_app("", 0, 0, vec![sqrt_function()]);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(ctrl), key(Key::Space, ctrl)],
        );
        assert!(
            app.editors[0].completion.is_open(),
            "Ctrl+Space must open the popup even with no typed prefix"
        );
        assert_eq!(
            app.editors[0]
                .completion
                .selected()
                .map(|item| item.label.as_str()),
            Some("break"),
            "with an empty prefix the catalogue starts at the first keyword in name order"
        );
        assert!(
            app.editors[0]
                .completion
                .items()
                .iter()
                .any(|item| item.label == "sqrt" && item.detail == "sqrt(x)"),
            "the whole catalogue includes the engine's registered functions"
        );
        assert_eq!(
            app.editors[0].source(),
            "",
            "Ctrl+Space must not insert a space"
        );

        // Inside a string literal the popup stays closed.
        let mut quoted = completion_app("", 0, 0, vec![sqrt_function()]);
        let ctx = egui::Context::default();
        frame(&ctx, &mut quoted, Vec::new());
        frame(
            &ctx,
            &mut quoted,
            vec![egui::Event::Text("\"sq".to_string())],
        );
        assert!(
            !quoted.editors[0].completion.is_open(),
            "a string literal must suppress completion"
        );

        // Inside a comment the popup stays closed too.
        let mut commented = completion_app("", 0, 0, vec![sqrt_function()]);
        let ctx = egui::Context::default();
        frame(&ctx, &mut commented, Vec::new());
        frame(
            &ctx,
            &mut commented,
            vec![egui::Event::Text("# sq".to_string())],
        );
        assert!(
            !commented.editors[0].completion.is_open(),
            "a comment must suppress completion"
        );
    }

    fn ctrl_modifiers() -> egui::Modifiers {
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        }
    }

    fn ctrl_z() -> egui::Event {
        key(Key::Z, ctrl_modifiers())
    }

    fn ctrl_y() -> egui::Event {
        key(Key::Y, ctrl_modifiers())
    }

    fn ctrl_shift_z() -> egui::Event {
        key(
            Key::Z,
            egui::Modifiers {
                shift: true,
                ..ctrl_modifiers()
            },
        )
    }

    #[test]
    fn ctrl_z_undoes_a_typing_burst_and_ctrl_y_redoes_it() {
        let mut app = app_with_source("", 0, 0);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // focus the editor

        // Three character events, one per pass: they are one burst because nothing else
        // happened between them and the pause is far below the coalescing gap.
        frame(&ctx, &mut app, vec![egui::Event::Text("a".to_string())]);
        frame(&ctx, &mut app, vec![egui::Event::Text("b".to_string())]);
        frame(&ctx, &mut app, vec![egui::Event::Text("c".to_string())]);
        assert_eq!(app.editors[0].source(), "abc");
        assert_eq!(app.editors[0].line_col_1based(), (1, 4));

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(
            app.editors[0].source(),
            "",
            "Ctrl+Z restores the empty buffer"
        );
        assert_eq!(
            app.editors[0].line_col_1based(),
            (1, 1),
            "with the caret at 0"
        );
        assert_eq!(
            app.workbench.documents()[0].source,
            "",
            "the document must be committed from the undone buffer"
        );

        frame(&ctx, &mut app, vec![ctrl_y()]);
        assert_eq!(app.editors[0].source(), "abc", "Ctrl+Y restores the burst");
        assert_eq!(app.editors[0].line_col_1based(), (1, 4));
        assert_eq!(app.workbench.documents()[0].source, "abc");

        // Ctrl+Shift+Z is the second redo spelling.
        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.editors[0].source(), "");
        frame(&ctx, &mut app, vec![ctrl_shift_z()]);
        assert_eq!(app.editors[0].source(), "abc");
    }

    #[test]
    fn a_letter_key_event_does_not_split_a_typing_burst_but_an_arrow_does() {
        let mut app = app_with_source("", 0, 0);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // focus the editor

        // A real backend delivers a Key event and the character it produced. The Key event for
        // a letter is not a non-typing action, so it must not split the burst; only the Text
        // events are the edits.
        frame(
            &ctx,
            &mut app,
            vec![
                key(Key::A, egui::Modifiers::default()),
                egui::Event::Text("a".to_string()),
            ],
        );
        frame(
            &ctx,
            &mut app,
            vec![
                key(Key::B, egui::Modifiers::default()),
                egui::Event::Text("b".to_string()),
            ],
        );
        assert_eq!(app.editors[0].source(), "ab");
        assert_eq!(
            app.editors[0].history.undo_steps(),
            1,
            "letter Key events must not split a typing burst"
        );

        // A caret move is a non-typing action and does start a new step.
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowLeft, egui::Modifiers::default())],
        );
        frame(
            &ctx,
            &mut app,
            vec![
                key(Key::X, egui::Modifiers::default()),
                egui::Event::Text("X".to_string()),
            ],
        );
        assert_eq!(app.editors[0].source(), "aXb");
        assert_eq!(app.editors[0].history.undo_steps(), 2);

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.editors[0].source(), "ab");
        assert_eq!(app.editors[0].line_col_1based(), (1, 2));
        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.editors[0].source(), "");
    }

    #[test]
    fn ctrl_z_restores_a_deleted_selection() {
        let mut app = app_with_source("abcdef", 0, 1);
        app.editors[0].select_to(0, 4);
        assert_eq!(app.editors[0].selected_text().as_deref(), Some("bcd"));
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // focus the editor

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Delete, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].source(), "aef");
        assert_eq!(app.editors[0].line_col_1based(), (1, 2));
        assert_eq!(app.workbench.documents()[0].source, "aef");

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.editors[0].source(), "abcdef");
        assert_eq!(
            app.editors[0].selection_range(),
            Some(((0, 1), (0, 4))),
            "the undo restores the selection, not only the deleted text"
        );
        assert_eq!(app.editors[0].selected_text().as_deref(), Some("bcd"));
        assert_eq!(app.workbench.documents()[0].source, "abcdef");
    }

    #[test]
    fn ctrl_z_restores_a_backspace_line_join() {
        let mut app = app_with_source("a\nb", 1, 0);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // focus the editor

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Backspace, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].source(), "ab");
        assert_eq!(app.workbench.documents()[0].source, "ab");

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.editors[0].source(), "a\nb");
        assert_eq!(
            app.editors[0].line_col_1based(),
            (2, 1),
            "the caret is back at the start of line 2"
        );
        assert_eq!(app.workbench.documents()[0].source, "a\nb");
    }

    #[test]
    fn undo_in_one_document_never_changes_the_other() {
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::FakeEngine::new())));
        workbench.set_source(0, String::new());
        workbench.set_source(1, String::new());
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // focus document A

        frame(&ctx, &mut app, vec![egui::Event::Text("abc".to_string())]);
        assert_eq!(app.workbench.documents()[0].source, "abc");

        // Switch to document B: its editor has its own history.
        app.workbench.set_active(1);
        frame(&ctx, &mut app, Vec::new());
        frame(&ctx, &mut app, vec![egui::Event::Text("xy".to_string())]);
        assert_eq!(app.workbench.documents()[1].source, "xy");
        assert_eq!(app.workbench.documents()[0].source, "abc");

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.workbench.documents()[1].source, "");
        assert_eq!(
            app.workbench.documents()[0].source,
            "abc",
            "an undo in B must not change A"
        );

        // Switch back to A: its own history still holds its typing burst.
        app.workbench.set_active(0);
        frame(&ctx, &mut app, Vec::new());
        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(app.workbench.documents()[0].source, "");
        assert_eq!(app.workbench.documents()[1].source, "");
    }

    #[test]
    fn a_completion_insertion_is_its_own_undo_step_in_the_window() {
        let mut app = completion_app("", 0, 0, vec![sqrt_function()]);
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // focus the editor
        frame(&ctx, &mut app, vec![egui::Event::Text("sq".to_string())]);
        assert!(app.editors[0].completion.is_open());

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert_eq!(app.editors[0].source(), "sqrt");
        assert_eq!(app.editors[0].line_col_1based(), (1, 5));

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(
            app.editors[0].source(),
            "sq",
            "one undo removes only the completion insertion"
        );
        assert_eq!(app.editors[0].line_col_1based(), (1, 3));
        assert!(!app.editors[0].completion.is_open());
        assert_eq!(app.workbench.documents()[0].source, "sq");

        frame(&ctx, &mut app, vec![ctrl_z()]);
        assert_eq!(
            app.editors[0].source(),
            "",
            "the typing that produced the prefix is the next step"
        );
        assert_eq!(app.workbench.documents()[0].source, "");
    }

    fn ctrl_shift_modifiers() -> egui::Modifiers {
        egui::Modifiers {
            ctrl: true,
            shift: true,
            command: true,
            ..Default::default()
        }
    }

    /// RUN-002 / LOG-003: Ctrl+Shift+E puts keyboard focus in the one-line field, typing there
    /// does not touch the editor buffer, and Enter submits the expression, clears the field only
    /// after the request was accepted, and leaves the document's source and revision unchanged.
    #[test]
    fn quick_eval_shortcut_focuses_the_field_and_enter_runs_without_editing_the_buffer() {
        let mut app = fake_app_with_source("a = 1\n");
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new()); // editor focus on the first frame
        let source_before = app.workbench.documents()[0].source.clone();
        let revision_before = app.workbench.documents()[0].revision;

        let shortcut = ctrl_shift_modifiers();
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::ModifiersChanged(shortcut),
                key(Key::E, shortcut),
            ],
        );
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(quick_eval_id()),
            "Ctrl+Shift+E must focus the quick-eval field"
        );
        // Release the shortcut's modifiers before using Enter, as the real backend does.
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::default())],
        );

        frame(&ctx, &mut app, vec![egui::Event::Text("6 * 7".to_string())]);
        assert_eq!(app.quick_eval.text(), "6 * 7");
        assert_eq!(
            app.editors[0].source(),
            "a = 1\n",
            "typing in quick eval must not reach the editor"
        );
        assert_eq!(app.workbench.documents()[0].source, source_before);

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert_eq!(
            app.quick_eval.text(),
            "",
            "an accepted submission clears the field"
        );
        assert!(
            app.workbench.active_is_running(),
            "Enter in the field must submit a run"
        );
        assert_eq!(
            app.workbench.documents()[0].source, source_before,
            "quick eval must not change the document buffer"
        );
        assert_eq!(
            app.workbench.documents()[0].revision, revision_before,
            "quick eval must not bump the document revision"
        );
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
        assert_eq!(app.workbench.documents()[0].state(), RunState::Finished);

        let document = &app.workbench.documents()[0];
        let echoes: Vec<&LogLine> = document
            .logs
            .iter()
            .filter(|line| line.kind == LogKind::Echo)
            .collect();
        assert_eq!(echoes.len(), 1, "exactly one echo line per submission");
        assert_eq!(echoes[0].render(), "[echo] >> 6 * 7");
        assert!(document
            .logs
            .iter()
            .any(|line| line.kind == LogKind::Result && line.text == "= 42"));
    }

    /// LOG-003: Up/Down on the focused field walk accepted submissions newest first and stop at
    /// the oldest; the restored text really lands back in the field.
    #[test]
    fn quick_eval_history_returns_previous_expressions_newest_first_and_stops_at_the_oldest() {
        let mut app = fake_app_with_source("a = 1\n");
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());

        let shortcut = ctrl_shift_modifiers();
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::ModifiersChanged(shortcut),
                key(Key::E, shortcut),
            ],
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::default())],
        );
        frame(&ctx, &mut app, vec![egui::Event::Text("6 * 7".to_string())]);
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
        frame(&ctx, &mut app, Vec::new()); // repaint the cleared field and install its key lock

        frame(&ctx, &mut app, vec![egui::Event::Text("2 + 3".to_string())]);
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
        frame(&ctx, &mut app, Vec::new());
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(quick_eval_id()),
            "the field keeps focus after a submission"
        );

        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowUp, egui::Modifiers::default())],
        );
        assert_eq!(app.quick_eval.text(), "2 + 3", "the first Up is the newest");
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowUp, egui::Modifiers::default())],
        );
        assert_eq!(app.quick_eval.text(), "6 * 7");
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowUp, egui::Modifiers::default())],
        );
        assert_eq!(
            app.quick_eval.text(),
            "6 * 7",
            "Up at the oldest keeps the oldest expression"
        );
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ArrowDown, egui::Modifiers::default())],
        );
        assert_eq!(app.quick_eval.text(), "2 + 3");
        assert_eq!(
            app.editors[0].source(),
            "a = 1\n",
            "history navigation must not edit the editor"
        );
    }

    /// RUN-002: the field is also focusable by pointer, not only by its shortcut.
    #[test]
    fn the_quick_eval_field_takes_keyboard_focus_from_a_mouse_click() {
        let mut app = fake_app_with_source("a = 1\n");
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());
        let rect = app
            .quick_eval_rect
            .expect("the quick-eval field was painted on the first frame");
        let pos = rect.center();
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                }],
            );
        }
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(quick_eval_id()),
            "clicking the field must give it keyboard focus"
        );
    }

    /// LOG-003: Esc leaves the quick-eval field without submitting, without clearing its text
    /// and without starting a run.
    #[test]
    fn escape_leaves_the_quick_eval_field_without_submitting_or_clearing_it() {
        let mut app = fake_app_with_source("a = 1\n");
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());

        let shortcut = ctrl_shift_modifiers();
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::ModifiersChanged(shortcut),
                key(Key::E, shortcut),
            ],
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::default())],
        );
        frame(&ctx, &mut app, vec![egui::Event::Text("6 * 7".to_string())]);
        assert_eq!(app.quick_eval.text(), "6 * 7");

        frame(
            &ctx,
            &mut app,
            vec![key(Key::Escape, egui::Modifiers::default())],
        );

        assert_eq!(
            app.quick_eval.text(),
            "6 * 7",
            "Esc must leave the text in the field"
        );
        assert_ne!(
            ctx.memory(|memory| memory.focused()),
            Some(quick_eval_id()),
            "Esc must clear focus from the field"
        );
        assert_eq!(
            app.workbench.documents()[0].state(),
            RunState::Idle,
            "Esc in the field must not start a run"
        );
        assert!(
            !app.workbench.active_logs().iter().any(|line| line.kind == LogKind::Echo),
            "Esc must not echo or submit"
        );
        assert_eq!(app.editors[0].source(), "a = 1\n");
    }

    /// LOG-003: when the host refuses (a run is in flight), the typed expression stays in the
    /// field, no echo line is faked, and the `[info]` advisory is logged.
    #[test]
    fn a_refused_quick_eval_keeps_the_typed_text_and_logs_info_without_an_echo() {
        let gate = Arc::new(Gate::default());
        let mut app = WorkbenchApp::with_workbench(gated_workbench(Arc::clone(&gate)));
        let ctx = egui::Context::default();
        frame(&ctx, &mut app, Vec::new());

        let ctrl = ctrl_modifiers();
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(ctrl), key(Key::Enter, ctrl)],
        );
        assert_eq!(
            app.workbench.documents()[0].state(),
            RunState::Queued,
            "the document run must be in flight"
        );

        let shortcut = ctrl_shift_modifiers();
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::ModifiersChanged(shortcut),
                key(Key::E, shortcut),
            ],
        );
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(quick_eval_id())
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::ModifiersChanged(egui::Modifiers::default())],
        );
        frame(&ctx, &mut app, vec![egui::Event::Text("6 * 7".to_string())]);
        frame(
            &ctx,
            &mut app,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );

        assert_eq!(
            app.quick_eval.text(),
            "6 * 7",
            "a refused submission must keep the input text"
        );
        let logs = app.workbench.active_logs();
        assert!(
            !logs.iter().any(|line| line.kind == LogKind::Echo),
            "a refused submission must not fake an echo line"
        );
        assert!(
            logs.iter().any(|line| {
                line.kind == LogKind::Info
                    && line.text.contains("Quick eval not submitted")
                    && line.text.contains("kept")
            }),
            "logs were {logs:?}"
        );

        gate.release();
        assert!(app.workbench.wait_until_idle(Duration::from_secs(5)));
    }

    /// One assembled view whose result is the given display text plus a 2x2 projection.
    fn result_view(display: &str) -> DocumentView {
        use lovelace_json::Json;
        let cell = |value: &str| {
            Json::obj([
                ("kind", Json::Str("Natural".to_string())),
                ("value", Json::Str(value.to_string())),
                ("exact", Json::Bool(true)),
            ])
        };
        let structured = Json::obj([
            ("kind", Json::Str("Array".to_string())),
            ("type", Json::Str("Array".to_string())),
            ("shape", Json::num_array([2, 2])),
            (
                "elements",
                Json::Arr(vec![cell("1"), cell("2"), cell("3"), cell("4")]),
            ),
        ]);
        DocumentView {
            revision: crate::execution::SourceRevision(1),
            has_snapshot: true,
            committed_steps: 1,
            total: 1,
            result: display.to_string(),
            result_structured: Some(structured),
            result_inspection: None,
            variables: Vec::new(),
            functions: Vec::new(),
            state: EngineState::default(),
            plot: None,
            output: Vec::new(),
            logs: Vec::new(),
            diagnostic: None,
            run_state: RunState::Idle,
            elapsed_ms: 0,
        }
    }

    fn collect_shape_text(shape: &egui::Shape, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => out.push(text.galley.text().to_string()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_shape_text(shape, out);
                }
            }
            _ => {}
        }
    }

    /// RES-001 at the paint layer: the workspace RESULT section paints the tree from the
    /// structured projection. The display is deliberately contradictory (it names a record and
    /// a flat list while the projection is a 2x2 array); no record header or field label may be
    /// painted, and the 2x2 rows and scalar values must be.
    #[test]
    fn the_result_section_paints_the_projection_and_never_parses_the_display() {
        let ctx = egui::Context::default();
        let view = result_view("DivRemResult(quotient: 9, remainder: 9) [1, 2, 3, 4]");
        let mut expansion = ExpansionState::new();
        let mut toggles = Vec::new();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(460.0, 900.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                paint_result_section(ui, &view, 420.0, &mut expansion, &mut toggles);
            });
        });
        output.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            collect_shape_text(&clipped.shape, &mut texts);
        }

        assert!(
            texts
                .iter()
                .any(|text| text == "\u{25be} Array 2 x 2 - 4 elements"),
            "the array headline must state the projection's shape: {texts:?}"
        );
        for value in ["1", "2", "3", "4"] {
            assert!(
                texts.iter().any(|text| text == value),
                "the matrix must paint element {value}: {texts:?}"
            );
        }
        assert!(
            texts.iter().any(|text| text == "Natural"),
            "the scalar kind must come from the projection: {texts:?}"
        );
        assert!(
            !texts
                .iter()
                .any(|text| text.contains("DivRemResult - ")),
            "the record-looking display must not create a record header: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.ends_with("quotient:")),
            "the record-looking display must not create a field label: {texts:?}"
        );
    }

    /// One nested `SolveResult` view at the paint layer: its `solutions` field is a vector of two
    /// `Solution` records with a symbolic value. It is the shape `solve_full(x^2 - 4 == 0, x)`
    /// publishes.
    fn solve_result_view(values: [&str; 2]) -> DocumentView {
        use lovelace_json::Json;
        let symbolic = |pretty: &str| {
            Json::obj([
                ("kind", Json::Str("Symbolic".to_string())),
                ("pretty", Json::Str(pretty.to_string())),
                ("canonical", Json::Str(format!("(sym {pretty})"))),
                ("freeSymbols", Json::str_array(["x"])),
                ("nodeCount", Json::Str("1".to_string())),
            ])
        };
        let solution = |value: &str| {
            Json::obj([
                ("kind", Json::Str("Record".to_string())),
                ("type", Json::Str("Solution".to_string())),
                (
                    "fields",
                    Json::Arr(vec![
                        Json::obj([
                            ("name", Json::Str("value".to_string())),
                            ("value", symbolic(value)),
                        ]),
                        Json::obj([
                            ("name", Json::Str("multiplicity".to_string())),
                            (
                                "value",
                                Json::obj([
                                    ("kind", Json::Str("Integer".to_string())),
                                    ("value", Json::Str("1".to_string())),
                                ]),
                            ),
                        ]),
                    ]),
                ),
            ])
        };
        let structured = Json::obj([
            ("kind", Json::Str("Record".to_string())),
            ("type", Json::Str("SolveResult".to_string())),
            (
                "fields",
                Json::Arr(vec![
                    Json::obj([
                        ("name", Json::Str("status".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Enum".to_string())),
                                ("value", Json::Str("Solved".to_string())),
                                ("type", Json::Str("SolveStatus".to_string())),
                            ]),
                        ),
                    ]),
                    Json::obj([
                        ("name", Json::Str("solutions".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Array".to_string())),
                                ("type", Json::Str("Vector".to_string())),
                                ("shape", Json::num_array([2])),
                                (
                                    "elements",
                                    Json::Arr(
                                        values.iter().map(|value| solution(value)).collect(),
                                    ),
                                ),
                            ]),
                        ),
                    ]),
                ]),
            ),
        ]);
        let mut view = result_view("SolveResult(status: Solved, solutions: [...])");
        view.result_structured = Some(structured);
        view
    }

    /// Paints one RESULT section pass and returns every Text shape in paint order. The toggle
    /// list is cleared first, so it always describes the pass that was just painted.
    fn paint_result_frame(
        ctx: &egui::Context,
        view: &DocumentView,
        expansion: &mut ExpansionState,
        toggles: &mut Vec<(String, Rect)>,
        events: Vec<egui::Event>,
    ) -> Vec<String> {
        toggles.clear();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(460.0, 900.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                paint_result_section(ui, view, 420.0, expansion, toggles);
            });
        });
        output.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            collect_shape_text(&clipped.shape, &mut texts);
        }
        texts
    }

    /// The rectangle the last pass painted for one header path.
    fn toggle_rect(toggles: &[(String, Rect)], path: &str) -> Rect {
        toggles
            .iter()
            .find(|(recorded, _)| recorded == path)
            .map(|(_, rect)| *rect)
            .unwrap_or_else(|| panic!("no painted header for {path}: {toggles:?}"))
    }

    /// One pointer button event at `pos`.
    fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// RES-004 at the paint layer: collapsing a nested record field hides its children, keeps
    /// them in the model, and re-expanding paints the identical content.
    #[test]
    fn the_result_tree_collapses_a_nested_field_and_restores_identical_content() {
        let ctx = egui::Context::default();
        let view = solve_result_view(["-2", "2"]);
        let mut expansion = ExpansionState::new();
        let mut toggles = Vec::new();

        let expanded = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            expanded
                .iter()
                .any(|text| text.ends_with("solutions: Vector 2 - 2 elements")),
            "the field header names the field and its value: {expanded:?}"
        );
        assert!(
            expanded
                .iter()
                .any(|text| text.ends_with("[0] Solution - 2 fields")),
            "the expanded field paints its nested rows: {expanded:?}"
        );
        assert!(
            expanded.iter().any(|text| text == "pretty     -2"),
            "the expanded field paints the nested symbolic value: {expanded:?}"
        );
        // The header's recorded path is the key the map uses.
        let _ = toggle_rect(&toggles, "result.solutions");

        expansion.set_collapsed("result.solutions", true);
        let collapsed = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            collapsed
                .iter()
                .any(|text| text == "\u{25b8} solutions: Vector 2 - 2 elements"),
            "the collapsed field keeps its header and shows the right-pointing indicator: {collapsed:?}"
        );
        assert!(
            !collapsed
                .iter()
                .any(|text| text.contains("Solution - 2 fields")),
            "collapsing hides the nested record rows: {collapsed:?}"
        );
        assert!(
            !collapsed.iter().any(|text| text == "pretty     -2"),
            "collapsing hides the nested symbolic value: {collapsed:?}"
        );

        // The children the pass stopped drawing are still in the model.
        let projection = ResultProjection::new(view.result.clone(), view.result_structured.clone());
        let tree = projection.tree().expect("the view's projection is a tree");
        let StructuredNode::Record(record) = &tree else {
            panic!("the projection is a record");
        };
        let StructuredNode::Array(solutions) = record
            .field("solutions")
            .expect("the record publishes its solutions")
        else {
            panic!("solutions is a vector");
        };
        assert_eq!(
            solutions.elements.len(),
            2,
            "a collapsed field's children stay in the model"
        );

        expansion.set_collapsed("result.solutions", false);
        let again = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert_eq!(
            again, expanded,
            "re-expanding must paint byte-identical content"
        );
    }

    /// RES-004: a pointer click and Space on the focused header change the same map entry. The
    /// click also gives the header keyboard focus, so Space acts on the node the user clicked.
    #[test]
    fn a_click_and_space_on_a_result_header_change_the_same_state() {
        let ctx = egui::Context::default();
        let view = solve_result_view(["-2", "2"]);
        let projection = ResultProjection::new(view.result.clone(), view.result_structured.clone());
        let mut expansion = ExpansionState::new();
        let mut toggles = Vec::new();

        let _ = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        let pos = toggle_rect(&toggles, "result.solutions").center();
        for event in [
            egui::Event::PointerMoved(pos),
            pointer_button(pos, true),
            pointer_button(pos, false),
        ] {
            let _ = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, vec![event]);
        }
        assert!(
            expansion.is_collapsed("result.solutions"),
            "a pointer click collapses the field it hits"
        );

        let mut clicked = ExpansionState::new();
        clicked.bind(&projection);
        clicked.toggle("result.solutions");
        assert_eq!(
            expansion, clicked,
            "a click writes the same path entry as a toggle"
        );

        // The click moved focus to the header; Space is the keyboard route to the same toggle.
        let _ = paint_result_frame(
            &ctx,
            &view,
            &mut expansion,
            &mut toggles,
            vec![key(Key::Space, egui::Modifiers::default())],
        );
        assert!(
            !expansion.is_collapsed("result.solutions"),
            "Space on the focused header expands it again"
        );
        let mut clicked_twice = ExpansionState::new();
        clicked_twice.bind(&projection);
        clicked_twice.toggle("result.solutions");
        clicked_twice.toggle("result.solutions");
        assert_eq!(
            expansion, clicked_twice,
            "click then Space leaves the same state as two clicks"
        );

        // Enter is the other keyboard activation egui maps to the same click.
        let _ = paint_result_frame(
            &ctx,
            &view,
            &mut expansion,
            &mut toggles,
            vec![key(Key::Enter, egui::Modifiers::default())],
        );
        assert!(
            expansion.is_collapsed("result.solutions"),
            "Enter on the focused header collapses it again"
        );
        let mut clicked_three_times = ExpansionState::new();
        clicked_three_times.bind(&projection);
        for _ in 0..3 {
            clicked_three_times.toggle("result.solutions");
        }
        assert_eq!(
            expansion, clicked_three_times,
            "click, Space and Enter leave the same state as three clicks"
        );
    }

    /// RES-004: the collapse state follows the committed result, not the paint pass. An edit
    /// (a new revision) and a quick eval (new logs/output, no published snapshot) keep it; a new
    /// result clears it and paints expanded.
    #[test]
    fn a_result_collapse_survives_edits_and_a_quick_eval_and_a_new_result_clears_it() {
        let ctx = egui::Context::default();
        let view = solve_result_view(["-2", "2"]);
        let mut expansion = ExpansionState::new();
        let mut toggles = Vec::new();
        let _ = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        expansion.set_collapsed("result.solutions", true);

        // An unrelated update: the buffer moved to revision 99 and the frame carries a quick
        // eval's output and a new run state. The committed result itself is unchanged.
        let mut edited = view.clone();
        edited.revision = crate::execution::SourceRevision(99);
        edited.output.push("6 * 7".to_string());
        edited.elapsed_ms = 3;
        edited.run_state = RunState::Running;
        let texts = paint_result_frame(&ctx, &edited, &mut expansion, &mut toggles, Vec::new());
        assert!(
            expansion.is_collapsed("result.solutions"),
            "an edit and a quick eval must not reset the tree"
        );
        assert!(
            !texts
                .iter()
                .any(|text| text.contains("Solution - 2 fields")),
            "the collapsed field stays collapsed across the repaint: {texts:?}"
        );

        // A new result replaces the old one: the map is cleared and the new tree paints expanded.
        let new_view = solve_result_view(["-3", "3"]);
        let texts = paint_result_frame(&ctx, &new_view, &mut expansion, &mut toggles, Vec::new());
        assert!(expansion.is_empty(), "a new result clears the path map");
        assert!(!expansion.is_collapsed("result.solutions"));
        assert!(
            texts
                .iter()
                .any(|text| text.contains("[0] Solution - 2 fields")),
            "the new result paints expanded: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "pretty     -3"),
            "and paints its own values: {texts:?}"
        );
    }

    /// One symbolic root view at the paint layer: `Symbolic x^2 - 1` with both renderings.
    fn symbolic_result_view() -> DocumentView {
        use lovelace_json::Json;
        let structured = Json::obj([
            ("kind", Json::Str("Symbolic".to_string())),
            ("pretty", Json::Str("x^2 - 1".to_string())),
            (
                "canonical",
                Json::Str("(add (pow (sym x) (rat 2 1)) (rat -1 1))".to_string()),
            ),
            ("freeSymbols", Json::str_array(["x"])),
            ("nodeCount", Json::Str("5".to_string())),
        ]);
        let mut view = result_view("Symbolic x^2 - 1");
        view.result_structured = Some(structured);
        view
    }

    /// RES-004: a symbolic node is collapsible like any other node; collapsing hides both
    /// renderings, the model keeps them, and re-expanding is byte-identical.
    #[test]
    fn the_result_tree_collapses_a_symbolic_result_without_losing_it() {
        let ctx = egui::Context::default();
        let view = symbolic_result_view();
        let mut expansion = ExpansionState::new();
        let mut toggles = Vec::new();
        let expanded = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            expanded
                .iter()
                .any(|text| text == "\u{25be} Symbolic  x^2 - 1"),
            "the symbolic header names the pretty rendering: {expanded:?}"
        );
        assert!(
            expanded.iter().any(|text| text == "pretty     x^2 - 1"),
            "the expanded symbolic node paints its pretty rendering: {expanded:?}"
        );

        expansion.set_collapsed("result", true);
        let collapsed = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            collapsed
                .iter()
                .any(|text| text == "\u{25b8} Symbolic  x^2 - 1"),
            "the collapsed symbol keeps a header with the right-pointing indicator: {collapsed:?}"
        );
        assert!(
            !collapsed.iter().any(|text| text.starts_with("pretty")),
            "collapsing hides the pretty rendering: {collapsed:?}"
        );
        assert!(
            !collapsed.iter().any(|text| text.starts_with("canonical")),
            "collapsing hides the canonical rendering: {collapsed:?}"
        );

        let projection = ResultProjection::new(view.result.clone(), view.result_structured.clone());
        let tree = projection.tree().expect("the view's projection is a tree");
        let StructuredNode::Symbolic(symbolic) = &tree else {
            panic!("the projection is a symbolic node");
        };
        assert_eq!(symbolic.pretty, "x^2 - 1");
        assert!(!symbolic.canonical.is_empty());

        expansion.set_collapsed("result", false);
        let again = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert_eq!(
            again, expanded,
            "re-expanding the symbolic node paints byte-identical content"
        );
    }

    /// RES-004: an array row is a collapsible group of its own. Collapsing one matrix/vector row
    /// hides that row's children and leaves its sibling untouched.
    #[test]
    fn an_array_row_collapses_independently_of_its_sibling() {
        let ctx = egui::Context::default();
        let view = solve_result_view(["-2", "2"]);
        let mut expansion = ExpansionState::new();
        let mut toggles = Vec::new();
        let expanded = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            expanded
                .iter()
                .any(|text| text.ends_with("[0] Solution - 2 fields")),
            "the vector paints one header per row: {expanded:?}"
        );
        assert!(
            expanded.iter().any(|text| text == "pretty     -2"),
            "row 0 paints its symbolic value: {expanded:?}"
        );
        assert!(
            expanded.iter().any(|text| text == "pretty     2"),
            "row 1 paints its symbolic value: {expanded:?}"
        );

        expansion.set_collapsed("result.solutions[0]", true);
        let collapsed = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            collapsed
                .iter()
                .any(|text| text == "\u{25b8} [0] Solution - 2 fields"),
            "row 0 is collapsed: {collapsed:?}"
        );
        assert!(
            !collapsed.iter().any(|text| text == "pretty     -2"),
            "collapsing row 0 hides its value: {collapsed:?}"
        );
        assert!(
            collapsed.iter().any(|text| text == "pretty     2"),
            "row 1 keeps its value: {collapsed:?}"
        );

        let projection = ResultProjection::new(view.result.clone(), view.result_structured.clone());
        let tree = projection.tree().expect("the view's projection is a tree");
        let StructuredNode::Record(record) = &tree else {
            panic!("the projection is a record");
        };
        let StructuredNode::Array(solutions) = record.field("solutions").unwrap() else {
            panic!("solutions is a vector");
        };
        assert_eq!(solutions.elements.len(), 2, "both rows stay in the model");

        expansion.set_collapsed("result.solutions[0]", false);
        let again = paint_result_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());
        assert_eq!(
            again, expanded,
            "re-expanding a row paints byte-identical content"
        );
    }

    /// RES-004 end to end through the shipped window layout: a pointer click on a painted field
    /// header collapses that path in the live tree, and editing the buffer (which advances the
    /// revision and marks the result stale without publishing a new one) leaves the map alone.
    #[test]
    #[cfg(feature = "real-engine")]
    fn a_window_click_collapses_a_field_and_an_edit_keeps_it() {
        let mut workbench = Workbench::new();
        workbench.set_source(
            0,
            "x = symbol(\"x\")\nsolve_full(x^2 - 4 == 0, x)".to_string(),
        );
        let mut app = WorkbenchApp::with_workbench(workbench);
        assert!(app.workbench.run_active(), "the test run must start");
        assert!(
            app.workbench.wait_until_idle(Duration::from_secs(15)),
            "the solve run must finish"
        );

        let ctx = egui::Context::default();
        let screen = vec2(2500.0, 1380.0);
        frame_sized(&ctx, &mut app, screen, Vec::new());
        let rect = app
            .result_toggle_rects
            .iter()
            .find(|(path, _)| path == "result.solutions")
            .map(|(_, rect)| *rect)
            .expect("the window paints a header for result.solutions");
        let pos = rect.center();
        frame_sized(&ctx, &mut app, screen, vec![egui::Event::PointerMoved(pos)]);
        frame_sized(&ctx, &mut app, screen, vec![pointer_button(pos, true)]);
        frame_sized(&ctx, &mut app, screen, vec![pointer_button(pos, false)]);
        assert!(
            app.expansion.is_collapsed("result.solutions"),
            "the click on the painted header must collapse the field"
        );

        // An edit advances the buffer revision and marks the displayed result stale; it
        // publishes no new result, so the tree keeps every expansion decision.
        app.workbench.set_source(
            0,
            "x = symbol(\"x\")\nsolve_full(x^2 - 4 == 0, x)\n# edited".to_string(),
        );
        frame_sized(&ctx, &mut app, screen, Vec::new());
        assert!(
            app.expansion.is_collapsed("result.solutions"),
            "editing the buffer must not reset the result tree"
        );
    }

    /// SYM-001: one inspection payload in the exact shape `engine::real::inspection` publishes.
    /// `mathir` is either the supplied IR object or a `{reason}` object.
    fn inspector_payload(mathir: lovelace_json::Json) -> lovelace_json::Json {
        use lovelace_json::Json;
        let relation = Json::obj([
            ("source", Json::Str("session".to_string())),
            ("kind", Json::Str("Relation".to_string())),
            ("op", Json::Str("gt".to_string())),
            ("opLabel", Json::Str(">".to_string())),
            ("left", Json::Str("x".to_string())),
            ("right", Json::Str("0".to_string())),
            ("label", Json::Str("x > 0".to_string())),
            (
                "canonical",
                Json::Str("(gt (sym x) (rat 0 1))".to_string()),
            ),
        ]);
        let leaf = |kind: &str, label: &str| {
            Json::obj([
                ("kind", Json::Str(kind.to_string())),
                ("label", Json::Str(label.to_string())),
                ("children", Json::Arr(Vec::new())),
            ])
        };
        let finite = Json::obj([
            ("source", Json::Str("required".to_string())),
            ("kind", Json::Str("FiniteCondition".to_string())),
            (
                "label",
                Json::Str("FiniteCondition(expression: log(x))".to_string()),
            ),
        ]);
        Json::obj([
            (
                "subject",
                Json::obj([
                    ("kind", Json::Str("Symbolic".to_string())),
                    ("path", Json::Str("result".to_string())),
                ]),
            ),
            (
                "sections",
                Json::obj([
                    (
                        "pretty",
                        Json::obj([("text", Json::Str("3*x^2".to_string()))]),
                    ),
                    (
                        "canonical",
                        Json::obj([(
                            "text",
                            Json::Str("(mul (rat 3 1) (pow (sym x) (rat 2 1)))".to_string()),
                        )]),
                    ),
                    (
                        "tree",
                        Json::obj([
                            ("nodes", Json::Num("5".to_string())),
                            (
                                "root",
                                Json::obj([
                                    ("kind", Json::Str("Mul".to_string())),
                                    ("label", Json::Str("3*x^2".to_string())),
                                    (
                                        "children",
                                        Json::Arr(vec![
                                            leaf("Num", "3"),
                                            Json::obj([
                                                ("kind", Json::Str("Pow".to_string())),
                                                ("label", Json::Str("x^2".to_string())),
                                                (
                                                    "children",
                                                    Json::Arr(vec![leaf("Sym", "x"), leaf("Num", "2")]),
                                                ),
                                            ]),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                    (
                        "assumptions",
                        Json::obj([("items", Json::Arr(vec![relation]))]),
                    ),
                    (
                        "trace",
                        Json::obj([(
                            "steps",
                            Json::Arr(vec![Json::obj([
                                ("ruleId", Json::Str("logexp.exp-log".to_string())),
                                ("classification", Json::Str("Conditional".to_string())),
                                (
                                    "before",
                                    Json::obj([
                                        ("pretty", Json::Str("exp(log(x))".to_string())),
                                        (
                                            "canonical",
                                            Json::Str("(fun exp (fun log (sym x)))".to_string()),
                                        ),
                                    ]),
                                ),
                                (
                                    "after",
                                    Json::obj([
                                        ("pretty", Json::Str("x".to_string())),
                                        ("canonical", Json::Str("(sym x)".to_string())),
                                    ]),
                                ),
                                ("conditions", Json::Arr(vec![finite])),
                            ])]),
                        )]),
                    ),
                    ("mathir", mathir),
                ]),
            ),
        ])
    }

    fn inspector_view(mathir: lovelace_json::Json) -> DocumentView {
        let mut view = result_view("Symbolic 3*x^2");
        view.result = "3*x^2".to_string();
        view.result_structured = Some(lovelace_json::Json::obj([
            (
                "kind",
                lovelace_json::Json::Str("Symbolic".to_string()),
            ),
            (
                "pretty",
                lovelace_json::Json::Str("3*x^2".to_string()),
            ),
            (
                "canonical",
                lovelace_json::Json::Str(
                    "(mul (rat 3 1) (pow (sym x) (rat 2 1)))".to_string(),
                ),
            ),
        ]));
        view.result_inspection = Some(inspector_payload(mathir));
        view
    }

    fn supplied_mathir() -> lovelace_json::Json {
        use lovelace_json::Json;
        Json::obj([
            (
                "text",
                Json::Str("#!mathir 2\nparam x\nParameter 0 0\nPowInt 0 0,1\nMul 0 0,2".to_string()),
            ),
            ("parameters", Json::str_array(["x"])),
            ("resultType", Json::Str("Integer".to_string())),
        ])
    }

    fn missing_mathir() -> lovelace_json::Json {
        use lovelace_json::Json;
        Json::obj([(
            "reason",
            Json::Str("Node kind Relation cannot be lowered to MathIR.".to_string()),
        )])
    }

    /// Paints one inspector pass and returns every Text shape in paint order.
    fn paint_inspector_frame(
        ctx: &egui::Context,
        view: &DocumentView,
        expansion: &mut InspectorExpansion,
        toggles: &mut Vec<(String, Rect)>,
        events: Vec<egui::Event>,
    ) -> Vec<String> {
        toggles.clear();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(460.0, 1200.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                paint_inspector(ui, view, 420.0, expansion, toggles);
            });
        });
        output.textures_delta.clear();
        let mut texts = Vec::new();
        for clipped in &output.shapes {
            collect_shape_text(&clipped.shape, &mut texts);
        }
        texts
    }

    /// SYM-001 at the paint layer: all six sections come from the payload, not from the display.
    #[test]
    fn the_inspector_paints_all_six_sections_from_the_engine_payload() {
        let ctx = egui::Context::default();
        let view = inspector_view(supplied_mathir());
        let mut expansion = InspectorExpansion::new();
        let mut toggles = Vec::new();
        let texts = paint_inspector_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());

        assert!(
            texts.iter().any(|text| text == "INSPECTOR · last committed result"),
            "the inspector heading: {texts:?}"
        );
        assert!(texts.iter().any(|text| text == "Pretty"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "3*x^2"),
            "the engine's pretty text: {texts:?}"
        );
        assert!(texts.iter().any(|text| text == "Canonical"), "{texts:?}");
        assert!(
            texts
                .iter()
                .any(|text| text == "(mul (rat 3 1) (pow (sym x) (rat 2 1)))"),
            "the engine's canonical text: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "Expression tree"),
            "{texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|text| text.ends_with("Mul · 3*x^2")),
            "the tree root's node kind and engine rendering: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "Num · 3"),
            "a leaf child in engine child order: {texts:?}"
        );
        assert!(texts.iter().any(|text| text == "Assumptions"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "relation >"),
            "the assumption is a typed relation, not prose: {texts:?}"
        );
        assert!(texts.iter().any(|text| text == "x > 0"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "Simplification trace"),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "1. logexp.exp-log"),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "classification  Conditional"),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "before  exp(log(x))"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|text| text == "after   x"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "FiniteCondition"),
            "the required condition's engine record kind: {texts:?}"
        );
        assert!(texts.iter().any(|text| text == "MathIR"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text.contains("#!mathir 2")),
            "the engine IR text is painted: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text.contains("display:")),
            "the inspector never paints the result display caption: {texts:?}"
        );
    }

    /// SYM-009: a subject that cannot lower to MathIR keeps its reason while every other
    /// supplied section stays usable.
    #[test]
    fn a_missing_mathir_shows_the_reason_and_leaves_the_other_sections_usable() {
        let ctx = egui::Context::default();
        let view = inspector_view(missing_mathir());
        let mut expansion = InspectorExpansion::new();
        let mut toggles = Vec::new();
        let texts = paint_inspector_frame(&ctx, &view, &mut expansion, &mut toggles, Vec::new());

        assert!(
            texts
                .iter()
                .any(|text| text == "Node kind Relation cannot be lowered to MathIR."),
            "the engine's specific refusal: {texts:?}"
        );
        assert!(texts.iter().any(|text| text == "MathIR"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "3*x^2"),
            "pretty stays usable: {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|text| text == "(mul (rat 3 1) (pow (sym x) (rat 2 1)))"),
            "canonical stays usable: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.ends_with("Mul · 3*x^2")),
            "the tree stays usable: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "relation >"),
            "assumptions stay usable: {texts:?}"
        );
    }

    /// SYM-002: the inspector renders the last committed result, not the editor buffer. An edit
    /// changes neither the payload nor the painted inspector; a new committed result does both.
    #[test]
    #[cfg(feature = "real-engine")]
    fn the_inspector_is_bound_to_the_committed_result_not_the_buffer() {
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::RealEngine::new())));
        workbench.set_source(
            0,
            "x = symbol(\"x\")\nassume(x > 0)\ndiff(x^3, x)".to_string(),
        );
        assert!(workbench.run_active(), "the first run must start");
        assert!(
            workbench.wait_until_idle(Duration::from_secs(15)),
            "the first run must finish"
        );
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        let screen = vec2(2500.0, 1380.0);
        frame_sized(&ctx, &mut app, screen, Vec::new());

        let before = app.view.clone();
        assert!(
            before.result_inspection.is_some(),
            "the real engine published an inspection payload"
        );
        let mut expansion = InspectorExpansion::new();
        let mut toggles = Vec::new();
        let first = paint_inspector_frame(&ctx, &before, &mut expansion, &mut toggles, Vec::new());
        assert!(
            first.iter().any(|text| text == "3*x^2"),
            "the committed diff result is inspected: {first:?}"
        );
        assert!(
            first.iter().any(|text| text == "x > 0"),
            "the session assumption is inspected: {first:?}"
        );

        // Editing advances the buffer revision and marks the result stale, but publishes no new
        // result: the inspector must not move.
        app.workbench.set_source(
            0,
            "x = symbol(\"x\")\n1 + 1".to_string(),
        );
        frame_sized(&ctx, &mut app, screen, Vec::new());
        assert_eq!(
            app.view.result_inspection, before.result_inspection,
            "editing the buffer must not change the inspected payload"
        );
        let after_edit =
            paint_inspector_frame(&ctx, &app.view, &mut expansion, &mut toggles, Vec::new());
        assert_eq!(
            after_edit, first,
            "editing the buffer must paint the same inspector"
        );

        // A new committed result replaces the payload, the tree and the rendered text.
        app.workbench.set_source(
            0,
            "x = symbol(\"x\")\nassume(x > 0)\ndiff(x^4, x)".to_string(),
        );
        assert!(app.workbench.run_active(), "the second run must start");
        assert!(
            app.workbench.wait_until_idle(Duration::from_secs(15)),
            "the second run must finish"
        );
        frame_sized(&ctx, &mut app, screen, Vec::new());
        assert_ne!(
            app.view.result_inspection, before.result_inspection,
            "a new committed result must publish a different inspection"
        );
        let second =
            paint_inspector_frame(&ctx, &app.view, &mut expansion, &mut toggles, Vec::new());
        assert!(
            second.iter().any(|text| text == "4*x^3"),
            "the new result's pretty form is painted: {second:?}"
        );
        assert!(
            !second.iter().any(|text| text == "3*x^2"),
            "the old result's pretty form is gone: {second:?}"
        );
    }

    /// SYM-001 through the shipped window layout: the Workspace pane's Inspector tab is a real
    /// pointer target, and selecting it swaps the Result view for the inspector.
    #[test]
    #[cfg(feature = "real-engine")]
    fn the_workspace_inspector_tab_swaps_the_view_and_the_result_tab_restores_it() {
        let mut workbench =
            Workbench::with_factory(Box::new(|| Box::new(crate::engine::RealEngine::new())));
        workbench.set_source(
            0,
            "x = symbol(\"x\")\nassume(x > 0)\ndiff(x^3, x)".to_string(),
        );
        assert!(workbench.run_active());
        assert!(workbench.wait_until_idle(Duration::from_secs(15)));
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        let screen = vec2(2500.0, 1380.0);
        frame_sized(&ctx, &mut app, screen, Vec::new());
        assert!(!app.inspector_open, "the Result view is the default");

        let inspector_tab = app
            .workspace_view_tabs[1]
            .expect("the window painted the Inspector tab");
        let pos = inspector_tab.center();
        frame_sized(&ctx, &mut app, screen, vec![egui::Event::PointerMoved(pos)]);
        frame_sized(&ctx, &mut app, screen, vec![pointer_button(pos, true)]);
        frame_sized(&ctx, &mut app, screen, vec![pointer_button(pos, false)]);
        assert!(
            app.inspector_open,
            "clicking the Inspector tab must select it"
        );
        frame_sized(&ctx, &mut app, screen, Vec::new());
        let mut texts = Vec::new();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.ui_root(ui));
        output.textures_delta.clear();
        for clipped in &output.shapes {
            collect_shape_text(&clipped.shape, &mut texts);
        }
        assert!(
            texts.iter().any(|text| text == "INSPECTOR · last committed result"),
            "the inspector heading is painted: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "3*x^2"),
            "the committed pretty form is painted: {texts:?}"
        );
        assert!(
            !texts.iter().any(|text| text == "RESULT · structured projection"),
            "the Result view is replaced, not stacked: {texts:?}"
        );

        let result_tab = app
            .workspace_view_tabs[0]
            .expect("the window painted the Result tab");
        let pos = result_tab.center();
        frame_sized(&ctx, &mut app, screen, vec![egui::Event::PointerMoved(pos)]);
        frame_sized(&ctx, &mut app, screen, vec![pointer_button(pos, true)]);
        frame_sized(&ctx, &mut app, screen, vec![pointer_button(pos, false)]);
        assert!(!app.inspector_open, "clicking Result must restore the result view");
    }

    // -----------------------------------------------------------------------------------------
    // DOC-002 / DOC-003: the window's file lifecycle and the unsaved-changes prompt.
    // -----------------------------------------------------------------------------------------

    /// A file in the test scratch directory, unique per test and process.
    fn fixture(name: &str, bytes: &[u8]) -> PathBuf {
        let directory = std::env::temp_dir().join("lovelace-desktop-window-file-tests");
        std::fs::create_dir_all(&directory).expect("scratch directory");
        let path = directory.join(format!("{name}-{}.lv", std::process::id()));
        std::fs::write(&path, bytes).expect("fixture bytes");
        path
    }

    /// A workbench with the fixture opened in its own tab, which becomes active.
    fn opened_workbench(path: &std::path::Path) -> Workbench {
        let mut workbench = slow_workbench();
        workbench.open(path).expect("open the fixture");
        workbench
    }

    /// Answers both dialogs from a queue, so Open and Save As can run headlessly.
    #[derive(Default)]
    struct ScriptedDialogs {
        open: std::collections::VecDeque<Option<PathBuf>>,
        save: std::collections::VecDeque<Option<PathBuf>>,
    }

    impl ScriptedDialogs {
        fn opening(paths: impl IntoIterator<Item = Option<PathBuf>>) -> ScriptedDialogs {
            ScriptedDialogs {
                open: paths.into_iter().collect(),
                save: std::collections::VecDeque::new(),
            }
        }

        fn saving(paths: impl IntoIterator<Item = Option<PathBuf>>) -> ScriptedDialogs {
            ScriptedDialogs {
                open: std::collections::VecDeque::new(),
                save: paths.into_iter().collect(),
            }
        }
    }

    impl FileDialogs for ScriptedDialogs {
        fn open(&mut self) -> Option<PathBuf> {
            self.open.pop_front().flatten()
        }

        fn save_as(&mut self, _suggested: &str, _initial_dir: Option<&std::path::Path>) -> Option<PathBuf> {
            self.save.pop_front().flatten()
        }
    }

    /// One pass with no events, used to settle the frame after a state change.
    fn settle(ctx: &egui::Context, app: &mut WorkbenchApp) {
        frame(ctx, app, Vec::new());
        frame(ctx, app, Vec::new());
    }

    #[test]
    fn the_prompt_names_the_document_and_offers_save_discard_and_cancel() {
        let path = fixture("prompt-buttons", b"a = 1\nb = 2\n");
        let mut workbench = opened_workbench(&path);
        let index = workbench.active();
        workbench.set_source(index, "a = 1\nb = 3\n".to_string());
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);
        assert!(app.close.is_some(), "a dirty tab must ask before closing");
        settle(&ctx, &mut app);

        assert_eq!(
            prompt_choices(&app),
            vec!["Save", "Discard", "Cancel"],
            "the prompt offers exactly the three answers, in that order"
        );
        let prompt = app.close.as_ref().expect("prompt");
        assert!(prompt.error.is_none());
        assert_eq!(prompt.pending.len(), 1);
        assert_eq!(prompt.total, 1);
        // The document the question names is the dirty one.
        assert_eq!(
            app.workbench
                .document_index(prompt.current())
                .map(|index| app.workbench.documents()[index].title_with_marker()),
            Some(format!("{} *", path.file_name().unwrap().to_string_lossy()))
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cancel_keeps_the_buffer_the_caret_the_tab_and_the_focus() {
        let path = fixture("prompt-cancel", b"a = 1\nb = 2\n");
        let mut workbench = opened_workbench(&path);
        let index = workbench.active();
        workbench.set_source(index, "a = 1\nb = 3\n".to_string());
        let tabs = workbench.documents().len();
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        app.editors[index].set_caret(1, 3);
        let caret = app.editors[index].caret();
        frame(&ctx, &mut app, Vec::new());
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(surface_id()),
            "the editor owns the focus before the prompt"
        );

        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);
        settle(&ctx, &mut app);
        assert!(app.close.is_some(), "the prompt must be open");
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            None,
            "the prompt owns the keyboard: the editor is not focused while it is open"
        );

        // Escape is the prompt's Cancel.
        frame(&ctx, &mut app, vec![key(Key::Escape, egui::Modifiers::default())]);

        assert!(app.close.is_none(), "Cancel dismisses the prompt");
        assert_eq!(
            app.workbench.documents().len(),
            tabs,
            "Cancel closes nothing"
        );
        assert_eq!(
            app.workbench.documents()[index].source, "a = 1\nb = 3\n",
            "Cancel keeps the edited buffer"
        );
        assert!(app.workbench.documents()[index].dirty(), "it is still unsaved");
        assert_eq!(app.editors[index].caret(), caret, "the caret has not moved");
        // The modal's layer blocks the editor for one pass after it stops being painted, so the
        // focus it had before the prompt is granted back on the pass after that.
        frame(&ctx, &mut app, Vec::new());
        frame(&ctx, &mut app, Vec::new());
        assert_eq!(
            ctx.memory(|memory| memory.focused()),
            Some(surface_id()),
            "focus is back in the editor, where it was"
        );
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            b"a = 1\nb = 2\n",
            "Cancel writes nothing"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn discard_closes_the_tab_without_writing_to_the_file() {
        let path = fixture("prompt-discard", b"x = 1\n");
        let mut workbench = opened_workbench(&path);
        let index = workbench.active();
        workbench.set_source(index, "x = 2\n".to_string());
        let tabs = workbench.documents().len();
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);
        settle(&ctx, &mut app);

        let discard = prompt_button(&app, UnsavedAnswer::Discard);
        pointer_click(&ctx, &mut app, discard);

        assert!(app.close.is_none(), "Discard finishes the close");
        assert_eq!(
            app.workbench.documents().len(),
            tabs - 1,
            "the tab is closed"
        );
        assert_eq!(app.editors.len(), tabs - 1, "its editor is gone too");
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            b"x = 1\n",
            "Discard writes nothing"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_in_the_prompt_writes_the_edit_then_closes_the_tab() {
        let path = fixture("prompt-save", b"x = 1\n");
        let mut workbench = opened_workbench(&path);
        let index = workbench.active();
        workbench.set_source(index, "x = 2\n".to_string());
        let tabs = workbench.documents().len();
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);
        settle(&ctx, &mut app);

        let save = prompt_button(&app, UnsavedAnswer::Save);
        pointer_click(&ctx, &mut app, save);

        assert!(app.close.is_none(), "Save finishes the close");
        assert_eq!(app.workbench.documents().len(), tabs - 1);
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            b"x = 2\n",
            "Save in the prompt writes the edited buffer"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_clean_tab_closes_without_a_prompt() {
        let path = fixture("clean-close", b"x = 1\n");
        let workbench = opened_workbench(&path);
        let tabs = workbench.documents().len();
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);

        assert!(app.close.is_none(), "a clean tab must not ask anything");
        assert_eq!(app.workbench.documents().len(), tabs - 1);
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            b"x = 1\n",
            "closing a clean tab writes nothing"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_window_close_request_with_unsaved_changes_is_refused_and_prompts() {
        let path = fixture("window-close", b"a = 1\n");
        let mut workbench = opened_workbench(&path);
        let index = workbench.active();
        workbench.set_source(index, "a = 2\n".to_string());
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        let output = close_request(&ctx, &mut app);

        assert!(
            cancelled_close(&output),
            "the window must refuse the close while a document is dirty"
        );
        assert!(app.close.is_some(), "the refusal is a prompt");
        assert!(
            !app.window_close_approved,
            "the window has not been allowed to close"
        );

        // Cancel the prompt with Escape: the window stays open, nothing was written.
        settle(&ctx, &mut app);
        frame(&ctx, &mut app, vec![key(Key::Escape, egui::Modifiers::default())]);
        assert!(app.close.is_none());
        assert!(!app.window_close_approved);
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            b"a = 1\n",
            "refusing the close writes nothing"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_window_close_request_on_a_clean_workbench_is_allowed() {
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        let output = close_request(&ctx, &mut app);

        assert!(
            !cancelled_close(&output),
            "a clean workbench does not refuse a close"
        );
        assert!(
            !approved_close(&output),
            "no close command is needed: the request is simply allowed through"
        );
        assert!(app.window_close_approved);
        assert!(app.close.is_none());
    }

    #[test]
    fn quit_from_the_file_bar_prompts_and_discard_closes_the_window() {
        let path = fixture("quit-prompt", b"a = 1\n");
        let mut workbench = opened_workbench(&path);
        let index = workbench.active();
        workbench.set_source(index, "a = 2\n".to_string());
        let mut app = WorkbenchApp::with_workbench(workbench);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        let quit = app
            .file_buttons
            .iter()
            .find(|(label, _)| *label == "Quit")
            .expect("the File bar paints Quit")
            .1
            .center();
        pointer_click(&ctx, &mut app, quit);

        assert!(app.close.is_some(), "Quit asks about the dirty document");
        settle(&ctx, &mut app);
        let discard = prompt_button(&app, UnsavedAnswer::Discard);
        pointer_click(&ctx, &mut app, discard);

        assert!(app.close.is_none());
        assert!(
            app.window_close_approved,
            "Discard on Quit approves closing the window"
        );
        // The next OS close request is allowed through instead of asking again.
        let output = close_request(&ctx, &mut app);
        assert!(!cancelled_close(&output));
        assert_eq!(
            std::fs::read(&path).expect("read back"),
            b"a = 1\n",
            "Discard from Quit writes nothing"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn saving_a_pathless_document_from_the_prompt_asks_for_a_path_then_closes() {
        let path = fixture("prompt-save-as", b"");
        let _ = std::fs::remove_file(&path);
        let mut workbench = slow_workbench();
        let index = workbench.new_document();
        workbench.set_source(index, "q = 5\n".to_string());
        let mut app = WorkbenchApp::with_workbench(workbench);
        app.dialogs = Box::new(ScriptedDialogs::saving([Some(path.clone())]));
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);
        settle(&ctx, &mut app);
        assert!(app.close.is_some(), "the dirty tab prompts first");
        assert!(
            prompt_choices(&app).contains(&"Save"),
            "the prompt offers Save"
        );

        // Answer Save: the workbench has no path, so the scripted Save As dialog supplies one.
        let save = prompt_button(&app, UnsavedAnswer::Save);
        pointer_click(&ctx, &mut app, save);

        assert!(app.close.is_none(), "the flow finished");
        assert_eq!(std::fs::read(&path).expect("read back"), b"q = 5\n");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ctrl_n_adds_a_tab_and_ctrl_w_closes_the_clean_one() {
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        assert_eq!(app.workbench.documents().len(), 2);

        frame(&ctx, &mut app, vec![key(Key::N, ctrl_modifiers())]);

        assert_eq!(app.workbench.documents().len(), 3, "Ctrl+N adds a tab");
        assert_eq!(app.editors.len(), 3, "and an editor for it");
        assert_eq!(app.workbench.active(), 2);
        assert_eq!(
            app.workbench.documents()[2].title_with_marker(),
            "untitled-2.lv"
        );
        assert_eq!(app.editors[2].source(), "", "the new tab starts empty");
        assert!(
            app.window_title.ends_with("untitled-2.lv"),
            "the window title names the active document: {:?}",
            app.window_title
        );

        frame(&ctx, &mut app, vec![key(Key::W, ctrl_modifiers())]);

        assert_eq!(app.workbench.documents().len(), 2, "Ctrl+W closes it");
        assert_eq!(app.editors.len(), 2);
        assert!(app.close.is_none(), "a clean tab does not prompt");
    }

    #[test]
    fn ctrl_shift_s_asks_for_a_path_and_ctrl_s_then_writes_to_it() {
        let path = fixture("save-as-binding", b"");
        let _ = std::fs::remove_file(&path);
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
        app.dialogs = Box::new(ScriptedDialogs::saving([Some(path.clone())]));
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        frame(&ctx, &mut app, vec![key(Key::S, ctrl_shift_modifiers())]);

        assert_eq!(
            app.workbench.documents()[0].path.as_deref(),
            Some(path.as_path()),
            "Ctrl+Shift+S adopts the chosen path"
        );
        assert_eq!(
            app.workbench.documents()[0].title,
            path.file_name().unwrap().to_string_lossy(),
            "Save As renames the tab"
        );
        let written = std::fs::read(&path).expect("the starter script was written");
        assert!(
            written.starts_with(b"# Lovelace Desktop."),
            "Save As wrote the buffer: {:?}",
            String::from_utf8_lossy(&written)
        );

        // One edit, then Ctrl+S: the document has a path, so no dialog may be needed. The
        // scripted queue is empty, so a dialog would return None and nothing would be written.
        frame(&ctx, &mut app, vec![egui::Event::Text("w".to_string())]);
        assert!(app.workbench.documents()[0].dirty());
        assert!(app.window_title.ends_with(" *"), "{:?}", app.window_title);
        frame(&ctx, &mut app, vec![key(Key::S, ctrl_modifiers())]);

        let written = std::fs::read(&path).expect("read back");
        assert!(
            written.starts_with(b"w# Lovelace Desktop."),
            "Ctrl+S wrote the edit to the document's own path: {:?}",
            String::from_utf8_lossy(&written)
        );
        assert!(!app.workbench.documents()[0].dirty(), "the marker is cleared");
        assert!(!app.window_title.ends_with(" *"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ctrl_o_opens_the_file_the_dialog_returns_and_the_tab_shows_its_name() {
        let path = fixture("open-binding", b"u = 7\r\n");
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
        app.dialogs = Box::new(ScriptedDialogs::opening([Some(path.clone())]));
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        frame(&ctx, &mut app, vec![key(Key::O, ctrl_modifiers())]);

        assert_eq!(app.workbench.documents().len(), 3, "Open added a tab");
        assert_eq!(app.editors.len(), 3, "with an editor");
        let document = &app.workbench.documents()[2];
        assert_eq!(document.title, path.file_name().unwrap().to_string_lossy());
        assert_eq!(document.source, "u = 7\r\n", "the CRLF text is intact");
        assert_eq!(document.newline, crate::document_io::NewlineStyle::CrLf);
        assert!(!document.dirty());
        assert!(
            app.window_title.ends_with(&document.title),
            "the window title names the opened file: {:?}",
            app.window_title
        );
        assert_eq!(
            app.editors[2].lines,
            vec!["u = 7", ""],
            "the editor shows no phantom carriage return"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn opening_a_non_utf8_file_through_the_dialog_is_refused_without_a_new_tab() {
        let path = fixture("open-bad-utf8", b"u = \xE9\n");
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
        app.dialogs = Box::new(ScriptedDialogs::opening([Some(path.clone())]));
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        frame(&ctx, &mut app, vec![key(Key::O, ctrl_modifiers())]);

        assert_eq!(
            app.workbench.documents().len(),
            2,
            "a refused file must not create a tab"
        );
        let logged = app
            .workbench
            .active_document()
            .logs
            .iter()
            .any(|line| line.kind == LogKind::Diagnostic && line.text.contains("not UTF-8"));
        assert!(logged, "the refusal is in the log with its reason");
        assert_eq!(
            std::fs::read(&path).expect("the file is untouched"),
            b"u = \xE9\n"
        );
        let _ = std::fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------------------------
    // DOC-004 / DOC-005: the persisted session, the recovery journal and the recovery prompt.
    // -----------------------------------------------------------------------------------------

    /// Scratch storage for a launch test: a settings file and a recovery directory.
    fn app_storage(name: &str) -> (AppStorage, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "lovelace-desktop-launch-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("scratch directory");
        (
            AppStorage {
                settings_path: Some(directory.join("desktop.json")),
                recovery_root: Some(directory.join("recovery")),
            },
            directory,
        )
    }

    /// Writes the state a forced kill leaves: a session naming the file and a journaled edit.
    fn crash_state(
        storage: &AppStorage,
        file: &std::path::Path,
        source: &str,
        caret: (usize, usize),
    ) -> RecoveryRecord {
        let record = RecoveryRecord {
            document_id: 1,
            revision: 1,
            title: file.file_name().unwrap().to_string_lossy().into_owned(),
            path: Some(file.to_path_buf()),
            bom: false,
            caret,
            selection: None,
            source_mtime: None,
            saved_at: std::time::SystemTime::UNIX_EPOCH,
            source: source.to_string(),
        };
        let root = storage.recovery_root.as_ref().expect("recovery root");
        record
            .write_atomic(&recovery::record_path(root, 1))
            .expect("write the recovery record");
        let mut settings = Settings::new();
        settings.set_session(crate::settings::SessionState::new(
            vec![crate::settings::SessionTab {
                path: Some(file.to_string_lossy().into_owned()),
                document_id: 1,
            }],
            0,
        ));
        settings
            .save(storage.settings_path.as_ref().expect("settings path"))
            .expect("save the settings");
        record
    }

    /// The centre of a recovery prompt button painted in the last pass.
    fn recovery_button(app: &WorkbenchApp, choice: RecoveryAnswer) -> egui::Pos2 {
        app.recovery_buttons
            .iter()
            .find(|(_, painted, _)| *painted == choice)
            .map(|(_, _, rect)| rect.center())
            .unwrap_or_else(|| panic!("the {choice:?} button was not painted"))
    }

    /// The recovery prompt buttons in their on-screen order.
    fn recovery_choices(app: &WorkbenchApp) -> Vec<&'static str> {
        app.recovery_buttons
            .iter()
            .map(|(_, choice, _)| choice.label())
            .collect()
    }

    #[test]
    fn the_recovery_prompt_offers_a_crashed_buffer_and_recovering_restores_it_before_any_save() {
        let (storage, directory) = app_storage("recover");
        let file = directory.join("notes.lv");
        std::fs::write(&file, b"a = 1\nb = 2\n").expect("fixture");
        let record = crash_state(&storage, &file, "a = 1\nb = 3\n", (1, 3));
        let record_path = recovery::record_path(storage.recovery_root.as_ref().unwrap(), 1);
        assert!(record_path.exists());

        let mut app = WorkbenchApp::launch(storage.clone(), None);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        // The session restored the file from disk; the missing edit is only in the journal.
        assert_eq!(app.workbench.documents().len(), 1);
        assert_eq!(app.workbench.documents()[0].source, "a = 1\nb = 2\n");
        assert_eq!(
            std::fs::read(&file).expect("their file"),
            b"a = 1\nb = 2\n",
            "restoring reads the file and writes nothing"
        );
        let prompt = app
            .recovery_prompt
            .as_ref()
            .expect("a surviving record is offered at launch");
        assert_eq!(prompt.records.len(), 1);
        assert_eq!(prompt.records[0], record, "the prompt lists the journaled record");
        assert_eq!(
            prompt.records[0].path_label(),
            file.display().to_string(),
            "the prompt reports the original path"
        );
        assert_eq!(recovery_choices(&app), vec!["Discard", "Recover"]);

        let recover_at = recovery_button(&app, RecoveryAnswer::Recover);
        pointer_click(&ctx, &mut app, recover_at);
        settle(&ctx, &mut app);

        assert!(app.recovery_prompt.is_none(), "the answer closes the prompt");
        let index = app.workbench.active();
        let document = &app.workbench.documents()[index];
        assert_eq!(document.source, "a = 1\nb = 3\n", "the exact journaled text is back");
        assert_eq!(document.path.as_deref(), Some(file.as_path()));
        assert_eq!(document.caret, (1, 3), "the journaled caret is back");
        assert!(document.dirty(), "the recovered text is unsaved work");
        assert_eq!(
            app.editors[index].source(),
            "a = 1\nb = 3\n",
            "the editor shows the recovered text"
        );
        assert_eq!(
            std::fs::read(&file).expect("their file"),
            b"a = 1\nb = 2\n",
            "recovering never writes the document file"
        );

        // Saving is the first action that may overwrite the file, and it retires the record.
        frame(&ctx, &mut app, vec![key(Key::S, ctrl_modifiers())]);
        settle(&ctx, &mut app);
        assert_eq!(std::fs::read(&file).expect("the saved file"), b"a = 1\nb = 3\n");
        assert!(!app.workbench.documents()[index].dirty());
        assert!(!record_path.exists(), "a clean document has no recovery record");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn discarding_a_recovery_record_deletes_it_and_leaves_the_file_and_buffer_alone() {
        let (storage, directory) = app_storage("discard");
        let file = directory.join("notes.lv");
        std::fs::write(&file, b"a = 1\nb = 2\n").expect("fixture");
        crash_state(&storage, &file, "a = 1\nb = 9\n", (0, 0));
        let record_path = recovery::record_path(storage.recovery_root.as_ref().unwrap(), 1);

        let mut app = WorkbenchApp::launch(storage.clone(), None);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        assert!(app.recovery_prompt.is_some());

        let discard_at = recovery_button(&app, RecoveryAnswer::Discard);
        pointer_click(&ctx, &mut app, discard_at);
        settle(&ctx, &mut app);

        assert!(app.recovery_prompt.is_none());
        assert!(!record_path.exists(), "discarding deletes the record");
        assert_eq!(std::fs::read(&file).expect("their file"), b"a = 1\nb = 2\n");
        assert_eq!(
            app.workbench.documents()[0].source, "a = 1\nb = 2\n",
            "discarding changes no buffer"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_corrupt_recovery_record_is_skipped_with_a_warning_and_the_launch_continues() {
        let (storage, directory) = app_storage("corrupt");
        let file = directory.join("notes.lv");
        std::fs::write(&file, b"a = 1\n").expect("fixture");
        crash_state(&storage, &file, "a = 1\nb = 3\n", (0, 0));
        let root = storage.recovery_root.as_ref().unwrap();
        let corrupt = recovery::record_path(root, 9);
        std::fs::write(&corrupt, b"LOVELACE-RECOVERY 1\nid: 9\nbytes: 100\n\nshort")
            .expect("corrupt record");

        let mut app = WorkbenchApp::launch(storage.clone(), None);
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);

        assert_eq!(
            app.recovery_prompt
                .as_ref()
                .map(|prompt| prompt.records.len()),
            Some(1),
            "only the valid record is offered"
        );
        let warned = app
            .workbench
            .documents()
            .iter()
            .flat_map(|document| document.logs.iter())
            .any(|line| {
                line.kind == LogKind::Diagnostic
                    && line.text.contains("corrupt recovery record")
                    && line.text.contains("doc-9.lvrec")
            });
        assert!(warned, "the skipped record is a warning naming the file");
        assert_eq!(std::fs::read(&file).expect("their file"), b"a = 1\n");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_session_is_written_at_a_clean_quit_and_restored_on_the_next_launch() {
        let (storage, directory) = app_storage("session");
        let first = directory.join("first.lv");
        let second = directory.join("second.lv");
        std::fs::write(&first, b"a = 1\n").expect("fixture");
        std::fs::write(&second, b"b = 2\n").expect("fixture");

        let ctx = egui::Context::default();
        let mut app = WorkbenchApp::launch(storage.clone(), None);
        app.dialogs = Box::new(ScriptedDialogs::opening([
            Some(first.clone()),
            Some(second.clone()),
        ]));
        settle(&ctx, &mut app);
        frame(&ctx, &mut app, vec![key(Key::O, ctrl_modifiers())]);
        frame(&ctx, &mut app, vec![key(Key::O, ctrl_modifiers())]);
        settle(&ctx, &mut app);
        assert_eq!(app.workbench.documents().len(), 4);
        let first_tab = app
            .workbench
            .documents()
            .iter()
            .position(|document| document.path.as_deref() == Some(first.as_path()))
            .expect("the first file has a tab");
        app.workbench.set_active(first_tab);
        settle(&ctx, &mut app);

        let output = close_request(&ctx, &mut app);
        assert!(
            !cancelled_close(&output),
            "a clean workbench does not refuse the close request"
        );
        assert!(app.window_close_approved);

        let saved = Settings::load(storage.settings_path.as_ref().expect("settings path"));
        assert!(saved.warnings.is_empty(), "{:?}", saved.warnings);
        let session = saved.session.expect("the clean quit wrote the session");
        assert_eq!(session.documents.len(), 4, "every tab is in the session");
        assert_eq!(session.active, first_tab, "the active tab is in the session");
        assert_eq!(
            session.documents[2].path.as_deref(),
            Some(first.to_string_lossy().as_ref()),
            "tab order is the tab order"
        );
        assert_eq!(
            session.documents[3].path.as_deref(),
            Some(second.to_string_lossy().as_ref())
        );
        assert!(
            session.documents[0].path.is_none(),
            "a pathless tab keeps its place with no path"
        );

        let mut relaunched = WorkbenchApp::launch(storage.clone(), None);
        settle(&ctx, &mut relaunched);
        let titles: Vec<&str> = relaunched
            .workbench
            .documents()
            .iter()
            .map(|document| document.title.as_str())
            .collect();
        assert_eq!(
            titles,
            vec!["first.lv", "second.lv"],
            "the file tabs come back in their original order"
        );
        assert_eq!(
            relaunched.workbench.active_document().title,
            "first.lv",
            "the active tab comes back"
        );
        assert!(
            relaunched
                .workbench
                .documents()
                .iter()
                .all(|document| !document.dirty() && document.state() == RunState::Idle),
            "a restored session is clean and has not run anything"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn discarding_at_the_window_close_does_not_recreate_the_record() {
        let (storage, directory) = app_storage("discard-quit");
        let file = directory.join("quit.lv");
        std::fs::write(&file, b"a = 1\n").expect("fixture");
        let mut app = WorkbenchApp::launch(storage.clone(), Some(file.clone()));
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        let index = app.workbench.active();
        app.workbench.set_source(index, "a = 2\n".to_string());
        settle(&ctx, &mut app);
        let record_path = recovery::record_path(
            storage.recovery_root.as_ref().expect("recovery root"),
            app.workbench.documents()[index].id.0,
        );
        assert!(record_path.exists(), "the dirty buffer is journaled");

        let output = close_request(&ctx, &mut app);
        assert!(
            cancelled_close(&output),
            "a dirty window refuses the close and asks first"
        );
        settle(&ctx, &mut app);
        assert!(app.close.is_some());

        let discard_at = prompt_button(&app, UnsavedAnswer::Discard);
        pointer_click(&ctx, &mut app, discard_at);

        assert!(app.window_close_approved, "Discard approves the window close");
        assert!(
            !record_path.exists(),
            "the clean exit deleted the record and the closing frame did not journal it again"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_edit_journals_the_buffer_and_the_save_retires_the_record() {
        let (storage, directory) = app_storage("journal");
        let file = directory.join("dirty.lv");
        std::fs::write(&file, b"a = 1\n").expect("fixture");

        let mut app = WorkbenchApp::launch(storage.clone(), Some(file.clone()));
        let ctx = egui::Context::default();
        settle(&ctx, &mut app);
        let index = app.workbench.active();
        assert_eq!(app.workbench.active_document().path.as_deref(), Some(file.as_path()));
        app.workbench.set_source(index, "a = 2\n".to_string());
        settle(&ctx, &mut app);

        let document_id = app.workbench.documents()[index].id.0;
        let record_path = recovery::record_path(
            storage.recovery_root.as_ref().expect("recovery root"),
            document_id,
        );
        assert!(
            record_path.exists(),
            "the dirty buffer is journaled within the debounce window"
        );
        let record = recovery::read_record(&record_path).expect("the record parses");
        assert_eq!(record.source, "a = 2\n", "the record is the buffer");
        assert_eq!(record.path, Some(file.clone()));
        assert_eq!(record.source.as_bytes(), b"a = 2\n");

        frame(&ctx, &mut app, vec![key(Key::S, ctrl_modifiers())]);
        settle(&ctx, &mut app);
        assert_eq!(std::fs::read(&file).expect("the saved file"), b"a = 2\n");
        assert!(
            !record_path.exists(),
            "a saved document's recovery record is deleted"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }
}
