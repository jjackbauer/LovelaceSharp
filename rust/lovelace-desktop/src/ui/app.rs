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

use crate::app::LogLevel;
use crate::keyboard;
use crate::settings::{MAX_PRECISION, MIN_PRECISION};
use crate::workbench::Workbench;

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
    editors: Vec<TextEditor>,
    loaded_revisions: Vec<u64>,
    plot: Plot,
    loaded_plot_svg: Option<String>,
    editor_frac: f32,
    workspace_frac: f32,
    logs_frac: f32,
    filter: String,
    layout: Option<PaneLayout>,
    /// True once the window has moved keyboard focus into the editor on its first frame.
    focus_requested: bool,
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
        WorkbenchApp {
            workbench,
            editors,
            loaded_revisions,
            plot: Plot::new(),
            loaded_plot_svg: None,
            editor_frac: 0.40,
            workspace_frac: 0.20,
            logs_frac: 0.18,
            filter: String::new(),
            layout: None,
            focus_requested: false,
        }
    }

    /// Applies queued events and re-loads the active editor if its document changed elsewhere.
    fn sync(&mut self) {
        self.workbench.poll();
        let active = self.workbench.active();
        let revision = self.workbench.documents()[active].revision;
        if self.loaded_revisions[active] != revision {
            let source = self.workbench.documents()[active].source.clone();
            self.editors[active].load(&source);
            self.loaded_revisions[active] = revision;
        }
        let current_svg = self.workbench.active_plot().map(|plot| plot.svg.clone());
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
        // Start with keyboard focus in the editor so a typed script lands there without a click.
        if !self.focus_requested {
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(surface_id()));
            self.focus_requested = true;
        }

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
            keyboard::command_for_shortcut(
                enter,
                escape,
                enter_command || input.modifiers.command || input.modifiers.ctrl,
            )
        });
        if let Some(command) = shortcut {
            self.workbench.command(command);
        }

        self.sync();

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::top("tabstrip").show(ui, |ui| self.tab_strip(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            self.panes(ui, full);
        });

        if self.workbench.is_running() {
            // Poll again soon so a fast worker's terminal event is applied promptly.
            ui.ctx().request_repaint_after(Duration::from_millis(16));
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

        self.editor_pane(ui, layout.editor);
        self.plot_pane(ui, layout.plot);
        self.workspace_pane(ui, layout.workspace);
        self.logs_pane(ui, layout.logs);

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

    fn editor_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let active = self.workbench.active();
        let title = match self.workbench.document(active) {
            Some(document) => document.title.clone(),
            None => return,
        };
        let lines = self.editors[active].line_count();
        let bytes = self.editors[active].bytes();
        let pane_title = format!("Editor - {title}");
        with_pane(ui, rect, &pane_title, |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                ui.label(RichText::new(&title).color(ACCENT).monospace());
                ui.label(RichText::new(format!("{lines} lines / {bytes} bytes")).color(DIM));
            });
            let before = self.editors[active].edits;
            let metrics = self.editors[active].ui(ui, body);
            if self.editors[active].edits != before {
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

    fn plot_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let active = self.workbench.active();
        let plot_title = self.workbench.documents()[active]
            .last_plot
            .as_ref()
            .map(|plot| plot.title.clone());
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

    fn workspace_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let (variables, functions, has_snapshot) = match self.workbench.active_snapshot() {
            Some(snapshot) => (snapshot.variables.clone(), snapshot.functions.clone(), true),
            None => (Vec::new(), Vec::new(), false),
        };
        let mut filter = self.filter.clone();
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
                    RichText::new(format!(
                        "{} variables · {} functions",
                        variables.len(),
                        functions.len()
                    ))
                    .color(DIM),
                );
            });
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(body.shrink(8.0))
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("workspace_scroll")
                        .show(ui, |ui| {
                            ui.label(RichText::new("VARIABLES").small().color(DIM));
                            let mut shown = 0usize;
                            for (name, value) in &variables {
                                if !filter.is_empty() && !name.contains(filter.as_str()) {
                                    continue;
                                }
                                shown += 1;
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(name).monospace().color(ACCENT));
                                    ui.label(RichText::new("=").monospace().color(DIM));
                                    ui.label(RichText::new(value).monospace().color(TEXT));
                                });
                            }
                            if shown == 0 {
                                let message = if !has_snapshot {
                                    "No run has published a workspace yet."
                                } else if variables.is_empty() {
                                    "The last run defined no variables."
                                } else {
                                    "No variables match the filter."
                                };
                                ui.label(RichText::new(message).color(DIM));
                            }
                            ui.add_space(8.0);
                            ui.separator();
                            ui.label(RichText::new("FUNCTIONS").small().color(DIM));
                            let mut shown = 0usize;
                            for (name, parameters) in &functions {
                                if !filter.is_empty() && !name.contains(filter.as_str()) {
                                    continue;
                                }
                                shown += 1;
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(name).monospace().color(ACCENT));
                                    ui.label(
                                        RichText::new(format!("({})", parameters.join(", ")))
                                            .monospace()
                                            .color(TEXT),
                                    );
                                });
                            }
                            if shown == 0 {
                                let message = if !has_snapshot {
                                    "No run has published a workspace yet."
                                } else if functions.is_empty() {
                                    "The last run defined no functions."
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
    }

    fn logs_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let active = self.workbench.active();
        with_pane(ui, rect, "Logs", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                let (line, column) = self.editors[active].line_col_1based();
                ui.label(
                    RichText::new(format!(
                        "run log · caret Ln {line}, Col {column} · engine {}",
                        self.workbench.engine_name()
                    ))
                    .color(DIM),
                );
            });
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(body.shrink2(vec2(10.0, 4.0)))
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    let logs = self.workbench.active_logs();
                    if logs.is_empty() {
                        ui.label(RichText::new("No log entries yet. Run the document.").color(DIM));
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("logs_scroll")
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in logs {
                                let tag = entry.level.tag();
                                let color = match entry.level {
                                    LogLevel::Error => ERROR,
                                    LogLevel::Warn => WARN,
                                    LogLevel::Info => DIM,
                                    LogLevel::Output => TEXT,
                                    LogLevel::Result => ACCENT,
                                };
                                let text = if tag.is_empty() {
                                    entry.text.clone()
                                } else {
                                    format!("[{tag}] {}", entry.text)
                                };
                                ui.label(RichText::new(text).monospace().color(color));
                            }
                        });
                },
            );
        });
    }
}

