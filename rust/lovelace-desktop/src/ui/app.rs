//! The native workbench window: toolbar, tab strip, four panes and the status strip.
//!
//! The window is a thin view over [`crate::workbench::Workbench`]. It never evaluates a script:
//! Run only submits to the `ExecutionHost`, every frame polls the typed event stream, and every
//! value in the panes comes from the snapshot or the engine's plot capture. Panes with nothing
//! to show render their empty state instead of sample data.

use std::path::PathBuf;
use std::time::Duration;

use egui::{
    Align, Align2, Color32, CursorIcon, FontId, Id, Key, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

use crate::engine::FunctionOrigin;
use crate::execution::Diagnostic;
use crate::keyboard;
use crate::log_kind::LogKind;
use crate::quick_eval::QuickEval;
use crate::settings::{MAX_PRECISION, MIN_PRECISION};
use crate::workbench::{DocumentView, Workbench};
use crate::workspace_table::{
    FunctionColumn, FunctionTable, VariableColumn, VariableTable, cell_text, ordered_functions,
    ordered_variables,
};

use super::editor::{TextEditor, surface_id};
use super::plot::Plot;
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

/// The window application.
pub struct WorkbenchApp {
    workbench: Workbench,
    /// The one assembled state this frame renders (RUN-003).
    ///
    /// Rebuilt in [`WorkbenchApp::sync`] immediately after `poll()`, so every pane draws the
    /// same committed snapshot, log, revision and diagnostic even though it is a separate egui
    /// closure. No pane calls back into the workbench for pane data.
    view: DocumentView,
    editors: Vec<TextEditor>,
    loaded_revisions: Vec<u64>,
    plot: Plot,
    loaded_plot_svg: Option<String>,
    editor_frac: f32,
    workspace_frac: f32,
    logs_frac: f32,
    filter: String,
    /// The two Workspace tables, rebuilt from the frame's committed snapshot in [`Self::sync`].
    variables_table: VariableTable,
    functions_table: FunctionTable,
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
    pub fn new(initial_path: Option<PathBuf>) -> WorkbenchApp {
        let mut workbench = Workbench::new();
        if let Some(path) = initial_path {
            let _ = workbench.load_file(0, &path);
        }
        WorkbenchApp::with_workbench(workbench)
    }

    /// Creates the window over an existing workbench (also the test seam).
    fn with_workbench(workbench: Workbench) -> WorkbenchApp {
        let editors = workbench
            .documents()
            .iter()
            .map(|document| TextEditor::from_source(&document.source))
            .collect();
        let loaded_revisions = workbench
            .documents()
            .iter()
            .map(|document| document.revision)
            .collect();
        let view = workbench.active_view();
        let variables_table = VariableTable::new(&view.state);
        let functions_table = FunctionTable::new(&view.state);
        WorkbenchApp {
            workbench,
            view,
            editors,
            loaded_revisions,
            plot: Plot::new(),
            loaded_plot_svg: None,
            editor_frac: 0.40,
            workspace_frac: 0.20,
            logs_frac: 0.18,
            filter: String::new(),
            variables_table,
            functions_table,
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

    /// Applies queued events and re-loads the active editor if its document changed elsewhere.
    fn sync(&mut self) {
        self.workbench.poll();
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
        let active = self.workbench.active();
        let revision = self.workbench.documents()[active].revision;
        if self.loaded_revisions[active] != revision {
            let source = self.workbench.documents()[active].source.clone();
            self.editors[active].load(&source);
            self.loaded_revisions[active] = revision;
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

        self.sync();
        if self.trace_pending {
            crate::run_trace::mark("sync_done", "event queue polled and document synced");
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
                .map(|document| {
                    format!(
                        "{}{}",
                        document.title,
                        if document.dirty() { " *" } else { "" }
                    )
                })
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
        let title = match self.workbench.document(active) {
            Some(document) => document.title.clone(),
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
                if ui.button("Fit").clicked() {
                    self.plot.fit();
                }
                ui.separator();
                for zoom in [1.0f32, 1.5, 2.0] {
                    if ui.button(format!("{zoom:.1}x")).clicked() {
                        self.plot.zoom = zoom;
                        self.plot.pan = Vec2::ZERO;
                    }
                }
                ui.separator();
                match &plot_title {
                    Some(title) => {
                        ui.label(RichText::new(title).color(ACCENT));
                    }
                    None => {
                        ui.label(RichText::new("no engine plot").color(DIM));
                    }
                }
            });
            if empty {
                self.plot.ui(ui, body);
                return;
            }
            let ppp = ui.ctx().pixels_per_point();
            self.plot.ensure(ui.ctx(), ppp * self.plot.zoom);
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
                    egui::ScrollArea::vertical()
                        .id_salt("workspace_scroll")
                        .show(ui, |ui| {
                            ui.label(RichText::new(&state_line).small().color(ACCENT));
                            ui.add_space(4.0);

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
                },
            );
        });
        self.filter = filter;
        self.variables_table.sort = variable_sort;
        self.functions_table.sort = function_sort;
        self.variable_header_rects = variable_header_rects;
        self.function_header_rects = function_header_rects;
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
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0))),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.ui_root(ui));
        // A headless pass has no painter to upload the font texture to; mark the deltas handled.
        output.textures_delta.clear();
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
}