impl eframe::App for WorkbenchApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.ui_root(ui);
    }
}

fn header_ui(header: Rect) -> UiBuilder {
    UiBuilder::new()
        .max_rect(header.shrink2(vec2(8.0, 3.0)))
        .layout(Layout::left_to_right(Align::Center))
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
    use crate::engine::Evaluator;
    use crate::workbench::{RunOutcome, Workbench};

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

    #[test]
    fn ctrl_enter_runs_and_escape_cancels_with_the_visible_running_state() {
        let mut app = WorkbenchApp::with_workbench(slow_workbench());
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
            app.workbench.documents()[0].source,
            source_before,
            "Ctrl+Enter must not also insert a newline into the document"
        );
        let toolbar = app.workbench.toolbar_state();
        assert!(!toolbar.run_enabled, "Run must be disabled while running");
        assert!(
            toolbar.cancel_visible,
            "Cancel must be visible while running"
        );
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
        assert!(
            app.workbench.wait_until_idle(Duration::from_secs(2)),
            "the run did not reach a terminal state after Esc"
        );
        assert_eq!(
            app.workbench.documents()[0].outcome,
            Some(RunOutcome::Cancelled)
        );
        assert!(
            app.workbench.status_text(1, 1).contains("run Cancelled"),
            "status strip was {:?}",
            app.workbench.status_text(1, 1)
        );
        let toolbar = app.workbench.toolbar_state();
        assert!(toolbar.run_enabled, "Run must return after the run stops");
        assert!(!toolbar.cancel_visible, "Cancel must leave with the run");
    }
}
